use async_trait::async_trait;
use chrono::{DateTime, Utc};
use dinnermate_core::{
    HoursPeriod, MatchEntry, Participant, RepoError, Restaurant, Room, RoomParams, RoomRepo,
};
use sqlx::types::Json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::into_repo_error;

pub struct PgRoomRepo {
    pool: PgPool,
}

impl PgRoomRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct RoomRow {
    id: Uuid,
    code: String,
    name: Option<String>,
    location_lat: f64,
    location_lng: f64,
    location_label: String,
    radius_m: i32,
    cuisines: Vec<String>,
    price_min: i16,
    price_max: i16,
    min_rating: f32,
    created_by: Uuid,
    created_at: DateTime<Utc>,
}

impl From<RoomRow> for Room {
    fn from(row: RoomRow) -> Self {
        Room {
            id: row.id,
            code: row.code,
            name: row.name,
            params: RoomParams {
                lat: row.location_lat,
                lng: row.location_lng,
                location_label: row.location_label,
                radius_m: row.radius_m as u32,
                cuisines: row.cuisines,
                price_min: row.price_min as u8,
                price_max: row.price_max as u8,
                min_rating: row.min_rating,
            },
            created_by: row.created_by,
            created_at: row.created_at,
            // v3 Task1: column lands with migration 0003 (Task 3); not
            // persisted yet.
            source_list_name: None,
        }
    }
}

// v3 Task1: optional columns mirror the now-optional model fields; the DB
// columns stay NOT NULL until migration 0003 (Task 3).
#[derive(sqlx::FromRow)]
struct RestaurantRow {
    restaurant_id: String,
    name: String,
    cuisine: Option<String>,
    price_level: Option<i16>,
    rating: Option<f32>,
    rating_count: Option<i32>,
    address: String,
    photo_url: Option<String>,
    lat: Option<f64>,
    lng: Option<f64>,
    hours: Option<Json<Vec<HoursPeriod>>>,
    utc_offset_minutes: Option<i32>,
}

impl From<RestaurantRow> for Restaurant {
    fn from(row: RestaurantRow) -> Self {
        Restaurant {
            id: row.restaurant_id,
            name: row.name,
            cuisine: row.cuisine,
            price_level: row.price_level.map(|p| p as u8),
            rating: row.rating,
            rating_count: row.rating_count.map(|c| c as u32),
            address: row.address,
            photo_url: row.photo_url,
            lat: row.lat,
            lng: row.lng,
            hours: row.hours.map(|json| json.0),
            utc_offset_minutes: row.utc_offset_minutes,
        }
    }
}

#[derive(sqlx::FromRow)]
struct ParticipantRow {
    id: Uuid,
    room_id: Uuid,
    user_id: Uuid,
    display_name: String,
    joined_at: DateTime<Utc>,
}

impl From<ParticipantRow> for Participant {
    fn from(row: ParticipantRow) -> Self {
        Participant {
            id: row.id,
            room_id: row.room_id,
            user_id: row.user_id,
            display_name: row.display_name,
            joined_at: row.joined_at,
        }
    }
}

#[derive(sqlx::FromRow)]
struct MatchRow {
    #[sqlx(flatten)]
    restaurant: RestaurantRow,
    like_count: i64,
    last_liked_at: DateTime<Utc>,
}

#[async_trait]
impl RoomRepo for PgRoomRepo {
    async fn create(&self, room: &Room, deck: &[Restaurant]) -> Result<(), RepoError> {
        let mut tx = self.pool.begin().await.map_err(into_repo_error)?;

        sqlx::query(
            "INSERT INTO rooms (id, code, name, location_lat, location_lng, location_label, \
             radius_m, cuisines, price_min, price_max, min_rating, created_by, created_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)",
        )
        .bind(room.id)
        .bind(&room.code)
        .bind(&room.name)
        .bind(room.params.lat)
        .bind(room.params.lng)
        .bind(&room.params.location_label)
        .bind(room.params.radius_m as i32)
        .bind(&room.params.cuisines)
        .bind(room.params.price_min as i16)
        .bind(room.params.price_max as i16)
        .bind(room.params.min_rating)
        .bind(room.created_by)
        .bind(room.created_at)
        .execute(&mut *tx)
        .await
        .map_err(into_repo_error)?;

        for (position, restaurant) in deck.iter().enumerate() {
            sqlx::query(
                "INSERT INTO room_restaurants (room_id, restaurant_id, position, name, cuisine, \
                 price_level, rating, rating_count, address, photo_url, lat, lng, hours, \
                 utc_offset_minutes) \
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)",
            )
            .bind(room.id)
            .bind(&restaurant.id)
            .bind(position as i32)
            .bind(&restaurant.name)
            .bind(&restaurant.cuisine)
            .bind(restaurant.price_level.map(|p| p as i16))
            .bind(restaurant.rating)
            .bind(restaurant.rating_count.map(|c| c as i32))
            .bind(&restaurant.address)
            .bind(&restaurant.photo_url)
            .bind(restaurant.lat)
            .bind(restaurant.lng)
            .bind(restaurant.hours.as_ref().map(Json))
            .bind(restaurant.utc_offset_minutes)
            .execute(&mut *tx)
            .await
            .map_err(into_repo_error)?;
        }

        tx.commit().await.map_err(into_repo_error)
    }

