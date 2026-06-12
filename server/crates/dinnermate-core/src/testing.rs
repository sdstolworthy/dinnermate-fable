//! In-memory fakes and fixture helpers for this crate's tests and for
//! downstream crates' tests. Not intended for production use.
//!
//! The fakes implement the repo traits faithfully where the services depend
//! on the behavior: duplicate room/list codes and duplicate swipes return
//! `RepoError::Conflict`, matching Postgres unique violations. They do not
//! enforce foreign keys — services always resolve rooms/lists before calling.

use std::collections::HashMap;
use std::sync::Mutex;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::error::{ProviderError, RepoError};
use crate::model::{
    List, ListItem, ListMembership, MatchEntry, Participant, ProviderDetails, Restaurant, Room,
    RoomParams,
};
use crate::provider::RestaurantProvider;
use crate::repo::{DetailsCacheRepo, ListRepo, RoomRepo};

/// Restaurant at the default test center (passes `valid_params` radius).
/// Args stay required; they land in the model's `Option` fields as `Some`.
pub fn restaurant(id: &str, name: &str, cuisine: &str, price: u8, rating: f32) -> Restaurant {
    Restaurant {
        id: id.into(),
        name: name.into(),
        cuisine: Some(cuisine.into()),
        price_level: Some(price),
        rating: Some(rating),
        rating_count: Some(100),
        address: "123 Main St".into(),
        photo_url: None,
        lat: Some(40.7600),
        lng: Some(-111.8900),
        hours: None,
        utc_offset_minutes: None,
    }
}

/// Permissive valid params: all cuisines, full price window, no rating cut.
pub fn valid_params() -> RoomParams {
    RoomParams {
        lat: 40.7600,
        lng: -111.8900,
        location_label: "Downtown".into(),
        radius_m: 10_000,
        cuisines: vec![],
        price_min: 1,
        price_max: 4,
        min_rating: 0.0,
        eat_at_utc: None,
    }
}

struct FakeProviderState {
    /// `Err(msg)` is surfaced as `ProviderError::Unavailable(msg)`
    /// (`ProviderError` itself is not `Clone`).
    details_result: Result<ProviderDetails, String>,
    details_calls: u32,
}

pub struct FakeProvider {
    restaurants: Vec<Restaurant>,
    state: Mutex<FakeProviderState>,
}

impl FakeProvider {
    pub fn new(restaurants: Vec<Restaurant>) -> Self {
        Self {
            restaurants,
            state: Mutex::new(FakeProviderState {
                details_result: Ok(ProviderDetails::default()),
                details_calls: 0,
            }),
        }
    }

    /// Next `details` calls return this payload.
    pub fn set_details(&self, details: ProviderDetails) {
        self.state.lock().unwrap().details_result = Ok(details);
    }

    /// Next `details` calls fail with `ProviderError::Unavailable(message)`.
    pub fn fail_details(&self, message: &str) {
        self.state.lock().unwrap().details_result = Err(message.to_string());
    }

    pub fn details_calls(&self) -> u32 {
        self.state.lock().unwrap().details_calls
    }
}

#[async_trait]
impl RestaurantProvider for FakeProvider {
    async fn search(&self, _params: &RoomParams) -> Result<Vec<Restaurant>, ProviderError> {
        Ok(self.restaurants.clone())
    }

    async fn details(&self, _restaurant_id: &str) -> Result<ProviderDetails, ProviderError> {
        let mut state = self.state.lock().unwrap();
        state.details_calls += 1;
        state
            .details_result
            .clone()
            .map_err(ProviderError::Unavailable)
    }
}

#[derive(Default)]
pub struct FakeDetailsCache {
    entries: Mutex<HashMap<String, (ProviderDetails, DateTime<Utc>)>>,
}

impl FakeDetailsCache {
    pub fn new() -> Self {
        Self::default()
    }

    /// Preload an entry with an explicit `fetched_at` (staleness tests).
    pub fn insert_at(&self, restaurant_id: &str, details: ProviderDetails, fetched_at: DateTime<Utc>) {
        self.entries
            .lock()
            .unwrap()
            .insert(restaurant_id.to_string(), (details, fetched_at));
    }

    pub fn stored(&self, restaurant_id: &str) -> Option<(ProviderDetails, DateTime<Utc>)> {
        self.entries.lock().unwrap().get(restaurant_id).cloned()
    }
}

