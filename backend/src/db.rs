use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;
use std::time::Duration;

#[derive(Debug)]
pub enum DbError {
    MissingConfiguration,
    ConnectionFailed,
    MigrationFailed,
}

impl std::fmt::Display for DbError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DbError::MissingConfiguration => write!(f, "database configuration is missing"),
            DbError::ConnectionFailed => write!(f, "database connection failed"),
            DbError::MigrationFailed => write!(f, "database migration failed"),
        }
    }
}

impl std::error::Error for DbError {}

pub async fn init_pool(database_url: Option<&str>) -> Result<Option<PgPool>, DbError> {
    let Some(database_url) = database_url.filter(|value| !value.trim().is_empty()) else {
        return Ok(None);
    };

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .acquire_timeout(Duration::from_secs(5))
        .connect(database_url)
        .await
        .map_err(|_| DbError::ConnectionFailed)?;

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .map_err(|_| DbError::MigrationFailed)?;

    Ok(Some(pool))
}
