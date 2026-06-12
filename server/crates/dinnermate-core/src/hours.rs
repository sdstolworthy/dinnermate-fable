//! Open/closed computation from weekly [`HoursPeriod`] spans.
//!
//! All reasoning happens in the restaurant's local time: `now_utc` is shifted
//! by `utc_offset_minutes` before any comparison. The client mirrors this
//! logic in Dart (`lib/src/hours.dart`); keep the semantics in sync.

use chrono::{DateTime, Datelike, Duration, Timelike, Utc};

use crate::model::HoursPeriod;

#[derive(Debug, Clone, PartialEq)]
pub enum OpenStatus {
    /// Open right now; `until` is the closing time "HH:MM" (local).
    Open { until: String },
    /// Closed; `opens_next` is the next opening time "HH:MM" within the
    /// coming week, if any.
    Closed { opens_next: Option<String> },
    /// Hours or timezone offset are unknown.
    Unknown,
}

/// Spans are half-open `[open, close)`: exactly at `open` is Open, exactly at
/// `close` is Closed. A span whose `close` is not after its `open` crosses
/// midnight and covers `[open, 24:00)` on `day` plus `[00:00, close)` on the
/// following day.
pub fn open_status(
    hours: Option<&[HoursPeriod]>,
    utc_offset_minutes: Option<i32>,
    now_utc: DateTime<Utc>,
) -> OpenStatus {
    let (Some(periods), Some(offset)) = (hours, utc_offset_minutes) else {
        return OpenStatus::Unknown;
    };
    let local = now_utc + Duration::minutes(i64::from(offset));
    let today = local.weekday().num_days_from_sunday() as u8;
    let now_minutes = (local.hour() * 60 + local.minute()) as u16;

    for period in periods {
        let (Some(open), Some(close)) = (parse_hhmm(&period.open), parse_hhmm(&period.close))
        else {
            continue;
        };
        let open_now = if open < close {
            period.day == today && (open..close).contains(&now_minutes)
        } else {
            (period.day == today && now_minutes >= open)
                || ((period.day + 1) % 7 == today && now_minutes < close)
        };
        if open_now {
            return OpenStatus::Open { until: period.close.clone() };
        }
    }

    // 0..=7: day 7 is the same weekday next week, for periods earlier today.
    let opens_next = (0..=7u8)
        .flat_map(|days_ahead| {
            let day = (today + days_ahead) % 7;
            periods.iter().filter_map(move |period| {
                let open = parse_hhmm(&period.open)?;
                (period.day == day && (days_ahead > 0 || open > now_minutes))
                    .then(|| (days_ahead, open, period.open.clone()))
            })
        })
        .min_by_key(|(days_ahead, open, _)| (*days_ahead, *open))
        .map(|(_, _, open)| open);
    OpenStatus::Closed { opens_next }
}

/// Minutes since local midnight, or None for malformed input (such periods
/// are skipped rather than poisoning the whole computation).
fn parse_hhmm(value: &str) -> Option<u16> {
    let (hour, minute) = value.split_once(':')?;
    let hour: u16 = hour.parse().ok()?;
    let minute: u16 = minute.parse().ok()?;
    (hour < 24 && minute < 60).then_some(hour * 60 + minute)
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::*;

    fn period(day: u8, open: &str, close: &str) -> HoursPeriod {
        HoursPeriod { day, open: open.into(), close: close.into() }
    }

    /// 2026-01-04 is a Sunday, so 01-05=Mon(1), 01-06=Tue(2), 01-07=Wed(3),
    /// 01-09=Fri(5), 01-10=Sat(6).
    fn utc(day: u32, hour: u32, minute: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 1, day, hour, minute, 0).unwrap()
    }

    fn open(until: &str) -> OpenStatus {
        OpenStatus::Open { until: until.into() }
    }

    fn closed(opens_next: Option<&str>) -> OpenStatus {
        OpenStatus::Closed { opens_next: opens_next.map(String::from) }
    }

    #[test]
    fn open_status_table() {
        let monday = vec![period(1, "11:00", "22:00")];
        let friday_late = vec![period(5, "17:00", "01:00")];
        let split_monday = vec![period(1, "11:00", "14:00"), period(1, "17:00", "22:00")];
        let mon_tue = vec![period(1, "11:00", "14:00"), period(2, "09:00", "14:00")];

        type Case = (&'static str, Option<Vec<HoursPeriod>>, Option<i32>, DateTime<Utc>, OpenStatus);
        let cases: Vec<Case> = vec![
            (
                "open within span",
                Some(monday.clone()),
                Some(0),
                utc(5, 12, 0),
                open("22:00"),
            ),
            (
                "closed before open, opens later today",
                Some(monday.clone()),
                Some(0),
                utc(5, 9, 0),
                closed(Some("11:00")),
            ),
            (
                "closed day without periods, opens next week",
                Some(monday.clone()),
                Some(0),
                utc(6, 12, 0),
                closed(Some("11:00")),
            ),
            (
                "midnight crossing, open after midnight",
                Some(friday_late.clone()),
                Some(0),
                utc(10, 0, 30),
                open("01:00"),
            ),
            (
                "midnight crossing, open before midnight",
                Some(friday_late.clone()),
                Some(0),
                utc(9, 23, 0),
                open("01:00"),
            ),
            (
                "midnight crossing, closed exactly at close",
                Some(friday_late),
                Some(0),
                utc(10, 1, 0),
                closed(Some("17:00")),
            ),
            (
                "24h span open midday",
                Some(vec![period(3, "00:00", "23:59")]),
                Some(0),
                utc(7, 13, 0),
                open("23:59"),
            ),
            ("unknown when hours none", None, Some(0), utc(5, 12, 0), OpenStatus::Unknown),
            (
                "unknown when offset none",
                Some(monday.clone()),
                None,
                utc(5, 12, 0),
                OpenStatus::Unknown,
            ),
            (
                "boundary exactly at open is open",
                Some(monday.clone()),
                Some(0),
                utc(5, 11, 0),
                open("22:00"),
            ),
            (
                "boundary exactly at close is closed",
                Some(monday.clone()),
                Some(0),
                utc(5, 22, 0),
                closed(Some("11:00")),
            ),
            (
                "opens_next finds later-today period",
                Some(split_monday),
                Some(0),
                utc(5, 15, 0),
                closed(Some("17:00")),
            ),
            (
                "opens_next falls to next-day period",
                Some(mon_tue),
                Some(0),
                utc(5, 15, 0),
                closed(Some("09:00")),
            ),
            (
                "empty periods closed with no opens_next",
                Some(vec![]),
                Some(0),
                utc(5, 12, 0),
                closed(None),
            ),
            (
                "utc offset shifts into previous local day",
                Some(monday),
                Some(-420),
                utc(6, 3, 0), // Tue 03:00 UTC = Mon 20:00 at UTC-7
                open("22:00"),
            ),
        ];

        for (name, hours, offset, now, want) in cases {
            let got = open_status(hours.as_deref(), offset, now);
            assert_eq!(got, want, "{name}");
        }
    }
}
