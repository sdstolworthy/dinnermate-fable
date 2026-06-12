use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::CoreError;

/// Weekly opening span in the restaurant's local time.
/// `day`: 0 = Sunday .. 6 = Saturday; `open`/`close` are "HH:MM".
/// `close` earlier than `open` means the span crosses midnight into the next day.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HoursPeriod {
    pub day: u8,
    pub open: String,
    pub close: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Restaurant {
    /// Provider-scoped id, e.g. "seed-001".
    pub id: String,
    pub name: String,
    /// Lowercase single tag, e.g. "thai". None = unknown (e.g. OSM entries
    /// without a cuisine tag, free-form list items).
    pub cuisine: Option<String>,
    pub price_level: Option<u8>,
    /// None = unrated, never 0.0.
    pub rating: Option<f32>,
    pub rating_count: Option<u32>,
    /// May be empty when the provider has no address.
    pub address: String,
    pub photo_url: Option<String>,
    pub lat: Option<f64>,
    pub lng: Option<f64>,
    /// None = hours unknown (e.g. pre-v2 rows, providers without hours data).
    #[serde(default)]
    pub hours: Option<Vec<HoursPeriod>>,
    #[serde(default)]
    pub utc_offset_minutes: Option<i32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RoomParams {
    pub lat: f64,
    pub lng: f64,
    pub location_label: String,
    pub radius_m: u32,
    /// Empty = all cuisines.
    pub cuisines: Vec<String>,
    pub price_min: u8,
    pub price_max: u8,
    pub min_rating: f32,
}

impl RoomParams {
    pub fn validate(&self) -> Result<(), CoreError> {
        let invalid = |msg: &str| Err(CoreError::InvalidParams(msg.to_string()));
        if !(-90.0..=90.0).contains(&self.lat) {
            return invalid("lat must be between -90 and 90");
        }
        if !(-180.0..=180.0).contains(&self.lng) {
            return invalid("lng must be between -180 and 180");
        }
        if !(100..=40_000).contains(&self.radius_m) {
            return invalid("radius_m must be between 100 and 40000 meters");
        }
        if !(1..=4).contains(&self.price_min) || !(1..=4).contains(&self.price_max) {
            return invalid("price levels must be between 1 and 4");
        }
        if self.price_min > self.price_max {
            return invalid("price_min must not exceed price_max");
        }
        if !(0.0..=5.0).contains(&self.min_rating) {
            return invalid("min_rating must be between 0 and 5");
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Room {
    pub id: Uuid,
    pub code: String,
    pub name: Option<String>,
    pub params: RoomParams,
    pub created_by: Uuid,
    pub created_at: DateTime<Utc>,
    /// Set when the room was created from a curated list (display only).
    #[serde(default)]
    pub source_list_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Participant {
    pub id: Uuid,
    pub room_id: Uuid,
    pub user_id: Uuid,
    pub display_name: String,
    pub joined_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MatchEntry {
    pub restaurant: Restaurant,
    pub like_count: i64,
    pub last_liked_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct List {
    pub id: Uuid,
    pub code: String,
    pub name: String,
    pub owner_user_id: Uuid,
    pub created_at: DateTime<Utc>,
}

/// A list as seen by one of its members (owner rows included).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ListMembership {
    pub list: List,
    pub is_owner: bool,
}

/// On-demand restaurant details fetched from a provider (cached server-side).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ProviderDetails {
    pub website: Option<String>,
    pub phone: Option<String>,
    pub maps_url: Option<String>,
    pub reviews: Vec<Review>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Review {
    pub author: String,
    pub rating: u8,
    pub text: String,
    pub relative_time: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ListItem {
    pub id: Uuid,
    pub list_id: Uuid,
    pub name: String,
    pub cuisine: Option<String>,
    pub price_level: Option<u8>,
    pub rating: Option<f32>,
    pub address: Option<String>,
    pub photo_url: Option<String>,
    pub added_by_user_id: Uuid,
    pub source_restaurant_id: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_params() -> RoomParams {
        RoomParams {
            lat: 40.76,
            lng: -111.89,
            location_label: "Downtown".into(),
            radius_m: 5_000,
            cuisines: vec!["thai".into()],
            price_min: 1,
            price_max: 4,
            min_rating: 3.5,
        }
    }

    #[test]
    fn validate_table() {
        let cases: Vec<(&str, RoomParams, Option<&str>)> = vec![
            ("valid full params", valid_params(), None),
            (
                "valid minimal bounds",
                RoomParams {
                    radius_m: 100,
                    cuisines: vec![],
                    min_rating: 0.0,
                    ..valid_params()
                },
                None,
            ),
            (
                "radius below 100",
                RoomParams { radius_m: 99, ..valid_params() },
                Some("radius_m"),
            ),
            (
                "radius above 40000",
                RoomParams { radius_m: 40_001, ..valid_params() },
                Some("radius_m"),
            ),
            (
                "price_min above price_max",
                RoomParams { price_min: 3, price_max: 2, ..valid_params() },
                Some("must not exceed"),
            ),
            (
                "price_min below 1",
                RoomParams { price_min: 0, ..valid_params() },
                Some("between 1 and 4"),
            ),
            (
                "price_max above 4",
                RoomParams { price_max: 5, ..valid_params() },
                Some("between 1 and 4"),
            ),
            (
                "min_rating below 0",
                RoomParams { min_rating: -0.1, ..valid_params() },
                Some("min_rating"),
            ),
            (
                "min_rating above 5",
                RoomParams { min_rating: 5.1, ..valid_params() },
                Some("min_rating"),
            ),
            ("lat above 90", RoomParams { lat: 90.1, ..valid_params() }, Some("lat")),
            ("lat below -90", RoomParams { lat: -90.1, ..valid_params() }, Some("lat")),
            ("lng above 180", RoomParams { lng: 180.1, ..valid_params() }, Some("lng")),
            ("lng below -180", RoomParams { lng: -180.1, ..valid_params() }, Some("lng")),
        ];
        for (name, params, want) in cases {
            let got = params.validate();
            match want {
                None => assert!(got.is_ok(), "{name}: expected ok, got {got:?}"),
                Some(substring) => {
                    let msg = got.expect_err(&format!("{name}: expected error")).to_string();
                    assert!(
                        msg.contains(substring),
                        "{name}: message {msg:?} should contain {substring:?}"
                    );
                }
            }
        }
    }
}
