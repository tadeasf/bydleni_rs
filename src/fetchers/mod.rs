pub mod cnb;
pub mod czso;
pub mod fred;
pub mod freshness;
pub mod sreality;

use anyhow::Result;
use sqlx::SqlitePool;

use crate::config::Config;

/// Fetch data from all sources. If `force` is false, skip sources that were fetched recently.
pub async fn fetch_all(pool: &SqlitePool, config: &Config, force: bool) -> Result<()> {
    tracing::info!("Fetching from all sources (force={force})...");

    if let Err(e) = fred::fetch_and_store(pool, config, force).await {
        tracing::error!("FRED fetch failed: {e:#}");
    }
    if let Err(e) = cnb::fetch_and_store(pool, config, force).await {
        tracing::error!("CNB fetch failed: {e:#}");
    }
    if let Err(e) = czso::fetch_and_store(pool, config, force).await {
        tracing::error!("CZSO fetch failed: {e:#}");
    }
    if let Err(e) = sreality::fetch_and_store(pool, config, force).await {
        tracing::error!("Sreality fetch failed: {e:#}");
    }

    tracing::info!("All fetches complete");
    Ok(())
}
