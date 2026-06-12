use std::sync::Arc;

use chrono::Utc;
use uuid::Uuid;

use crate::code::generate_code;
use crate::error::{CoreError, RepoError};
use crate::filter;
use crate::model::{MatchEntry, Participant, ProviderDetails, Restaurant, Room, RoomParams};
use crate::provider::RestaurantProvider;
use crate::repo::{DetailsCacheRepo, RoomRepo};
use crate::service::MAX_CODE_ATTEMPTS;

pub struct CreateRoom {
    pub name: Option<String>,
    pub params: RoomParams,
}

pub struct RoomService {
    repo: Arc<dyn RoomRepo>,
    provider: Arc<dyn RestaurantProvider>,
    cache: Arc<dyn DetailsCacheRepo>,
}

impl RoomService {
    pub fn new(
        repo: Arc<dyn RoomRepo>,
        provider: Arc<dyn RestaurantProvider>,
        cache: Arc<dyn DetailsCacheRepo>,
    ) -> Self {
        Self { repo, provider, cache }
    }

    pub async fn create_room(
        &self,
        user_id: Uuid,
        input: CreateRoom,
    ) -> Result<(Room, Vec<Restaurant>), CoreError> {
        input.params.validate()?;
        let found = self.provider.search(&input.params).await?;
        let deck = filter::apply(&input.params, found);
        if deck.is_empty() {
            return Err(CoreError::InvalidParams(
                "no restaurants match these filters".to_string(),
            ));
        }
        for _ in 0..MAX_CODE_ATTEMPTS {
            let room = Room {
                id: Uuid::new_v4(),
                code: generate_code(&mut rand::rng()),
                name: input.name.clone(),
                params: input.params.clone(),
                created_by: user_id,
                created_at: Utc::now(),
            };
            match self.repo.create(&room, &deck).await {
                Ok(()) => return Ok((room, deck)),
                Err(RepoError::Conflict) => continue,
                Err(err) => return Err(err.into()),
            }
        }
        Err(CoreError::Repo(RepoError::Conflict))
    }

    pub async fn get_room(
        &self,
        code: &str,
        user_id: Uuid,
    ) -> Result<(Room, Vec<Restaurant>, Option<Participant>), CoreError> {
        let (room, deck) = self.find_room(code).await?;
        let participant = self.repo.find_participant(room.id, user_id).await?;
        Ok((room, deck, participant))
    }

    pub async fn join(
        &self,
        code: &str,
        user_id: Uuid,
        display_name: &str,
    ) -> Result<Participant, CoreError> {
        let display_name = display_name.trim();
        if display_name.is_empty() {
            return Err(CoreError::InvalidParams("display name must not be empty".to_string()));
        }
        let (room, _) = self.find_room(code).await?;
        if let Some(existing) = self.repo.find_participant(room.id, user_id).await? {
            return Ok(existing);
        }
        Ok(self.repo.join(room.id, user_id, display_name).await?)
    }

    pub async fn swipe(
        &self,
        code: &str,
        user_id: Uuid,
        restaurant_id: &str,
        liked: bool,
    ) -> Result<(), CoreError> {
        let (room, deck) = self.find_room(code).await?;
        let participant = self
            .repo
            .find_participant(room.id, user_id)
            .await?
            .ok_or(CoreError::NotInRoom)?;
        if !deck.iter().any(|r| r.id == restaurant_id) {
            return Err(CoreError::UnknownRestaurant);
        }
        self.repo
            .record_swipe(room.id, participant.id, restaurant_id, liked)
            .await
            .map_err(|err| match err {
                RepoError::Conflict => CoreError::AlreadySwiped,
                other => other.into(),
            })
    }

    pub async fn matches(&self, code: &str) -> Result<(Vec<MatchEntry>, i64), CoreError> {
        let (room, _) = self.find_room(code).await?;
        let entries = self.repo.matches(room.id).await?;
        let participant_count = self.repo.participant_count(room.id).await?;
        Ok((entries, participant_count))
    }

