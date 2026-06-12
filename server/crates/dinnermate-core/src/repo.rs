use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::error::RepoError;
use crate::model::{
    List, ListItem, ListMembership, MatchEntry, Participant, ProviderDetails, Restaurant, Room,
};

#[async_trait]
pub trait RoomRepo: Send + Sync {
    /// Returns `RepoError::Conflict` if the room code is already taken.
    async fn create(&self, room: &Room, deck: &[Restaurant]) -> Result<(), RepoError>;
    async fn find_by_code(&self, code: &str) -> Result<Option<(Room, Vec<Restaurant>)>, RepoError>;
    async fn join(
        &self,
        room_id: Uuid,
        user_id: Uuid,
        display_name: &str,
    ) -> Result<Participant, RepoError>;
    async fn find_participant(
        &self,
        room_id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<Participant>, RepoError>;
    /// Returns `RepoError::Conflict` on a duplicate (room, participant, restaurant) swipe.
    async fn record_swipe(
        &self,
        room_id: Uuid,
        participant_id: Uuid,
        restaurant_id: &str,
        liked: bool,
    ) -> Result<(), RepoError>;
    async fn matches(&self, room_id: Uuid) -> Result<Vec<MatchEntry>, RepoError>;
    async fn participant_count(&self, room_id: Uuid) -> Result<i64, RepoError>;
    /// Deletes rooms created before `cutoff` (decks/participants/swipes
    /// cascade). Returns the number of rooms deleted.
    async fn delete_older_than(&self, cutoff: DateTime<Utc>) -> Result<u64, RepoError>;
    /// All participants of the room, joined_at asc.
    async fn participants(&self, room_id: Uuid) -> Result<Vec<Participant>, RepoError>;
}

#[async_trait]
pub trait ListRepo: Send + Sync {
    /// Returns `RepoError::Conflict` if the list code is already taken.
    /// Also inserts the owner's membership row (same transaction).
    async fn create(&self, list: &List) -> Result<(), RepoError>;
    async fn find_by_code(&self, code: &str) -> Result<Option<(List, Vec<ListItem>)>, RepoError>;
    async fn add_item(&self, item: &ListItem) -> Result<(), RepoError>;
    /// Idempotent: joining a list you are already a member of is a no-op.
    async fn join(&self, list_id: Uuid, user_id: Uuid) -> Result<(), RepoError>;
    async fn leave(&self, list_id: Uuid, user_id: Uuid) -> Result<(), RepoError>;
    async fn is_member(&self, list_id: Uuid, user_id: Uuid) -> Result<bool, RepoError>;
    /// All lists the user belongs to (owned and joined), joined_at desc.
    async fn lists_for_member(&self, user_id: Uuid) -> Result<Vec<ListMembership>, RepoError>;
}

#[async_trait]
pub trait DetailsCacheRepo: Send + Sync {
    async fn get(
        &self,
        restaurant_id: &str,
    ) -> Result<Option<(ProviderDetails, DateTime<Utc>)>, RepoError>;
    /// Upsert; `fetched_at` is set to now().
    async fn put(&self, restaurant_id: &str, details: &ProviderDetails) -> Result<(), RepoError>;
}
