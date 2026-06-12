use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use chrono::{DateTime, Utc};
use dinnermate_core::service::CreateRoom;
use dinnermate_core::{MatchEntry, Participant, Restaurant, Room, RoomParams};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::error::ApiError;
use crate::extract::UserId;
use crate::server::AppState;

#[derive(Debug, Deserialize)]
pub struct CreateRoomRequest {
    pub name: Option<String>,
    pub location_label: String,
    pub lat: f64,
    pub lng: f64,
    pub radius_m: u32,
    pub cuisines: Vec<String>,
    pub price_min: u8,
    pub price_max: u8,
    pub min_rating: f32,
}

impl From<CreateRoomRequest> for CreateRoom {
    fn from(req: CreateRoomRequest) -> Self {
        CreateRoom {
            name: req.name,
            params: RoomParams {
                lat: req.lat,
                lng: req.lng,
                location_label: req.location_label,
                radius_m: req.radius_m,
                cuisines: req.cuisines,
                price_min: req.price_min,
                price_max: req.price_max,
                min_rating: req.min_rating,
            },
        }
    }
}

/// Room with its params flattened, mirroring the create-request field names.
#[derive(Debug, Serialize)]
pub struct RoomDto {
    pub id: Uuid,
    pub code: String,
    pub name: Option<String>,
    pub location_label: String,
    pub lat: f64,
    pub lng: f64,
    pub radius_m: u32,
    pub cuisines: Vec<String>,
    pub price_min: u8,
    pub price_max: u8,
    pub min_rating: f32,
    pub created_at: DateTime<Utc>,
}

impl From<Room> for RoomDto {
    fn from(room: Room) -> Self {
        RoomDto {
            id: room.id,
            code: room.code,
            name: room.name,
            location_label: room.params.location_label,
            lat: room.params.lat,
            lng: room.params.lng,
            radius_m: room.params.radius_m,
            cuisines: room.params.cuisines,
            price_min: room.params.price_min,
            price_max: room.params.price_max,
            min_rating: room.params.min_rating,
            created_at: room.created_at,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct RestaurantDto {
    pub id: String,
    pub name: String,
    pub cuisine: String,
    pub price_level: u8,
    pub rating: f32,
    pub rating_count: u32,
    pub address: String,
    pub photo_url: Option<String>,
    pub lat: f64,
    pub lng: f64,
}

impl From<Restaurant> for RestaurantDto {
    fn from(r: Restaurant) -> Self {
        RestaurantDto {
            id: r.id,
            name: r.name,
            cuisine: r.cuisine,
            price_level: r.price_level,
            rating: r.rating,
            rating_count: r.rating_count,
            address: r.address,
            photo_url: r.photo_url,
            lat: r.lat,
            lng: r.lng,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ParticipantDto {
    pub id: Uuid,
    pub room_id: Uuid,
    pub user_id: Uuid,
    pub display_name: String,
}

impl From<Participant> for ParticipantDto {
    fn from(p: Participant) -> Self {
        ParticipantDto {
            id: p.id,
            room_id: p.room_id,
            user_id: p.user_id,
            display_name: p.display_name,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct JoinRequest {
    pub display_name: String,
}

#[derive(Debug, Deserialize)]
pub struct SwipeRequest {
    pub restaurant_id: String,
    pub liked: bool,
}

#[derive(Debug, Serialize)]
pub struct MatchDto {
    pub restaurant: RestaurantDto,
    pub like_count: i64,
    pub last_liked_at: DateTime<Utc>,
}

impl From<MatchEntry> for MatchDto {
    fn from(entry: MatchEntry) -> Self {
        MatchDto {
            restaurant: entry.restaurant.into(),
            like_count: entry.like_count,
            last_liked_at: entry.last_liked_at,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct MatchesResponse {
    pub matches: Vec<MatchDto>,
    pub participant_count: i64,
}

#[derive(Debug, Serialize)]
pub struct CreateRoomResponse {
    pub room: RoomDto,
    pub deck: Vec<RestaurantDto>,
}

#[derive(Debug, Serialize)]
pub struct GetRoomResponse {
    pub room: RoomDto,
    pub deck: Vec<RestaurantDto>,
    pub me: Option<ParticipantDto>,
}

#[derive(Debug, Serialize)]
pub struct JoinResponse {
    pub participant: ParticipantDto,
}

fn to_deck(deck: Vec<Restaurant>) -> Vec<RestaurantDto> {
    deck.into_iter().map(Into::into).collect()
}

pub async fn create(
    State(state): State<AppState>,
    UserId(user): UserId,
    Json(body): Json<CreateRoomRequest>,
) -> Result<(StatusCode, Json<CreateRoomResponse>), ApiError> {
    let (room, deck) = state.rooms.create_room(user, body.into()).await?;
    let response = CreateRoomResponse { room: room.into(), deck: to_deck(deck) };
    Ok((StatusCode::CREATED, Json(response)))
}

pub async fn get(
    State(state): State<AppState>,
    UserId(user): UserId,
    Path(code): Path<String>,
) -> Result<Json<GetRoomResponse>, ApiError> {
    let (room, deck, me) = state.rooms.get_room(&code, user).await?;
    Ok(Json(GetRoomResponse {
        room: room.into(),
        deck: to_deck(deck),
        me: me.map(Into::into),
    }))
}

pub async fn join(
    State(state): State<AppState>,
    UserId(user): UserId,
    Path(code): Path<String>,
    Json(body): Json<JoinRequest>,
) -> Result<Json<JoinResponse>, ApiError> {
    let participant = state.rooms.join(&code, user, &body.display_name).await?;
    Ok(Json(JoinResponse { participant: participant.into() }))
}

pub async fn swipe(
    State(state): State<AppState>,
    UserId(user): UserId,
    Path(code): Path<String>,
    Json(body): Json<SwipeRequest>,
) -> Result<(StatusCode, Json<Value>), ApiError> {
    state.rooms.swipe(&code, user, &body.restaurant_id, body.liked).await?;
    Ok((StatusCode::CREATED, Json(json!({}))))
}

pub async fn matches(
    State(state): State<AppState>,
    UserId(_): UserId,
    Path(code): Path<String>,
) -> Result<Json<MatchesResponse>, ApiError> {
    let (entries, participant_count) = state.rooms.matches(&code).await?;
    Ok(Json(MatchesResponse {
        matches: entries.into_iter().map(Into::into).collect(),
        participant_count,
    }))
}
