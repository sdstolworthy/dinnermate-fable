use async_trait::async_trait;
use uuid::Uuid;

use crate::error::RepoError;
use crate::model::{List, ListItem, MatchEntry, Participant, Restaurant, Room};

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
}

#[async_trait]
pub trait ListRepo: Send + Sync {
    /// Returns `RepoError::Conflict` if the list code is already taken.
    async fn create(&self, list: &List) -> Result<(), RepoError>;
    async fn find_by_code(&self, code: &str) -> Result<Option<(List, Vec<ListItem>)>, RepoError>;
    async fn add_item(&self, item: &ListItem) -> Result<(), RepoError>;
    async fn lists_for_owner(&self, owner: Uuid) -> Result<Vec<List>, RepoError>;
}
