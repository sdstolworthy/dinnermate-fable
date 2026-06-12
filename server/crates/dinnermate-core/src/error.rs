use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProviderError {
    #[error("restaurant provider unavailable: {0}")]
    Unavailable(String),
    #[error("restaurant provider returned an invalid response: {0}")]
    InvalidResponse(String),
}

#[derive(Debug, Error)]
pub enum RepoError {
    #[error("not found")]
    NotFound,
    #[error("conflict")]
    Conflict,
    #[error("database error: {0}")]
    Database(String),
}

#[derive(Debug, Error)]
pub enum CoreError {
    #[error("room not found")]
    RoomNotFound,
    #[error("list not found")]
    ListNotFound,
    #[error("you are not a participant in this room")]
    NotInRoom,
    #[error("you already swiped on this restaurant")]
    AlreadySwiped,
    #[error("unknown restaurant for this room")]
    UnknownRestaurant,
    #[error("you are not a member of this list")]
    NotListMember,
    #[error("the list owner cannot leave the list")]
    OwnerCannotLeave,
    #[error("{0}")]
    InvalidParams(String),
    #[error(transparent)]
    Provider(#[from] ProviderError),
    #[error(transparent)]
    Repo(#[from] RepoError),
}
