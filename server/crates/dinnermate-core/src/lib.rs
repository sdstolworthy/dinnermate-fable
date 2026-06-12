//! Domain types, services, and the RestaurantProvider trait. No I/O.

pub mod code;
pub mod error;
pub mod filter;
pub mod hours;
pub mod list_deck;
pub mod model;
pub mod osm_hours;
pub mod provider;
pub mod repo;
pub mod seed;
pub mod service;
pub mod testing;

pub use code::generate_code;
pub use error::{CoreError, ProviderError, RepoError};
pub use hours::{open_status, OpenStatus};
pub use list_deck::deck_from_items;
pub use model::{
    HoursPeriod, List, ListItem, ListMembership, MatchEntry, Participant, ProviderDetails,
    Restaurant, Review, Room, RoomParams,
};
pub use osm_hours::parse_osm_opening_hours;
pub use provider::RestaurantProvider;
pub use repo::{DetailsCacheRepo, ListRepo, RoomRepo};
pub use seed::SeedProvider;
pub use service::{CreateRoom, ListService, NewListItem, RoomService};
