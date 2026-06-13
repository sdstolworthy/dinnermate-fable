//! Coordinate → UTC offset resolution for providers that have no timezone
//! data of their own (OSM). Uses tzf-rs's embedded polygon data for the
//! lat/lng → IANA name step and chrono-tz for the name → offset step.

use std::str::FromStr;
use std::sync::OnceLock;

use chrono::{DateTime, Offset, Utc};
use chrono_tz::Tz;
use tzf_rs::DefaultFinder;

/// Built once per process; construction deserializes the embedded polygon
/// data (~100ms), so it must not happen per lookup.
static FINDER: OnceLock<DefaultFinder> = OnceLock::new();

/// UTC offset in minutes of the timezone containing (lat, lng), evaluated at
/// `at` (so DST is accounted for). `None` when the point matches no timezone
/// or the resolved name fails to parse.
pub fn utc_offset_minutes(lat: f64, lng: f64, at: DateTime<Utc>) -> Option<i32> {
    let finder = FINDER.get_or_init(DefaultFinder::new);
    // tzf-rs takes longitude first.
    let name = finder.get_tz_name(lng, lat);
    if name.is_empty() {
        return None;
    }
    let tz = Tz::from_str(name).ok()?;
    Some(at.with_timezone(&tz).offset().fix().local_minus_utc() / 60)
}

#[cfg(test)]
mod tests {
    use chrono::{DateTime, Utc};

    use super::utc_offset_minutes;

    fn instant(rfc3339: &str) -> DateTime<Utc> {
        rfc3339.parse().expect("test instant must parse")
    }

    #[test]
    fn offset_table() {
        // (name, lat, lng, at, expected)
        let cases = [
            ("SLC in June is MDT", 40.76, -111.89, "2026-06-15T12:00:00Z", Some(-360)),
            ("SLC in January is MST", 40.76, -111.89, "2026-01-15T12:00:00Z", Some(-420)),
            ("London in June is BST", 51.5074, -0.1278, "2026-06-15T12:00:00Z", Some(60)),
        ];
        for (name, lat, lng, at, expected) in cases {
            assert_eq!(utc_offset_minutes(lat, lng, instant(at)), expected, "{name}");
        }
    }

    #[test]
    fn null_island_returns_without_panicking() {
        let offset = utc_offset_minutes(0.0, 0.0, instant("2026-06-15T12:00:00Z"));
        // Mid-ocean may resolve to an Etc/* zone or nothing; either is fine,
        // but a resolved offset must at least be a real-world one.
        assert!(
            offset.is_none() || (-720..=840).contains(&offset.unwrap()),
            "implausible offset {offset:?}"
        );
    }
}
