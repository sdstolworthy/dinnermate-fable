use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

pub async fn connect_and_migrate(url: &str) -> Result<PgPool, sqlx::Error> {
    let pool = PgPoolOptions::new().connect(url).await?;
    sqlx::migrate!("../../migrations").run(&pool).await?;
    Ok(pool)
}
