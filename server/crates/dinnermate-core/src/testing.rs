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
use crate::model::{List, ListItem, MatchEntry, Participant, Restaurant, Room, RoomParams};
use crate::provider::RestaurantProvider;
use crate::repo::{ListRepo, RoomRepo};

/// Restaurant at the default test center (passes `valid_params` radius).
pub fn restaurant(id: &str, name: &str, cuisine: &str, price: u8, rating: f32) -> Restaurant {
    Restaurant {
        id: id.into(),
        name: name.into(),
        cuisine: cuisine.into(),
        price_level: price,
        rating,
        rating_count: 100,
        address: "123 Main St".into(),
        photo_url: None,
        lat: 40.7600,
        lng: -111.8900,
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
    }
}

pub struct FakeProvider(pub Vec<Restaurant>);

#[async_trait]
impl RestaurantProvider for FakeProvider {
    async fn search(&self, _params: &RoomParams) -> Result<Vec<Restaurant>, ProviderError> {
        Ok(self.0.clone())
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
}

#[derive(Default)]
pub struct FakeListRepo {
    lists: Mutex<HashMap<String, (List, Vec<ListItem>)>>,
}

impl FakeListRepo {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl ListRepo for FakeListRepo {
    async fn create(&self, list: &List) -> Result<(), RepoError> {
        let mut lists = self.lists.lock().unwrap();
        if lists.contains_key(&list.code) {
            return Err(RepoError::Conflict);
        }
        lists.insert(list.code.clone(), (list.clone(), Vec::new()));
        Ok(())
    }

    async fn find_by_code(&self, code: &str) -> Result<Option<(List, Vec<ListItem>)>, RepoError> {
        Ok(self.lists.lock().unwrap().get(code).cloned())
    }

    async fn add_item(&self, item: &ListItem) -> Result<(), RepoError> {
        let mut lists = self.lists.lock().unwrap();
        let (_, items) = lists
            .values_mut()
            .find(|(list, _)| list.id == item.list_id)
            .ok_or(RepoError::NotFound)?;
        items.push(item.clone());
        Ok(())
    }

    async fn lists_for_owner(&self, owner: Uuid) -> Result<Vec<List>, RepoError> {
        Ok(self
            .lists
            .lock()
            .unwrap()
            .values()
            .filter(|(list, _)| list.owner_user_id == owner)
            .map(|(list, _)| list.clone())
            .collect())
    }
}
