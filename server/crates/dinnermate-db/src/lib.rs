//! sqlx/Postgres repositories implementing the dinnermate-core traits.

mod error;
mod list_repo;
mod pool;
mod room_repo;

pub use list_repo::PgListRepo;
pub use pool::connect_and_migrate;
pub use room_repo::PgRoomRepo;
