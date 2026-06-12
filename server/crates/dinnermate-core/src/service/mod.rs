pub mod lists;
pub mod rooms;

pub use lists::{ListService, NewListItem};
pub use rooms::{CreateRoom, RoomService};

/// Room/list code collision retry budget (per shared contracts).
pub(crate) const MAX_CODE_ATTEMPTS: usize = 5;