#[async_trait]
impl DetailsCacheRepo for FakeDetailsCache {
    async fn get(
        &self,
        restaurant_id: &str,
    ) -> Result<Option<(ProviderDetails, DateTime<Utc>)>, RepoError> {
        Ok(self.entries.lock().unwrap().get(restaurant_id).cloned())
    }

    async fn put(&self, restaurant_id: &str, details: &ProviderDetails) -> Result<(), RepoError> {
        self.insert_at(restaurant_id, details.clone(), Utc::now());
        Ok(())
    }
}

/// Cache that stores nothing: every `get` misses, every `put` is dropped.
/// For wiring sites that don't exercise details caching.
pub struct NoopDetailsCache;

#[async_trait]
impl DetailsCacheRepo for NoopDetailsCache {
    async fn get(
        &self,
        _restaurant_id: &str,
    ) -> Result<Option<(ProviderDetails, DateTime<Utc>)>, RepoError> {
        Ok(None)
    }

    async fn put(&self, _restaurant_id: &str, _details: &ProviderDetails) -> Result<(), RepoError> {
        Ok(())
    }
}

struct SwipeRecord {
    room_id: Uuid,
    participant_id: Uuid,
    restaurant_id: String,
    liked: bool,
    created_at: DateTime<Utc>,
}

#[derive(Default)]
struct RoomState {
    rooms: HashMap<String, (Room, Vec<Restaurant>)>,
    participants: Vec<Participant>,
    swipes: Vec<SwipeRecord>,
    conflict_creates_remaining: u32,
    attempted_codes: Vec<String>,
}

#[derive(Default)]
pub struct FakeRoomRepo {
    state: Mutex<RoomState>,
}

impl FakeRoomRepo {
    pub fn new() -> Self {
        Self::default()
    }

    /// Forces the next `n` `create` calls to fail with `Conflict`, simulating
    /// code collisions without having to predict randomly generated codes.
    pub fn conflict_next_creates(&self, n: u32) {
        self.state.lock().unwrap().conflict_creates_remaining = n;
    }

    /// Codes passed to `create`, in order, including rejected attempts.
    pub fn attempted_codes(&self) -> Vec<String> {
        self.state.lock().unwrap().attempted_codes.clone()
    }
}

#[async_trait]
impl RoomRepo for FakeRoomRepo {
    async fn create(&self, room: &Room, deck: &[Restaurant]) -> Result<(), RepoError> {
        let mut state = self.state.lock().unwrap();
        state.attempted_codes.push(room.code.clone());
        if state.conflict_creates_remaining > 0 {
            state.conflict_creates_remaining -= 1;
            return Err(RepoError::Conflict);
        }
        if state.rooms.contains_key(&room.code) {
            return Err(RepoError::Conflict);
        }
        state.rooms.insert(room.code.clone(), (room.clone(), deck.to_vec()));
        Ok(())
    }

    async fn find_by_code(&self, code: &str) -> Result<Option<(Room, Vec<Restaurant>)>, RepoError> {
        Ok(self.state.lock().unwrap().rooms.get(code).cloned())
    }

    async fn join(
        &self,
        room_id: Uuid,
        user_id: Uuid,
        display_name: &str,
    ) -> Result<Participant, RepoError> {
        let mut state = self.state.lock().unwrap();
        if state
            .participants
            .iter()
            .any(|p| p.room_id == room_id && p.user_id == user_id)
        {
            return Err(RepoError::Conflict);
        }
        let participant = Participant {
            id: Uuid::new_v4(),
            room_id,
            user_id,
            display_name: display_name.to_string(),
            joined_at: Utc::now(),
        };
        state.participants.push(participant.clone());
        Ok(participant)
    }

