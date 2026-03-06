use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use sqlx::SqlitePool;
use tokio::sync::RwLock;
use tokio_cron_scheduler::{Job, JobScheduler};

use crate::compute::{affordability, historical};
use crate::config::Config;
use crate::fetchers;

/// Run a one-shot data refresh (fetch all + compute).
pub async fn run_refresh(
    pool: &SqlitePool,
    config: &Config,
    refreshing: &AtomicBool,
    last_refresh: &RwLock<Option<String>>,
) {
    if refreshing.swap(true, Ordering::SeqCst) {
        tracing::info!("Refresh already in progress, skipping");
        return;
    }

    tracing::info!("Starting data refresh...");

    if let Err(e) = fetchers::fetch_all(pool, config, false).await {
        tracing::error!("Fetch failed: {e:#}");
    }

    if let Err(e) = affordability::compute_all(pool).await {
        tracing::error!("Compute failed: {e:#}");
    }

    if let Err(e) = historical::compute_historical_snapshots(pool).await {
        tracing::error!("Historical compute failed: {e:#}");
    }

    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M UTC").to_string();
    *last_refresh.write().await = Some(now);

    refreshing.store(false, Ordering::SeqCst);
    tracing::info!("Data refresh complete");
}

/// Start a periodic scheduler that refreshes data every 6 hours.
pub async fn start_periodic(
    pool: SqlitePool,
    config: Arc<Config>,
    refreshing: Arc<AtomicBool>,
    last_refresh: Arc<RwLock<Option<String>>>,
) -> anyhow::Result<()> {
    let sched = JobScheduler::new().await?;

    let job = Job::new_async("0 0 */6 * * *", move |_uuid, _lock| {
        let pool = pool.clone();
        let config = config.clone();
        let refreshing = refreshing.clone();
        let last_refresh = last_refresh.clone();
        Box::pin(async move {
            run_refresh(&pool, &config, &refreshing, &last_refresh).await;
        })
    })?;

    sched.add(job).await?;
    sched.start().await?;

    // Keep scheduler alive by leaking it (it runs in the background via tokio)
    std::mem::forget(sched);

    tracing::info!("Scheduled data refresh every 6 hours");
    Ok(())
}
