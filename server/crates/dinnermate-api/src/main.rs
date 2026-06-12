use std::error::Error;
use std::sync::Arc;

use dinnermate_api::config::{Config, RestaurantProviderKind};
use dinnermate_api::google::{GooglePlacesProvider, GOOGLE_PLACES_BASE_URL};
use dinnermate_api::server::{build_router, cors_layer, AppState};
use dinnermate_core::{ListService, RestaurantProvider, RoomService, SeedProvider};
use dinnermate_db::{connect_and_migrate, PgListRepo, PgRoomRepo};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt::init();

    let config = Config::from_env(|key| std::env::var(key).ok())?;
    let pool = connect_and_migrate(&config.database_url).await?;

    let provider: Arc<dyn RestaurantProvider> = match config.provider {
        RestaurantProviderKind::Seed => Arc::new(SeedProvider::new()),
        RestaurantProviderKind::Google => {
            // NOT verified against the live Google Places API (no key was
            // available at build time); covered by stub-server tests only.
            let api_key = config
                .google_places_api_key
                .clone()
                .expect("Config guarantees a key when provider=google");
            Arc::new(GooglePlacesProvider::new(
                reqwest::Client::new(),
                api_key,
                GOOGLE_PLACES_BASE_URL.to_string(),
            ))
        }
    };

    let state = AppState {
        rooms: Arc::new(RoomService::new(Arc::new(PgRoomRepo::new(pool.clone())), provider)),
        lists: Arc::new(ListService::new(Arc::new(PgListRepo::new(pool)))),
    };
    let router = build_router(state, cors_layer(&config.cors_allowed_origins)?);

    let listener = tokio::net::TcpListener::bind(&config.bind_addr).await?;
    tracing::info!("listening on {}", listener.local_addr()?);
    axum::serve(listener, router).await?;
    Ok(())
}
