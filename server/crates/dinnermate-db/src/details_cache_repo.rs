use async_trait::async_trait;
use chrono::{DateTime, Utc};
use dinnermate_core::{DetailsCacheRepo, ProviderDetails, RepoError};
use sqlx::types::Json;
use sqlx::PgPool;

use crate::error::into_repo_error;

pub struct PgDetailsCacheRepo {
    pool: PgPool,
}

impl PgDetailsCacheRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl DetailsCacheRepo for PgDetailsCacheRepo {
    async fn get(
        &self,
        restaurant_id: &str,
    ) -> Result<Option<(ProviderDetails, DateTime<Utc>)>, RepoError> {
        let row: Option<(Json<ProviderDetails>, DateTime<Utc>)> = sqlx::query_as(
            "SELECT payload, fetched_at FROM restaurant_details_cache WHERE restaurant_id = $1",
        )
        .bind(restaurant_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(into_repo_error)?;
        Ok(row.map(|(payload, fetched_at)| (payload.0, fetched_at)))
    }

    async fn put(&self, restaurant_id: &str, details: &ProviderDetails) -> Result<(), RepoError> {
        sqlx::query(
            "INSERT INTO restaurant_details_cache (restaurant_id, payload, fetched_at) \
             VALUES ($1, $2, now()) \
             ON CONFLICT (restaurant_id) DO UPDATE SET payload = EXCLUDED.payload, \
             fetched_at = now()",
        )
        .bind(restaurant_id)
        .bind(Json(details))
        .execute(&self.pool)
        .await
        .map_err(into_repo_error)?;
        Ok(())
    }
}
