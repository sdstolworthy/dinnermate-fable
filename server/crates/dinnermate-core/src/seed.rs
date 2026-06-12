//! Embedded seed restaurant dataset and the default [`RestaurantProvider`].
//!
//! The dataset lives in `data/seed_restaurants.json`, is embedded at compile
//! time, and is parsed exactly once on first use.

use std::sync::OnceLock;

use async_trait::async_trait;
use serde::Deserialize;

use crate::error::ProviderError;
use crate::model::{ProviderDetails, Restaurant, RoomParams};
use crate::provider::RestaurantProvider;

const SEED_JSON: &str = include_str!("../data/seed_restaurants.json");

/// One seed JSON object: the restaurant plus details-only extras that are not
/// part of the `Restaurant` snapshot (served via `SeedProvider::details`).
#[derive(Debug, Deserialize)]
struct SeedEntry {
    #[serde(flatten)]
    restaurant: Restaurant,
    #[serde(default)]
    website: Option<String>,
    #[serde(default)]
    phone: Option<String>,
}

static SEED_ENTRIES: OnceLock<Vec<SeedEntry>> = OnceLock::new();
static SEED_RESTAURANTS: OnceLock<Vec<Restaurant>> = OnceLock::new();

fn seed_entries() -> &'static [SeedEntry] {
    SEED_ENTRIES.get_or_init(|| {
        serde_json::from_str(SEED_JSON).expect("embedded seed_restaurants.json must be valid")
    })
}

fn seed_restaurants() -> &'static [Restaurant] {
    SEED_RESTAURANTS
        .get_or_init(|| seed_entries().iter().map(|e| e.restaurant.clone()).collect())
}

/// Provider backed by the embedded seed dataset.
///
/// `search` returns *all* seed restaurants and ignores the search center:
/// the seed data is location-agnostic by design, and `RoomService` applies
/// `filter::apply` (cuisine, price window, min rating, haversine radius)
/// to the provider result afterward.
#[derive(Debug, Default, Clone, Copy)]
pub struct SeedProvider;

impl SeedProvider {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl RestaurantProvider for SeedProvider {
    async fn search(&self, _params: &RoomParams) -> Result<Vec<Restaurant>, ProviderError> {
        Ok(seed_restaurants().to_vec())
    }

