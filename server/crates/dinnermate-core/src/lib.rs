//! Domain types, services, and the RestaurantProvider trait. No I/O.

pub mod code;
pub mod error;
pub mod filter;
pub mod model;
pub mod provider;
pub mod repo;
pub mod service;
pub mod testing;

pub use code::generate_code;
pub use error::{CoreError, ProviderError, RepoError};
pub use model::{List, ListItem, MatchEntry, Participant, Restaurant, Room, RoomParams};
pub use provider::RestaurantProvider;
pub use repo::{ListRepo, RoomRepo};
pub use service::{CreateRoom, ListService, NewListItem, RoomService};
