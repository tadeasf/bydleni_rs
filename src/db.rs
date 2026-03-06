use std::str::FromStr;

use anyhow::Result;
use sqlx::SqlitePool;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous};

/// Initialize the SQLite connection pool and run migrations.
pub async fn init_pool(database_url: &str) -> Result<SqlitePool> {
    let options = SqliteConnectOptions::from_str(database_url)?
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal)
        .synchronous(SqliteSynchronous::Normal)
        .pragma("cache_size", "-64000"); // 64MB cache

    let pool = SqlitePoolOptions::new().max_connections(5).connect_with(options).await?;

    sqlx::migrate!("./migrations").run(&pool).await?;

    tracing::info!("Database initialized with WAL mode, migrations applied");
    Ok(pool)
}
