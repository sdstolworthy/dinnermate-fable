use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use chrono::{DateTime, Utc};
use dinnermate_core::service::CreateRoom;
use dinnermate_core::{
    deck_from_items, CoreError, HoursPeriod, MatchEntry, Participant, Restaurant, Review, Room,
    RoomParams,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::error::ApiError;
use crate::extract::UserId;
use crate::server::AppState;

#[derive(Debug, Deserialize)]
pub struct CreateRoomFromListRequest {
    pub list_code: String,
    pub name: Option<String>,
}

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
    pub eat_at: Option<DateTime<Utc>>,
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
                eat_at_utc: req.eat_at,
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
    pub eat_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub source_list_name: Option<String>,
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
            eat_at: room.params.eat_at_utc,
            created_at: room.created_at,
            source_list_name: room.source_list_name,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct HoursPeriodDto {
    pub day: u8,
    pub open: String,
    pub close: String,
}

impl From<HoursPeriod> for HoursPeriodDto {
    fn from(p: HoursPeriod) -> Self {
        HoursPeriodDto { day: p.day, open: p.open, close: p.close }
    }
}

#[derive(Debug, Serialize)]
pub struct RestaurantDto {
    pub id: String,
    pub name: String,
    pub cuisine: Option<String>,
    pub price_level: Option<u8>,
    pub rating: Option<f32>,
    pub rating_count: Option<u32>,
    pub address: String,
    pub photo_url: Option<String>,
    pub lat: Option<f64>,
    pub lng: Option<f64>,
    pub hours: Option<Vec<HoursPeriodDto>>,
    pub utc_offset_minutes: Option<i32>,
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
            hours: r.hours.map(|periods| periods.into_iter().map(Into::into).collect()),
            utc_offset_minutes: r.utc_offset_minutes,
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

/// Public view of a participant in room responses: display name only, no ids.
#[derive(Debug, Serialize)]
pub struct ParticipantNameDto {
    pub display_name: String,
}

impl From<Participant> for ParticipantNameDto {
    fn from(p: Participant) -> Self {
        ParticipantNameDto { display_name: p.display_name }
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
    pub participants: Vec<ParticipantNameDto>,
}

#[derive(Debug, Serialize)]
pub struct JoinResponse {
    pub participant: ParticipantDto,
}

#[derive(Debug, Serialize)]
pub struct ReviewDto {
    pub author: String,
    pub rating: u8,
    pub text: String,
    pub relative_time: Option<String>,
}

impl From<Review> for ReviewDto {
    fn from(r: Review) -> Self {
        ReviewDto {
            author: r.author,
            rating: r.rating,
            text: r.text,
            relative_time: r.relative_time,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct RestaurantDetailsResponse {
    pub restaurant: RestaurantDto,
    pub website: Option<String>,
    pub phone: Option<String>,
    pub maps_url: Option<String>,
    pub reviews: Vec<ReviewDto>,
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

/// POST /rooms/from-list: a room whose deck is a curated list snapshot.
/// Member-only — the list code is shared more widely than its membership.
pub async fn create_from_list(
    State(state): State<AppState>,
    UserId(user): UserId,
    Json(body): Json<CreateRoomFromListRequest>,
) -> Result<(StatusCode, Json<CreateRoomResponse>), ApiError> {
    let (list, items, is_member, _) = state.lists.get(&body.list_code, user).await?;
    if !is_member {
        return Err(CoreError::NotListMember.into());
    }
    let deck = deck_from_items(&items);
    let (room, deck) = state.rooms.create_with_deck(user, body.name, &list.name, deck).await?;
    let response = CreateRoomResponse { room: room.into(), deck: to_deck(deck) };
    Ok((StatusCode::CREATED, Json(response)))
}

pub async fn get(
    State(state): State<AppState>,
    UserId(user): UserId,
    Path(code): Path<String>,
) -> Result<Json<GetRoomResponse>, ApiError> {
    let (room, deck, me) = state.rooms.get_room(&code, user).await?;
    let participants = state.rooms.participants(&code).await?;
    Ok(Json(GetRoomResponse {
        room: room.into(),
        deck: to_deck(deck),
        me: me.map(Into::into),
        participants: participants.into_iter().map(Into::into).collect(),
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

pub async fn restaurant_details(
    State(state): State<AppState>,
    UserId(user): UserId,
    Path((code, restaurant_id)): Path<(String, String)>,
) -> Result<Json<RestaurantDetailsResponse>, ApiError> {
    let (restaurant, details) = state
        .rooms
        .restaurant_details(&code, user, &restaurant_id)
        .await
        // Route-local mapping: here an unknown restaurant is a failed resource
        // lookup (404); the swipe route keeps the generic 422 for the same error.
        .map_err(|err| match err {
            CoreError::UnknownRestaurant => ApiError::RestaurantNotFound,
            other => other.into(),
        })?;
    Ok(Json(RestaurantDetailsResponse {
        restaurant: restaurant.into(),
        website: details.website,
        phone: details.phone,
        maps_url: details.maps_url,
        reviews: details.reviews.into_iter().map(Into::into).collect(),
    }))
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
