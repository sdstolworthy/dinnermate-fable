use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use chrono::{DateTime, Utc};
use dinnermate_core::service::NewListItem;
use dinnermate_core::{List, ListItem};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::ApiError;
use crate::extract::UserId;
use crate::server::AppState;

#[derive(Debug, Deserialize)]
pub struct CreateListRequest {
    pub name: String,
}

#[derive(Debug, Serialize)]
pub struct ListDto {
    pub id: Uuid,
    pub code: String,
    pub name: String,
    pub owner_user_id: Uuid,
    pub created_at: DateTime<Utc>,
}

impl From<List> for ListDto {
    fn from(list: List) -> Self {
        ListDto {
            id: list.id,
            code: list.code,
            name: list.name,
            owner_user_id: list.owner_user_id,
            created_at: list.created_at,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ListItemDto {
    pub id: Uuid,
    pub list_id: Uuid,
    pub name: String,
    pub cuisine: Option<String>,
    pub price_level: Option<u8>,
    pub rating: Option<f32>,
    pub address: Option<String>,
    pub photo_url: Option<String>,
    pub added_by_user_id: Uuid,
    pub source_restaurant_id: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl From<ListItem> for ListItemDto {
    fn from(item: ListItem) -> Self {
        ListItemDto {
            id: item.id,
            list_id: item.list_id,
            name: item.name,
            cuisine: item.cuisine,
            price_level: item.price_level,
            rating: item.rating,
            address: item.address,
            photo_url: item.photo_url,
            added_by_user_id: item.added_by_user_id,
            source_restaurant_id: item.source_restaurant_id,
            created_at: item.created_at,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct NewListItemRequest {
    pub name: String,
    pub cuisine: Option<String>,
    pub price_level: Option<u8>,
    pub rating: Option<f32>,
    pub address: Option<String>,
    pub photo_url: Option<String>,
    pub source_restaurant_id: Option<String>,
}

impl From<NewListItemRequest> for NewListItem {
    fn from(req: NewListItemRequest) -> Self {
        NewListItem {
            name: req.name,
            cuisine: req.cuisine,
            price_level: req.price_level,
            rating: req.rating,
            address: req.address,
            photo_url: req.photo_url,
            source_restaurant_id: req.source_restaurant_id,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct CreateListResponse {
    pub list: ListDto,
}

#[derive(Debug, Serialize)]
pub struct MyListsResponse {
    pub lists: Vec<ListDto>,
}

#[derive(Debug, Serialize)]
pub struct ListDetailResponse {
    pub list: ListDto,
    pub items: Vec<ListItemDto>,
}

#[derive(Debug, Serialize)]
pub struct AddItemResponse {
    pub item: ListItemDto,
}

pub async fn create(
    State(state): State<AppState>,
    UserId(user): UserId,
    Json(body): Json<CreateListRequest>,
) -> Result<(StatusCode, Json<CreateListResponse>), ApiError> {
    let list = state.lists.create(user, &body.name).await?;
    Ok((StatusCode::CREATED, Json(CreateListResponse { list: list.into() })))
}

pub async fn mine(
    State(state): State<AppState>,
    UserId(user): UserId,
) -> Result<Json<MyListsResponse>, ApiError> {
    let lists = state.lists.mine(user).await?;
    Ok(Json(MyListsResponse { lists: lists.into_iter().map(Into::into).collect() }))
}

pub async fn get(
    State(state): State<AppState>,
    UserId(_): UserId,
    Path(code): Path<String>,
) -> Result<Json<ListDetailResponse>, ApiError> {
    let (list, items) = state.lists.get(&code).await?;
    Ok(Json(ListDetailResponse {
        list: list.into(),
        items: items.into_iter().map(Into::into).collect(),
    }))
}

pub async fn add_item(
    State(state): State<AppState>,
    UserId(user): UserId,
    Path(code): Path<String>,
    Json(body): Json<NewListItemRequest>,
) -> Result<(StatusCode, Json<AddItemResponse>), ApiError> {
    let item = state.lists.add_item(&code, user, body.into()).await?;
    Ok((StatusCode::CREATED, Json(AddItemResponse { item: item.into() })))
}