    /// Details for a restaurant in the room's deck, with a 24h server cache.
    /// Any authenticated user with the room code may look up details — joining
    /// the room is not required. On provider failure a stale cache entry is
    /// served rather than erroring.
    pub async fn restaurant_details(
        &self,
        code: &str,
        _user_id: Uuid,
        restaurant_id: &str,
    ) -> Result<(Restaurant, ProviderDetails), CoreError> {
        let (_, deck) = self.find_room(code).await?;
        let restaurant = deck
            .into_iter()
            .find(|r| r.id == restaurant_id)
            .ok_or(CoreError::UnknownRestaurant)?;

        let cached = self.cache.get(restaurant_id).await?;
        if let Some((details, fetched_at)) = &cached {
            if Utc::now() - *fetched_at < chrono::Duration::hours(24) {
                return Ok((restaurant, details.clone()));
            }
        }
        match self.provider.details(restaurant_id).await {
            Ok(details) => {
                self.cache.put(restaurant_id, &details).await?;
                Ok((restaurant, details))
            }
            Err(err) => match cached {
                Some((stale, _)) => Ok((restaurant, stale)),
                None => Err(err.into()),
            },
        }
    }

    async fn find_room(&self, code: &str) -> Result<(Room, Vec<Restaurant>), CoreError> {
        self.repo
            .find_by_code(code)
            .await?
            .ok_or(CoreError::RoomNotFound)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::RepoError;
    use crate::testing::{restaurant, valid_params, FakeDetailsCache, FakeProvider, FakeRoomRepo};

    fn service_with(restaurants: Vec<Restaurant>) -> (Arc<FakeRoomRepo>, RoomService) {
        let repo = Arc::new(FakeRoomRepo::new());
        let service = RoomService::new(
            repo.clone(),
            Arc::new(FakeProvider::new(restaurants)),
            Arc::new(FakeDetailsCache::new()),
        );
        (repo, service)
    }

    /// Like `service_with`, but exposes the provider and cache for the
    /// details-caching tests.
    fn details_service(
        restaurants: Vec<Restaurant>,
    ) -> (Arc<FakeProvider>, Arc<FakeDetailsCache>, RoomService) {
        let provider = Arc::new(FakeProvider::new(restaurants));
        let cache = Arc::new(FakeDetailsCache::new());
        let service =
            RoomService::new(Arc::new(FakeRoomRepo::new()), provider.clone(), cache.clone());
        (provider, cache, service)
    }

    fn default_deck() -> Vec<Restaurant> {
        vec![
            restaurant("r1", "Alpha", "thai", 2, 4.5),
            restaurant("r2", "Beta", "mexican", 2, 4.0),
            restaurant("r3", "Gamma", "italian", 3, 3.8),
        ]
    }

    async fn create_default_room(service: &RoomService) -> (Room, Vec<Restaurant>) {
        service
            .create_room(Uuid::new_v4(), CreateRoom { name: None, params: valid_params() })
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn create_room_snapshots_filtered_deck() {
        let (repo, service) = service_with(vec![
            restaurant("r1", "Alpha", "thai", 2, 4.5),
            restaurant("r2", "Beta", "mexican", 2, 4.0),
            restaurant("low", "Low", "thai", 2, 3.0),
        ]);
        let params = RoomParams { min_rating: 4.0, ..valid_params() };
        let (room, deck) = service
            .create_room(Uuid::new_v4(), CreateRoom { name: Some("Dinner".into()), params })
            .await
            .unwrap();
        assert_eq!(room.code.len(), 6);
        let ids: Vec<&str> = deck.iter().map(|r| r.id.as_str()).collect();
        assert_eq!(ids, ["r1", "r2"]);
        let (_, stored) = repo.find_by_code(&room.code).await.unwrap().unwrap();
        assert_eq!(stored, deck, "repo snapshot must equal returned deck");
    }

    #[tokio::test]
    async fn create_room_with_no_matching_restaurants_is_invalid_params() {
        let (_, service) = service_with(vec![restaurant("low", "Low", "thai", 2, 3.0)]);
        let params = RoomParams { min_rating: 4.0, ..valid_params() };
        let err = service
            .create_room(Uuid::new_v4(), CreateRoom { name: None, params })
            .await
            .unwrap_err();
        assert!(
            matches!(&err, CoreError::InvalidParams(msg) if msg == "no restaurants match these filters"),
            "got {err:?}"
        );
    }

    #[tokio::test]
    async fn create_room_retries_on_code_conflict() {
        let (repo, service) = service_with(default_deck());
        repo.conflict_next_creates(1);
        let (room, _) = create_default_room(&service).await;
        let attempted = repo.attempted_codes();
        assert_eq!(attempted.len(), 2, "one rejected attempt plus one success");
        assert_eq!(room.code, attempted[1]);
    }

    #[tokio::test]
    async fn create_room_gives_up_after_five_code_conflicts() {
        let (repo, service) = service_with(default_deck());
        repo.conflict_next_creates(5);
        let err = service
            .create_room(Uuid::new_v4(), CreateRoom { name: None, params: valid_params() })
            .await
            .unwrap_err();
        assert!(matches!(err, CoreError::Repo(RepoError::Conflict)), "got {err:?}");
        assert_eq!(repo.attempted_codes().len(), 5);
    }

    #[tokio::test]
    async fn get_room_unknown_code_is_room_not_found() {
        let (_, service) = service_with(default_deck());
        let err = service.get_room("NOSUCH", Uuid::new_v4()).await.unwrap_err();
        assert!(matches!(err, CoreError::RoomNotFound), "got {err:?}");
    }

    #[tokio::test]
    async fn get_room_returns_participant_after_join() {
        let (_, service) = service_with(default_deck());
        let (room, _) = create_default_room(&service).await;
        let user = Uuid::new_v4();
        let joined = service.join(&room.code, user, "Sam").await.unwrap();
        let (_, _, me) = service.get_room(&room.code, user).await.unwrap();
        assert_eq!(me.map(|p| p.id), Some(joined.id));
    }

    #[tokio::test]
    async fn join_twice_returns_same_participant() {
        let (_, service) = service_with(default_deck());
        let (room, _) = create_default_room(&service).await;
        let user = Uuid::new_v4();
        let first = service.join(&room.code, user, "Sam").await.unwrap();
        let second = service.join(&room.code, user, "Sam Again").await.unwrap();
        assert_eq!(first.id, second.id);
    }

    #[tokio::test]
    async fn join_with_blank_display_name_is_invalid_params() {
        let (_, service) = service_with(default_deck());
        let (room, _) = create_default_room(&service).await;
        for name in ["", "   ", "\t\n"] {
            let err = service.join(&room.code, Uuid::new_v4(), name).await.unwrap_err();
            assert!(matches!(err, CoreError::InvalidParams(_)), "name {name:?}: got {err:?}");
        }
    }

    #[tokio::test]
    async fn swipe_before_join_is_not_in_room() {
        let (_, service) = service_with(default_deck());
        let (room, _) = create_default_room(&service).await;
        let err = service.swipe(&room.code, Uuid::new_v4(), "r1", true).await.unwrap_err();
        assert!(matches!(err, CoreError::NotInRoom), "got {err:?}");
    }

    #[tokio::test]
    async fn swipe_on_restaurant_outside_deck_is_unknown_restaurant() {
        let (_, service) = service_with(default_deck());
        let (room, _) = create_default_room(&service).await;
        let user = Uuid::new_v4();
        service.join(&room.code, user, "Sam").await.unwrap();
        let err = service.swipe(&room.code, user, "not-in-deck", true).await.unwrap_err();
        assert!(matches!(err, CoreError::UnknownRestaurant), "got {err:?}");
    }

    #[tokio::test]
    async fn duplicate_swipe_is_already_swiped() {
        let (_, service) = service_with(default_deck());
        let (room, _) = create_default_room(&service).await;
        let user = Uuid::new_v4();
        service.join(&room.code, user, "Sam").await.unwrap();
        service.swipe(&room.code, user, "r1", true).await.unwrap();
        let err = service.swipe(&room.code, user, "r1", false).await.unwrap_err();
        assert!(matches!(err, CoreError::AlreadySwiped), "got {err:?}");
    }

    fn details_fixture(website: &str) -> ProviderDetails {
        ProviderDetails {
            website: Some(website.into()),
            phone: Some("+1 801-555-0100".into()),
            maps_url: Some("https://maps.example.com/r1".into()),
            reviews: vec![crate::model::Review {
                author: "Pat".into(),
                rating: 5,
                text: "Great noodles".into(),
                relative_time: Some("2 months ago".into()),
            }],
        }
    }

    #[tokio::test]
    async fn restaurant_details_miss_fetches_from_provider_and_caches() {
        let (provider, cache, service) = details_service(default_deck());
        let (room, _) = create_default_room(&service).await;
        let fetched = details_fixture("https://r1.example.com");
        provider.set_details(fetched.clone());

        let (restaurant, details) =
            service.restaurant_details(&room.code, Uuid::new_v4(), "r1").await.unwrap();
        assert_eq!(restaurant.id, "r1");
        assert_eq!(details, fetched);
        assert_eq!(provider.details_calls(), 1);
        let (stored, _) = cache.stored("r1").expect("result must be cached");
        assert_eq!(stored, fetched);
    }

    #[tokio::test]
    async fn restaurant_details_fresh_cache_skips_provider() {
        let (provider, cache, service) = details_service(default_deck());
        let (room, _) = create_default_room(&service).await;
        let cached = details_fixture("https://cached.example.com");
        cache.insert_at("r1", cached.clone(), Utc::now() - chrono::Duration::hours(23));

        let (_, details) =
            service.restaurant_details(&room.code, Uuid::new_v4(), "r1").await.unwrap();
        assert_eq!(details, cached);
        assert_eq!(provider.details_calls(), 0, "fresh cache must not hit the provider");
    }

    #[tokio::test]
    async fn restaurant_details_stale_cache_refetches_and_updates() {
        let (provider, cache, service) = details_service(default_deck());
        let (room, _) = create_default_room(&service).await;
        let stale_at = Utc::now() - chrono::Duration::hours(25);
        cache.insert_at("r1", details_fixture("https://stale.example.com"), stale_at);
        let fresh = details_fixture("https://fresh.example.com");
        provider.set_details(fresh.clone());

        let (_, details) =
            service.restaurant_details(&room.code, Uuid::new_v4(), "r1").await.unwrap();
        assert_eq!(details, fresh);
        assert_eq!(provider.details_calls(), 1);
        let (stored, fetched_at) = cache.stored("r1").unwrap();
        assert_eq!(stored, fresh);
        assert!(fetched_at > stale_at, "cache entry must be refreshed");
    }

    #[tokio::test]
    async fn restaurant_details_provider_error_serves_stale_cache() {
        let (provider, cache, service) = details_service(default_deck());
        let (room, _) = create_default_room(&service).await;
        let stale = details_fixture("https://stale.example.com");
        cache.insert_at("r1", stale.clone(), Utc::now() - chrono::Duration::hours(25));
        provider.fail_details("places down");

        let (_, details) =
            service.restaurant_details(&room.code, Uuid::new_v4(), "r1").await.unwrap();
        assert_eq!(details, stale, "stale cache must be served on provider failure");
    }

    #[tokio::test]
    async fn restaurant_details_provider_error_without_cache_propagates() {
        let (provider, _, service) = details_service(default_deck());
        let (room, _) = create_default_room(&service).await;
        provider.fail_details("places down");

        let err =
            service.restaurant_details(&room.code, Uuid::new_v4(), "r1").await.unwrap_err();
        assert!(matches!(err, CoreError::Provider(_)), "got {err:?}");
    }

    #[tokio::test]
    async fn restaurant_details_outside_deck_is_unknown_restaurant() {
        let (_, _, service) = details_service(default_deck());
        let (room, _) = create_default_room(&service).await;
        let err = service
            .restaurant_details(&room.code, Uuid::new_v4(), "not-in-deck")
            .await
            .unwrap_err();
        assert!(matches!(err, CoreError::UnknownRestaurant), "got {err:?}");
    }

    #[tokio::test]
    async fn restaurant_details_unknown_room_is_room_not_found() {
        let (_, _, service) = details_service(default_deck());
        let err =
            service.restaurant_details("NOSUCH", Uuid::new_v4(), "r1").await.unwrap_err();
        assert!(matches!(err, CoreError::RoomNotFound), "got {err:?}");
    }

    #[tokio::test]
    async fn matches_sorted_by_like_count_with_participant_count() {
        let (_, service) = service_with(default_deck());
        let (room, _) = create_default_room(&service).await;
        let (a, b, c) = (Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4());
        for (user, name) in [(a, "A"), (b, "B"), (c, "C")] {
            service.join(&room.code, user, name).await.unwrap();
        }
        service.swipe(&room.code, a, "r1", true).await.unwrap();
        service.swipe(&room.code, a, "r2", true).await.unwrap();
        service.swipe(&room.code, b, "r1", true).await.unwrap();
        service.swipe(&room.code, c, "r1", true).await.unwrap();
        service.swipe(&room.code, c, "r3", false).await.unwrap();

        let (entries, participant_count) = service.matches(&room.code).await.unwrap();
        let got: Vec<(&str, i64)> =
            entries.iter().map(|e| (e.restaurant.id.as_str(), e.like_count)).collect();
        assert_eq!(got, [("r1", 3), ("r2", 1)]);
        assert_eq!(participant_count, 3);
    }
}
