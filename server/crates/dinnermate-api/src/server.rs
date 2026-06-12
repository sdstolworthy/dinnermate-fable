use std::sync::Arc;

use axum::http::HeaderValue;
use axum::routing::{delete, get, post};
use axum::Router;
use dinnermate_core::{ListService, RoomService};
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;

use crate::routes::{health, lists, rooms};

#[derive(Clone)]
pub struct AppState {
    pub rooms: Arc<RoomService>,
    pub lists: Arc<ListService>,
}

/// Builds a `CorsLayer` from the comma-separated `CORS_ALLOWED_ORIGINS`
/// config value; `"*"` yields a fully permissive layer.
pub fn cors_layer(allowed_origins: &str) -> Result<CorsLayer, axum::http::header::InvalidHeaderValue> {
    if allowed_origins.trim() == "*" {
        return Ok(CorsLayer::permissive());
    }
    let origins = allowed_origins
        .split(',')
        .map(|origin| origin.trim().parse::<HeaderValue>())
        .collect::<Result<Vec<_>, _>>()?;
    Ok(CorsLayer::new()
        .allow_origin(origins)
        .allow_methods(Any)
        .allow_headers(Any))
}

pub fn build_router(state: AppState, cors: CorsLayer) -> Router {
    let api = Router::new()
        .route("/rooms", post(rooms::create))
        .route("/rooms/from-list", post(rooms::create_from_list))
        .route("/rooms/{code}", get(rooms::get))
        .route("/rooms/{code}/join", post(rooms::join))
        .route("/rooms/{code}/swipes", post(rooms::swipe))
        .route("/rooms/{code}/matches", get(rooms::matches))
        .route(
            "/rooms/{code}/restaurants/{restaurant_id}/details",
            get(rooms::restaurant_details),
        )
        .route("/lists", post(lists::create).get(lists::mine))
        .route("/lists/{code}", get(lists::get))
        .route("/lists/{code}/items", post(lists::add_item))
        .route("/lists/{code}/join", post(lists::join))
        .route("/lists/{code}/members/me", delete(lists::leave))
        .with_state(state);
    Router::new()
        .route("/healthz", get(health::healthz))
        .nest("/api/v1", api)
        .layer(cors)
        .layer(TraceLayer::new_for_http())
}
