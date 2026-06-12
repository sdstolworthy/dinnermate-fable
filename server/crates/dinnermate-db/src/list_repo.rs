use async_trait::async_trait;
use chrono::{DateTime, Utc};
use dinnermate_core::{List, ListItem, ListMembership, ListRepo, RepoError};
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::into_repo_error;

pub struct PgListRepo {
    pool: PgPool,
}

impl PgListRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct ListRow {
    id: Uuid,
    code: String,
    name: String,
    owner_user_id: Uuid,
    created_at: DateTime<Utc>,
}

impl From<ListRow> for List {
    fn from(row: ListRow) -> Self {
        List {
            id: row.id,
            code: row.code,
            name: row.name,
            owner_user_id: row.owner_user_id,
            created_at: row.created_at,
        }
    }
}

#[derive(sqlx::FromRow)]
struct MembershipRow {
    #[sqlx(flatten)]
    list: ListRow,
    is_owner: bool,
}

#[derive(sqlx::FromRow)]
struct ListItemRow {
    id: Uuid,
    list_id: Uuid,
    name: String,
    cuisine: Option<String>,
    price_level: Option<i16>,
    rating: Option<f32>,
    address: Option<String>,
    photo_url: Option<String>,
    added_by_user_id: Uuid,
    source_restaurant_id: Option<String>,
    created_at: DateTime<Utc>,
}

impl From<ListItemRow> for ListItem {
    fn from(row: ListItemRow) -> Self {
        ListItem {
            id: row.id,
            list_id: row.list_id,
            name: row.name,
            cuisine: row.cuisine,
            price_level: row.price_level.map(|p| p as u8),
            rating: row.rating,
            address: row.address,
            photo_url: row.photo_url,
            added_by_user_id: row.added_by_user_id,
            source_restaurant_id: row.source_restaurant_id,
            created_at: row.created_at,
        }
    }
}

#[async_trait]
impl ListRepo for PgListRepo {
    async fn create(&self, list: &List) -> Result<(), RepoError> {
        let mut tx = self.pool.begin().await.map_err(into_repo_error)?;

        sqlx::query(
            "INSERT INTO lists (id, code, name, owner_user_id, created_at) \
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(list.id)
        .bind(&list.code)
        .bind(&list.name)
        .bind(list.owner_user_id)
        .bind(list.created_at)
        .execute(&mut *tx)
        .await
        .map_err(into_repo_error)?;

        // Owner joins at creation time so lists_for_member ordering matches
        // the migration 0002 backfill semantics (joined_at = created_at).
        sqlx::query(
            "INSERT INTO list_members (list_id, user_id, joined_at) VALUES ($1, $2, $3)",
        )
        .bind(list.id)
        .bind(list.owner_user_id)
        .bind(list.created_at)
        .execute(&mut *tx)
        .await
        .map_err(into_repo_error)?;

        tx.commit().await.map_err(into_repo_error)
    }

    async fn find_by_code(&self, code: &str) -> Result<Option<(List, Vec<ListItem>)>, RepoError> {
        let row: Option<ListRow> = sqlx::query_as("SELECT * FROM lists WHERE code = $1")
            .bind(code)
            .fetch_optional(&self.pool)
            .await
            .map_err(into_repo_error)?;
        let Some(row) = row else { return Ok(None) };

        let items: Vec<ListItemRow> = sqlx::query_as(
            "SELECT * FROM list_items WHERE list_id = $1 ORDER BY created_at ASC",
        )
        .bind(row.id)
        .fetch_all(&self.pool)
        .await
        .map_err(into_repo_error)?;

        Ok(Some((row.into(), items.into_iter().map(Into::into).collect())))
    }

    async fn add_item(&self, item: &ListItem) -> Result<(), RepoError> {
        sqlx::query(
            "INSERT INTO list_items (id, list_id, name, cuisine, price_level, rating, address, \
             photo_url, added_by_user_id, source_restaurant_id, created_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)",
        )
        .bind(item.id)
        .bind(item.list_id)
        .bind(&item.name)
        .bind(&item.cuisine)
        .bind(item.price_level.map(|p| p as i16))
        .bind(item.rating)
        .bind(&item.address)
        .bind(&item.photo_url)
        .bind(item.added_by_user_id)
        .bind(&item.source_restaurant_id)
        .bind(item.created_at)
        .execute(&self.pool)
        .await
        .map_err(into_repo_error)?;
        Ok(())
    }

    async fn join(&self, list_id: Uuid, user_id: Uuid) -> Result<(), RepoError> {
        sqlx::query(
            "INSERT INTO list_members (list_id, user_id) VALUES ($1, $2) \
             ON CONFLICT DO NOTHING",
        )
        .bind(list_id)
        .bind(user_id)
        .execute(&self.pool)
        .await
        .map_err(into_repo_error)?;
        Ok(())
    }

    async fn leave(&self, list_id: Uuid, user_id: Uuid) -> Result<(), RepoError> {
        sqlx::query("DELETE FROM list_members WHERE list_id = $1 AND user_id = $2")
            .bind(list_id)
            .bind(user_id)
            .execute(&self.pool)
            .await
            .map_err(into_repo_error)?;
        Ok(())
    }

    async fn is_member(&self, list_id: Uuid, user_id: Uuid) -> Result<bool, RepoError> {
        sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM list_members WHERE list_id = $1 AND user_id = $2)",
        )
        .bind(list_id)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await
        .map_err(into_repo_error)
    }

    async fn lists_for_member(&self, user_id: Uuid) -> Result<Vec<ListMembership>, RepoError> {
        let rows: Vec<MembershipRow> = sqlx::query_as(
            "SELECT l.*, (l.owner_user_id = lm.user_id) AS is_owner \
             FROM lists l \
             JOIN list_members lm ON lm.list_id = l.id \
             WHERE lm.user_id = $1 \
             ORDER BY lm.joined_at DESC",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
        .map_err(into_repo_error)?;
        Ok(rows
            .into_iter()
            .map(|row| ListMembership { list: row.list.into(), is_owner: row.is_owner })
            .collect())
    }
}