    async fn find_by_code(&self, code: &str) -> Result<Option<(Room, Vec<Restaurant>)>, RepoError> {
        let row: Option<RoomRow> = sqlx::query_as("SELECT * FROM rooms WHERE code = $1")
            .bind(code)
            .fetch_optional(&self.pool)
            .await
            .map_err(into_repo_error)?;
        let Some(row) = row else { return Ok(None) };

        let deck: Vec<RestaurantRow> = sqlx::query_as(
            "SELECT * FROM room_restaurants WHERE room_id = $1 ORDER BY position",
        )
        .bind(row.id)
        .fetch_all(&self.pool)
        .await
        .map_err(into_repo_error)?;

        Ok(Some((row.into(), deck.into_iter().map(Into::into).collect())))
    }

    async fn join(
        &self,
        room_id: Uuid,
        user_id: Uuid,
        display_name: &str,
    ) -> Result<Participant, RepoError> {
        let row: ParticipantRow = sqlx::query_as(
            "INSERT INTO participants (id, room_id, user_id, display_name) \
             VALUES ($1, $2, $3, $4) \
             RETURNING id, room_id, user_id, display_name, joined_at",
        )
        .bind(Uuid::new_v4())
        .bind(room_id)
        .bind(user_id)
        .bind(display_name)
        .fetch_one(&self.pool)
        .await
        .map_err(into_repo_error)?;
        Ok(row.into())
    }

    async fn find_participant(
        &self,
        room_id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<Participant>, RepoError> {
        let row: Option<ParticipantRow> = sqlx::query_as(
            "SELECT * FROM participants WHERE room_id = $1 AND user_id = $2",
        )
        .bind(room_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(into_repo_error)?;
        Ok(row.map(Into::into))
    }

    async fn record_swipe(
        &self,
        room_id: Uuid,
        participant_id: Uuid,
        restaurant_id: &str,
        liked: bool,
    ) -> Result<(), RepoError> {
        sqlx::query(
            "INSERT INTO swipes (room_id, participant_id, restaurant_id, liked) \
             VALUES ($1, $2, $3, $4)",
        )
        .bind(room_id)
        .bind(participant_id)
        .bind(restaurant_id)
        .bind(liked)
        .execute(&self.pool)
        .await
        .map_err(into_repo_error)?;
        Ok(())
    }

    async fn matches(&self, room_id: Uuid) -> Result<Vec<MatchEntry>, RepoError> {
        let rows: Vec<MatchRow> = sqlx::query_as(
            "SELECT rr.*, count(*) AS like_count, max(s.created_at) AS last_liked_at \
             FROM swipes s \
             JOIN room_restaurants rr ON rr.room_id = s.room_id AND rr.restaurant_id = s.restaurant_id \
             WHERE s.room_id = $1 AND s.liked \
             GROUP BY rr.room_id, rr.restaurant_id \
             ORDER BY like_count DESC, last_liked_at DESC",
        )
        .bind(room_id)
        .fetch_all(&self.pool)
        .await
        .map_err(into_repo_error)?;

        Ok(rows
            .into_iter()
            .map(|row| MatchEntry {
                restaurant: row.restaurant.into(),
                like_count: row.like_count,
                last_liked_at: row.last_liked_at,
            })
            .collect())
    }

    async fn participant_count(&self, room_id: Uuid) -> Result<i64, RepoError> {
        sqlx::query_scalar("SELECT count(*) FROM participants WHERE room_id = $1")
            .bind(room_id)
            .fetch_one(&self.pool)
            .await
            .map_err(into_repo_error)
    }

    // v3 Task3 tests this
    async fn delete_older_than(&self, cutoff: DateTime<Utc>) -> Result<u64, RepoError> {
        let result = sqlx::query("DELETE FROM rooms WHERE created_at < $1")
            .bind(cutoff)
            .execute(&self.pool)
            .await
            .map_err(into_repo_error)?;
        Ok(result.rows_affected())
    }

    // v3 Task3 tests this
    async fn participants(&self, room_id: Uuid) -> Result<Vec<Participant>, RepoError> {
        let rows: Vec<ParticipantRow> = sqlx::query_as(
            "SELECT * FROM participants WHERE room_id = $1 ORDER BY joined_at ASC",
        )
        .bind(room_id)
        .fetch_all(&self.pool)
        .await
        .map_err(into_repo_error)?;
        Ok(rows.into_iter().map(Into::into).collect())
    }
}
