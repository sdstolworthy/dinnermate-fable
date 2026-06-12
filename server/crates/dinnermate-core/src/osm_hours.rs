//! Parser for a simple subset of the OSM `opening_hours` tag.
//!
//! Pure function, no I/O; the OSM provider (api crate) calls it when mapping
//! Overpass results. Anything outside the supported subset rejects the whole
//! string: a partially understood schedule is worse than an honest "unknown".

use crate::model::HoursPeriod;

/// Parses an OSM `opening_hours` value into weekly spans.
///
/// Supported subset:
/// - `24/7` → all 7 days, 00:00–23:59
/// - `;`-separated rules, each `Days Times`
/// - Days: single (`Mo`), range (`Mo-Fr`, may wrap: `Sa-Mo` = Sa,Su,Mo),
///   or comma list (`Mo,We,Fr`)
/// - Times: comma list of `HH:MM-HH:MM` (close < open crosses midnight)
///
/// Day mapping is OSM `Mo`=1 .. `Sa`=6, `Su`=0 ([`HoursPeriod`] day 0=Sun).
/// ANY unrecognized token (months, `PH`, `off`, `sunrise`, `+`, ...) returns
/// `None` for the whole string. Whitespace-tolerant.
pub fn parse_osm_opening_hours(raw: &str) -> Option<Vec<HoursPeriod>> {
    let raw = raw.trim();
    if raw == "24/7" {
        return Some(
            (0..7u8)
                .map(|day| HoursPeriod { day, open: "00:00".into(), close: "23:59".into() })
                .collect(),
        );
    }
    if raw.is_empty() {
        return None;
    }

    let mut periods = Vec::new();
    for rule in raw.split(';') {
        // A rule is `Days Times`; the times part starts at the first digit.
        let times_start = rule.find(|c: char| c.is_ascii_digit())?;
        let days = parse_days(&rule[..times_start])?;
        let times = parse_times(&rule[times_start..])?;
        for &day in &days {
            for (open, close) in &times {
                periods.push(HoursPeriod { day, open: open.clone(), close: close.clone() });
            }
        }
    }
    Some(periods)
}

/// OSM day order; index here is *not* the `HoursPeriod` day number.
const OSM_DAYS: [&str; 7] = ["Mo", "Tu", "We", "Th", "Fr", "Sa", "Su"];

/// Mo..Sa = 1..6, Su = 0 (HoursPeriod day 0 = Sunday).
fn day_number(osm_index: usize) -> u8 {
    ((osm_index + 1) % 7) as u8
}

fn day_index(token: &str) -> Option<usize> {
    OSM_DAYS.iter().position(|&d| d == token)
}

/// Parses `Mo`, `Mo-Fr` (ranges may wrap past Su), or `Mo,We,Fr`.
fn parse_days(spec: &str) -> Option<Vec<u8>> {
    let spec: String = spec.chars().filter(|c| !c.is_whitespace()).collect();
    if spec.is_empty() {
        return None;
    }
    let mut days = Vec::new();
    for token in spec.split(',') {
        match token.split_once('-') {
            None => days.push(day_number(day_index(token)?)),
            Some((start, end)) => {
                let (start, end) = (day_index(start)?, day_index(end)?);
                let mut index = start;
                loop {
                    days.push(day_number(index));
                    if index == end {
                        break;
                    }
                    index = (index + 1) % 7;
                }
            }
        }
    }
    Some(days)
}

/// Parses a comma list of `HH:MM-HH:MM` spans, normalizing to zero-padded
/// times. Close earlier than open is allowed (crosses midnight).
fn parse_times(spec: &str) -> Option<Vec<(String, String)>> {
    let spec: String = spec.chars().filter(|c| !c.is_whitespace()).collect();
    let mut times = Vec::new();
    for range in spec.split(',') {
        let (open, close) = range.split_once('-')?;
        times.push((parse_hhmm(open)?, parse_hhmm(close)?));
    }
    Some(times)
}

fn parse_hhmm(value: &str) -> Option<String> {
    let (hour, minute) = value.split_once(':')?;
    // Explicit digit check: `u8::parse` would accept `+5`, which is an
    // unsupported token and must reject the whole string.
    if hour.is_empty() || !hour.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    if minute.is_empty() || !minute.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let hour: u8 = hour.parse().ok()?;
    let minute: u8 = minute.parse().ok()?;
    (hour < 24 && minute < 60).then(|| format!("{hour:02}:{minute:02}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn period(day: u8, open: &str, close: &str) -> HoursPeriod {
        HoursPeriod { day, open: open.into(), close: close.into() }
    }

    #[test]
    fn parse_table() {
        let cases: Vec<(&str, &str, Option<Vec<HoursPeriod>>)> = vec![
            (
                "24/7 covers every day",
                "24/7",
                Some((0..7u8).map(|day| period(day, "00:00", "23:59")).collect()),
            ),
            (
                "multi-rule with multi-span times",
                "Mo-Fr 11:00-14:30,17:00-22:00; Sa-Su 12:00-23:00",
                Some(vec![
                    period(1, "11:00", "14:30"),
                    period(1, "17:00", "22:00"),
                    period(2, "11:00", "14:30"),
                    period(2, "17:00", "22:00"),
                    period(3, "11:00", "14:30"),
                    period(3, "17:00", "22:00"),
                    period(4, "11:00", "14:30"),
                    period(4, "17:00", "22:00"),
                    period(5, "11:00", "14:30"),
                    period(5, "17:00", "22:00"),
                    period(6, "12:00", "23:00"),
                    period(0, "12:00", "23:00"),
                ]),
            ),
            (
                "single day",
                "We 09:00-17:00",
                Some(vec![period(3, "09:00", "17:00")]),
            ),
            (
                "comma day list",
                "Mo,We,Fr 10:00-12:00",
                Some(vec![
                    period(1, "10:00", "12:00"),
                    period(3, "10:00", "12:00"),
                    period(5, "10:00", "12:00"),
                ]),
            ),
            (
                "day range wraps through Sunday",
                "Sa-Mo 10:00-12:00",
                Some(vec![
                    period(6, "10:00", "12:00"),
                    period(0, "10:00", "12:00"),
                    period(1, "10:00", "12:00"),
                ]),
            ),
            (
                "close before open crosses midnight",
                "Fr-Sa 17:00-01:00",
                Some(vec![period(5, "17:00", "01:00"), period(6, "17:00", "01:00")]),
            ),
            ("PH off rejects whole string", "Mo-Fr 11:00-22:00; PH off", None),
            ("sunrise-sunset rejected", "sunrise-sunset", None),
            ("month prefix rejects whole string", "Jan-Mar Mo 10:00-12:00", None),
            ("empty string rejected", "", None),
        ];
        for (name, raw, want) in cases {
            let got = parse_osm_opening_hours(raw);
            assert_eq!(got, want, "{name}: input {raw:?}");
        }
    }
}
