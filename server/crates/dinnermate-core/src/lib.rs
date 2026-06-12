//! Domain types, services, and the RestaurantProvider trait. No I/O.

pub mod code;
pub mod error;
pub mod filter;
pub mod model;
pub mod provider;

pub use code::generate_code;
pub use error::{CoreError, ProviderError, RepoError};
pub use model::{List, ListItem, MatchEntry, Participant, Restaurant, Room, RoomParams};
pub use provider::RestaurantProvider;
