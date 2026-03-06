#![allow(dead_code)]

use anyhow::Result;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

/// A single observation in a time series.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct TimeSeries {
    pub id: Option<i64>,
    pub indicator: String,
    pub region: String,
    pub date: String,
    pub value: f64,
    pub unit: String,
    pub source: String,
    pub fetched_at: Option<String>,
}

/// Insert or replace a time series record (upserts on unique constraint).
pub async fn upsert(pool: &SqlitePool, record: &TimeSeries) -> Result<()> {
    sqlx::query(
        "INSERT OR REPLACE INTO time_series (indicator, region, date, value, unit, source)
         VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(&record.indicator)
    .bind(&record.region)
    .bind(&record.date)
    .bind(record.value)
    .bind(&record.unit)
    .bind(&record.source)
    .execute(pool)
    .await?;
    Ok(())
}

/// Batch insert time series records within a transaction.
pub async fn upsert_batch(pool: &SqlitePool, records: &[TimeSeries]) -> Result<usize> {
    let mut tx = pool.begin().await?;
    let mut count = 0;
    for record in records {
        sqlx::query(
            "INSERT OR REPLACE INTO time_series (indicator, region, date, value, unit, source)
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(&record.indicator)
        .bind(&record.region)
        .bind(&record.date)
        .bind(record.value)
        .bind(&record.unit)
        .bind(&record.source)
        .execute(&mut *tx)
        .await?;
        count += 1;
    }
    tx.commit().await?;
    Ok(count)
}

/// Query time series by indicator, region, and optional date range.
pub async fn query(
    pool: &SqlitePool,
    indicator: &str,
    region: &str,
    date_from: Option<&str>,
    date_to: Option<&str>,
) -> Result<Vec<TimeSeries>> {
    let mut sql = String::from(
        "SELECT id, indicator, region, date, value, unit, source, fetched_at
         FROM time_series WHERE indicator = ? AND region = ?",
    );
    if date_from.is_some() {
        sql.push_str(" AND date >= ?");
    }
    if date_to.is_some() {
        sql.push_str(" AND date <= ?");
    }
    sql.push_str(" ORDER BY date ASC");

    let mut q = sqlx::query_as::<_, TimeSeries>(&sql).bind(indicator).bind(region);
    if let Some(from) = date_from {
        q = q.bind(from);
    }
    if let Some(to) = date_to {
        q = q.bind(to);
    }

    let rows = q.fetch_all(pool).await?;
    Ok(rows)
}

/// Get the latest value for an indicator+region combination.
pub async fn latest(
    pool: &SqlitePool,
    indicator: &str,
    region: &str,
) -> Result<Option<TimeSeries>> {
    let row = sqlx::query_as::<_, TimeSeries>(
        "SELECT id, indicator, region, date, value, unit, source, fetched_at
         FROM time_series WHERE indicator = ? AND region = ?
         ORDER BY date DESC LIMIT 1",
    )
    .bind(indicator)
    .bind(region)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}
