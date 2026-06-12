use std::error::Error;
use std::sync::Arc;

use chrono::Utc;
use dinnermate_api::config::{Config, RestaurantProviderKind};
use dinnermate_api::google::{GooglePlacesProvider, GOOGLE_PLACES_BASE_URL};
use dinnermate_api::osm::{OsmProvider, OVERPASS_BASE_URL};
use dinnermate_api::server::{build_router, cors_layer, AppState};
use dinnermate_core::{ListService, RestaurantProvider, RoomRepo, RoomService, SeedProvider};
use dinnermate_db::{connect_and_migrate, PgDetailsCacheRepo, PgListRepo, PgRoomRepo};

const ROOM_TTL_DAYS: i64 = 30;
const EXPIRY_SWEEP_INTERVAL: std::time::Duration = std::time::Duration::from_secs(6 * 60 * 60);

/// Deletes rooms older than the TTL every six hours (first sweep immediately
/// on boot). Deck, participants, and swipes cascade in the database.
fn spawn_room_expiry(repo: Arc<dyn RoomRepo>) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(EXPIRY_SWEEP_INTERVAL);
        loop {
            interval.tick().await;
            match repo.delete_older_than(Utc::now() - chrono::Duration::days(ROOM_TTL_DAYS)).await {
                Ok(deleted) => tracing::info!(deleted, "expired rooms cleaned"),
                Err(err) => tracing::error!(error = %err, "room expiry sweep failed"),
            }
        }
    });
}

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
        RestaurantProviderKind::Osm => Arc::new(OsmProvider::new(
            reqwest::Client::new(),
            config
                .overpass_url
                .clone()
                .unwrap_or_else(|| OVERPASS_BASE_URL.to_string()),
        )),
    };

    let room_repo = Arc::new(PgRoomRepo::new(pool.clone()));
    spawn_room_expiry(room_repo.clone());

    let state = AppState {
        rooms: Arc::new(RoomService::new(
            room_repo,
            provider,
            Arc::new(PgDetailsCacheRepo::new(pool.clone())),
        )),
        lists: Arc::new(ListService::new(Arc::new(PgListRepo::new(pool)))),
    };
    let router = build_router(state, cors_layer(&config.cors_allowed_origins)?);

    let listener = tokio::net::TcpListener::bind(&config.bind_addr).await?;
    tracing::info!("listening on {}", listener.local_addr()?);
    axum::serve(listener, router).await?;
    Ok(())
}