    async fn find_participant(
        &self,
        room_id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<Participant>, RepoError> {
        Ok(self
            .state
            .lock()
            .unwrap()
            .participants
            .iter()
            .find(|p| p.room_id == room_id && p.user_id == user_id)
            .cloned())
    }

    async fn record_swipe(
        &self,
        room_id: Uuid,
        participant_id: Uuid,
        restaurant_id: &str,
        liked: bool,
    ) -> Result<(), RepoError> {
        let mut state = self.state.lock().unwrap();
        if state.swipes.iter().any(|s| {
            s.room_id == room_id
                && s.participant_id == participant_id
                && s.restaurant_id == restaurant_id
        }) {
            return Err(RepoError::Conflict);
        }
        state.swipes.push(SwipeRecord {
            room_id,
            participant_id,
            restaurant_id: restaurant_id.to_string(),
            liked,
            created_at: Utc::now(),
        });
        Ok(())
    }

    async fn matches(&self, room_id: Uuid) -> Result<Vec<MatchEntry>, RepoError> {
        let state = self.state.lock().unwrap();
        let deck = state
            .rooms
            .values()
            .find(|(room, _)| room.id == room_id)
            .map(|(_, deck)| deck.clone())
            .unwrap_or_default();
        let mut likes: HashMap<&str, (i64, DateTime<Utc>)> = HashMap::new();
        for swipe in state.swipes.iter().filter(|s| s.room_id == room_id && s.liked) {
            let entry = likes
                .entry(swipe.restaurant_id.as_str())
                .or_insert((0, swipe.created_at));
            entry.0 += 1;
            entry.1 = entry.1.max(swipe.created_at);
        }
        let mut entries: Vec<MatchEntry> = likes
            .into_iter()
            .filter_map(|(restaurant_id, (like_count, last_liked_at))| {
                deck.iter().find(|r| r.id == restaurant_id).map(|r| MatchEntry {
                    restaurant: r.clone(),
                    like_count,
                    last_liked_at,
                })
            })
            .collect();
        entries.sort_by(|a, b| {
            b.like_count
                .cmp(&a.like_count)
                .then(b.last_liked_at.cmp(&a.last_liked_at))
        });
        Ok(entries)
    }

    async fn participant_count(&self, room_id: Uuid) -> Result<i64, RepoError> {
        let count = self
            .state
            .lock()
            .unwrap()
            .participants
            .iter()
            .filter(|p| p.room_id == room_id)
            .count();
        Ok(count as i64)
    }

    async fn delete_older_than(&self, cutoff: DateTime<Utc>) -> Result<u64, RepoError> {
        let mut state = self.state.lock().unwrap();
        let expired: Vec<Uuid> = state
            .rooms
            .values()
            .filter(|(room, _)| room.created_at < cutoff)
            .map(|(room, _)| room.id)
            .collect();
        state.rooms.retain(|_, (room, _)| room.created_at >= cutoff);
        state.participants.retain(|p| !expired.contains(&p.room_id));
        state.swipes.retain(|s| !expired.contains(&s.room_id));
        Ok(expired.len() as u64)
    }

    async fn participants(&self, room_id: Uuid) -> Result<Vec<Participant>, RepoError> {
        let mut participants: Vec<Participant> = self
            .state
            .lock()
            .unwrap()
            .participants
            .iter()
            .filter(|p| p.room_id == room_id)
            .cloned()
            .collect();
        // Stable sort: ties (equal Utc::now() in tests) keep join order.
        participants.sort_by_key(|p| p.joined_at);
        Ok(participants)
    }
}

struct Membership {
    list_id: Uuid,
    user_id: Uuid,
    /// Monotonic join order standing in for `joined_at` — `Utc::now()` can
    /// produce equal timestamps within a test, making "desc" ambiguous.
    joined_seq: u64,
}

#[derive(Default)]
struct ListState {
    lists: HashMap<String, (List, Vec<ListItem>)>,
    members: Vec<Membership>,
    next_seq: u64,
}

impl ListState {
    fn insert_membership(&mut self, list_id: Uuid, user_id: Uuid) {
        if self
            .members
            .iter()
            .any(|m| m.list_id == list_id && m.user_id == user_id)
        {
            return;
        }
        let joined_seq = self.next_seq;
        self.next_seq += 1;
        self.members.push(Membership { list_id, user_id, joined_seq });
    }
}

#[derive(Default)]
pub struct FakeListRepo {
    state: Mutex<ListState>,
}

impl FakeListRepo {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl ListRepo for FakeListRepo {
    async fn create(&self, list: &List) -> Result<(), RepoError> {
        let mut state = self.state.lock().unwrap();
        if state.lists.contains_key(&list.code) {
            return Err(RepoError::Conflict);
        }
        state.lists.insert(list.code.clone(), (list.clone(), Vec::new()));
        state.insert_membership(list.id, list.owner_user_id);
        Ok(())
    }

    async fn find_by_code(&self, code: &str) -> Result<Option<(List, Vec<ListItem>)>, RepoError> {
        Ok(self.state.lock().unwrap().lists.get(code).cloned())
    }

