use anyhow::Result;
use sqlx::SqlitePool;

/// Check whether data for a given source was successfully fetched within the last `max_age_hours`.
pub async fn is_fresh(pool: &SqlitePool, source: &str, max_age_hours: i64) -> Result<bool> {
    let cutoff = chrono::Utc::now() - chrono::Duration::hours(max_age_hours);
    let cutoff_str = cutoff.format("%Y-%m-%d %H:%M:%S").to_string();

    let row: Option<(i64,)> = sqlx::query_as(
        "SELECT COUNT(*) FROM fetch_log
         WHERE source = ? AND status = 'success' AND finished_at IS NOT NULL
           AND finished_at > ?",
    )
    .bind(source)
    .bind(&cutoff_str)
    .fetch_optional(pool)
    .await?;

    Ok(row.is_some_and(|(count,)| count > 0))
}

/// Record a fetch attempt in the fetch_log table.
pub async fn log_fetch(
    pool: &SqlitePool,
    source: &str,
    indicator: Option<&str>,
    status: &str,
    records_count: i64,
    error_message: Option<&str>,
) {
    let result = sqlx::query(
        "INSERT INTO fetch_log (source, indicator, status, records_count, error_message, finished_at)
         VALUES (?, ?, ?, ?, ?, datetime('now'))",
    )
    .bind(source)
    .bind(indicator)
    .bind(status)
    .bind(records_count)
    .bind(error_message)
    .execute(pool)
    .await;

    if let Err(e) = result {
        tracing::warn!("Failed to log fetch for {source}: {e}");
    }
}