    async fn details(&self, restaurant_id: &str) -> Result<ProviderDetails, ProviderError> {
        let entry = seed_entries()
            .iter()
            .find(|e| e.restaurant.id == restaurant_id)
            .ok_or_else(|| {
                ProviderError::InvalidResponse(format!("unknown seed restaurant: {restaurant_id}"))
            })?;
        Ok(ProviderDetails {
            website: entry.website.clone(),
            phone: entry.phone.clone(),
            maps_url: None,
            // Seed data never fabricates review text.
            reviews: Vec::new(),
        })
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use std::sync::Arc;

    use uuid::Uuid;

    use super::*;
    use crate::error::CoreError;
    use crate::service::{CreateRoom, RoomService};
    use crate::testing::{valid_params, FakeRoomRepo};

    const EXPECTED_CUISINES: [&str; 10] = [
        "mexican",
        "thai",
        "italian",
        "japanese",
        "indian",
        "american",
        "chinese",
        "mediterranean",
        "korean",
        "vietnamese",
    ];

    #[test]
    fn dataset_parses_with_exactly_60_entries() {
        assert_eq!(seed_restaurants().len(), 60);
    }

    #[test]
    fn ids_are_unique_and_match_seed_pattern() {
        let mut seen = HashSet::new();
        for r in seed_restaurants() {
            assert!(seen.insert(&r.id), "duplicate id {}", r.id);
            let digits = r.id.strip_prefix("seed-").unwrap_or_else(|| {
                panic!("id {:?} must match ^seed-\\d{{3}}$", r.id);
            });
            assert!(
                digits.len() == 3 && digits.chars().all(|c| c.is_ascii_digit()),
                "id {:?} must match ^seed-\\d{{3}}$",
                r.id
            );
        }
    }

    #[test]
    fn every_entry_is_within_validation_ranges() {
        for r in seed_restaurants() {
            assert!((1..=4).contains(&r.price_level), "{}: price {}", r.id, r.price_level);
            assert!((0.0..=5.0).contains(&r.rating), "{}: rating {}", r.id, r.rating);
            assert!((40.5..=41.0).contains(&r.lat), "{}: lat {}", r.id, r.lat);
            assert!((-112.2..=-111.6).contains(&r.lng), "{}: lng {}", r.id, r.lng);
            assert!(!r.name.trim().is_empty(), "{}: empty name", r.id);
            assert!(!r.cuisine.trim().is_empty(), "{}: empty cuisine", r.id);
            assert!(!r.address.trim().is_empty(), "{}: empty address", r.id);
        }
    }

    #[test]
    fn all_ten_expected_cuisines_are_present() {
        let cuisines: HashSet<&str> =
            seed_restaurants().iter().map(|r| r.cuisine.as_str()).collect();
        for cuisine in EXPECTED_CUISINES {
            assert!(cuisines.contains(cuisine), "missing cuisine {cuisine}");
        }
        assert_eq!(cuisines.len(), 10, "unexpected extra cuisines: {cuisines:?}");
    }

    fn seed_service() -> RoomService {
        RoomService::new(
            Arc::new(FakeRoomRepo::new()),
            Arc::new(SeedProvider::new()),
            Arc::new(crate::testing::FakeDetailsCache::new()),
        )
    }

    fn hhmm_minutes(value: &str) -> u16 {
        let valid = value.len() == 5
            && value.as_bytes()[2] == b':'
            && value[..2].chars().all(|c| c.is_ascii_digit())
            && value[3..].chars().all(|c| c.is_ascii_digit());
        assert!(valid, "time {value:?} must match HH:MM");
        let hour: u16 = value[..2].parse().unwrap();
        let minute: u16 = value[3..].parse().unwrap();
        assert!(hour < 24 && minute < 60, "time {value:?} out of range");
        hour * 60 + minute
    }

    fn is_lunch_only(periods: &[crate::model::HoursPeriod]) -> bool {
        !periods.is_empty() && periods.iter().all(|p| p.open == "11:00" && p.close == "14:30")
    }

    fn crosses_midnight(periods: &[crate::model::HoursPeriod]) -> bool {
        periods.iter().any(|p| hhmm_minutes(&p.close) < hhmm_minutes(&p.open))
    }

    fn is_closed_monday(periods: &[crate::model::HoursPeriod]) -> bool {
        !periods.is_empty() && periods.iter().all(|p| p.day != 1)
    }

    fn is_around_the_clock(periods: &[crate::model::HoursPeriod]) -> bool {
        periods.len() == 7
            && (0..7u8).all(|day| {
                periods.iter().any(|p| p.day == day && p.open == "00:00" && p.close == "23:59")
            })
    }

    #[test]
    fn every_entry_has_valid_hours_or_explicit_null() {
        for r in seed_restaurants() {
            match (&r.hours, r.utc_offset_minutes) {
                (Some(periods), Some(offset)) => {
                    assert!(!periods.is_empty(), "{}: empty hours should be null", r.id);
                    assert_eq!(offset, -360, "{}: utc offset", r.id);
                    for p in periods {
                        assert!(p.day <= 6, "{}: day {} out of range", r.id, p.day);
                        hhmm_minutes(&p.open);
                        hhmm_minutes(&p.close);
                    }
                }
                (None, None) => {}
                (hours, offset) => {
                    panic!("{}: hours/offset must both be set or both null, got {hours:?}/{offset:?}", r.id)
                }
            }
        }
    }

    #[test]
    fn hours_variety_matches_spec() {
        let mut lunch_only = 0;
        let mut midnight_crossing = 0;
        let mut closed_monday = 0;
        let mut around_the_clock = 0;
        let mut unknown = 0;
        for r in seed_restaurants() {
            match &r.hours {
                None => unknown += 1,
                Some(periods) => {
                    if is_lunch_only(periods) {
                        lunch_only += 1;
                    }
                    if crosses_midnight(periods) {
                        midnight_crossing += 1;
                    }
                    if is_closed_monday(periods) {
                        closed_monday += 1;
                    }
                    if is_around_the_clock(periods) {
                        around_the_clock += 1;
                    }
                }
            }
        }
        assert!(lunch_only >= 5, "lunch-only: {lunch_only}");
        assert!(midnight_crossing >= 3, "midnight-crossing: {midnight_crossing}");
        assert!(closed_monday >= 2, "closed-monday: {closed_monday}");
        assert_eq!(around_the_clock, 2, "around-the-clock");
        assert!(unknown >= 3, "null-hours: {unknown}");
    }

    #[tokio::test]
    async fn details_returns_embedded_website_and_phone() {
        let provider = SeedProvider::new();
        let mut with_website = 0;
        for entry in seed_entries() {
            let details = provider.details(&entry.restaurant.id).await.unwrap();
            assert_eq!(details.website, entry.website, "{}", entry.restaurant.id);
            assert_eq!(details.phone, entry.phone, "{}", entry.restaurant.id);
            if details.website.is_some() {
                with_website += 1;
            }
        }
        assert!(
            (25..=35).contains(&with_website),
            "~30 of 60 entries should embed a website, got {with_website}"
        );
    }

    #[tokio::test]
    async fn details_never_fabricates_reviews_or_maps_url() {
        let provider = SeedProvider::new();
        for entry in seed_entries() {
            let details = provider.details(&entry.restaurant.id).await.unwrap();
            assert!(details.reviews.is_empty(), "{}: seed reviews must be empty", entry.restaurant.id);
            assert!(details.maps_url.is_none(), "{}: seed maps_url must be null", entry.restaurant.id);
        }
    }

    #[tokio::test]
    async fn details_for_unknown_id_is_provider_error() {
        let err = SeedProvider::new().details("not-a-seed-id").await.unwrap_err();
        assert!(matches!(err, ProviderError::InvalidResponse(_)), "got {err:?}");
    }

    #[tokio::test]
    async fn create_room_with_thai_filter_yields_all_thai_deck() {
        let params = crate::model::RoomParams {
            cuisines: vec!["thai".into()],
            radius_m: 40_000,
            ..valid_params()
        };
        let (_, deck) = seed_service()
            .create_room(Uuid::new_v4(), CreateRoom { name: None, params })
            .await
            .unwrap();
        assert!(!deck.is_empty());
        assert!(deck.iter().all(|r| r.cuisine == "thai"), "non-thai entry in deck");
    }

    #[tokio::test]
    async fn create_room_with_impossible_rating_is_invalid_params() {
        let params = crate::model::RoomParams { min_rating: 4.95, ..valid_params() };
        let err = seed_service()
            .create_room(Uuid::new_v4(), CreateRoom { name: None, params })
            .await
            .unwrap_err();
        assert!(
            matches!(&err, CoreError::InvalidParams(msg) if msg.contains("no restaurants match")),
            "got {err:?}"
        );
    }
}