    async fn add_item(&self, item: &ListItem) -> Result<(), RepoError> {
        let mut state = self.state.lock().unwrap();
        let (_, items) = state
            .lists
            .values_mut()
            .find(|(list, _)| list.id == item.list_id)
            .ok_or(RepoError::NotFound)?;
        items.push(item.clone());
        Ok(())
    }

    async fn join(&self, list_id: Uuid, user_id: Uuid) -> Result<(), RepoError> {
        self.state.lock().unwrap().insert_membership(list_id, user_id);
        Ok(())
    }

    async fn leave(&self, list_id: Uuid, user_id: Uuid) -> Result<(), RepoError> {
        self.state
            .lock()
            .unwrap()
            .members
            .retain(|m| !(m.list_id == list_id && m.user_id == user_id));
        Ok(())
    }

    async fn is_member(&self, list_id: Uuid, user_id: Uuid) -> Result<bool, RepoError> {
        Ok(self
            .state
            .lock()
            .unwrap()
            .members
            .iter()
            .any(|m| m.list_id == list_id && m.user_id == user_id))
    }

    async fn lists_for_member(&self, user_id: Uuid) -> Result<Vec<ListMembership>, RepoError> {
        let state = self.state.lock().unwrap();
        let mut memberships: Vec<(u64, ListMembership)> = state
            .members
            .iter()
            .filter(|m| m.user_id == user_id)
            .filter_map(|m| {
                state
                    .lists
                    .values()
                    .find(|(list, _)| list.id == m.list_id)
                    .map(|(list, _)| {
                        let is_owner = list.owner_user_id == user_id;
                        (m.joined_seq, ListMembership { list: list.clone(), is_owner })
                    })
            })
            .collect();
        memberships.sort_by_key(|(seq, _)| std::cmp::Reverse(*seq));
        Ok(memberships.into_iter().map(|(_, m)| m).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    fn room_created_at(created_at: DateTime<Utc>) -> Room {
        Room {
            id: Uuid::new_v4(),
            code: Uuid::new_v4().to_string()[..6].to_uppercase(),
            name: None,
            params: valid_params(),
            created_by: Uuid::new_v4(),
            created_at,
            source_list_name: None,
        }
    }

    #[tokio::test]
    async fn fake_delete_older_than_respects_cutoff() {
        let repo = FakeRoomRepo::new();
        let cutoff = Utc::now() - Duration::days(30);
        let old = room_created_at(cutoff - Duration::seconds(1));
        let fresh = room_created_at(cutoff + Duration::seconds(1));
        repo.create(&old, &[]).await.unwrap();
        repo.create(&fresh, &[]).await.unwrap();

        let deleted = repo.delete_older_than(cutoff).await.unwrap();

        assert_eq!(deleted, 1, "exactly the backdated room is deleted");
        assert!(repo.find_by_code(&old.code).await.unwrap().is_none(), "old room gone");
        assert!(repo.find_by_code(&fresh.code).await.unwrap().is_some(), "fresh room kept");
    }

    #[tokio::test]
    async fn fake_participants_sorted_by_joined_at_asc() {
        let repo = FakeRoomRepo::new();
        let room = room_created_at(Utc::now());
        repo.create(&room, &[]).await.unwrap();
        for name in ["Alice", "Bob", "Cleo"] {
            repo.join(room.id, Uuid::new_v4(), name).await.unwrap();
        }

        let participants = repo.participants(room.id).await.unwrap();

        let names: Vec<&str> = participants.iter().map(|p| p.display_name.as_str()).collect();
        assert_eq!(names, ["Alice", "Bob", "Cleo"]);
    }

    #[tokio::test]
    async fn fake_participants_scoped_to_room() {
        let repo = FakeRoomRepo::new();
        let (room_a, room_b) = (room_created_at(Utc::now()), room_created_at(Utc::now()));
        repo.create(&room_a, &[]).await.unwrap();
        repo.create(&room_b, &[]).await.unwrap();
        repo.join(room_a.id, Uuid::new_v4(), "Alice").await.unwrap();
        repo.join(room_b.id, Uuid::new_v4(), "Bob").await.unwrap();

        let participants = repo.participants(room_a.id).await.unwrap();

        assert_eq!(participants.len(), 1);
        assert_eq!(participants[0].display_name, "Alice");
    }
}
