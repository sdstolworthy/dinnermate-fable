//! Embedded seed restaurant dataset and the default [`RestaurantProvider`].
//!
//! The dataset lives in `data/seed_restaurants.json`, is embedded at compile
//! time, and is parsed exactly once on first use.

use std::sync::OnceLock;

use async_trait::async_trait;

use crate::error::ProviderError;
use crate::model::{Restaurant, RoomParams};
use crate::provider::RestaurantProvider;

const SEED_JSON: &str = include_str!("../data/seed_restaurants.json");

static SEED_RESTAURANTS: OnceLock<Vec<Restaurant>> = OnceLock::new();

fn seed_restaurants() -> &'static [Restaurant] {
    SEED_RESTAURANTS.get_or_init(|| {
        serde_json::from_str(SEED_JSON).expect("embedded seed_restaurants.json must be valid")
    })
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
        RoomService::new(Arc::new(FakeRoomRepo::new()), Arc::new(SeedProvider::new()))
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
