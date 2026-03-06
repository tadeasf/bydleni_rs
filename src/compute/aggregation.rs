#![allow(dead_code)]

use anyhow::Result;
use sqlx::SqlitePool;

use crate::models::time_series::TimeSeries;

/// Compute year-over-year change for an indicator+region.
///
/// Returns a vector of (date, yoy_change_pct) pairs.
pub async fn yoy_change(
    pool: &SqlitePool,
    indicator: &str,
    region: &str,
) -> Result<Vec<(String, f64)>> {
    let rows = crate::models::time_series::query(pool, indicator, region, None, None).await?;

    let mut changes = Vec::new();
    for i in 0..rows.len() {
        // Find a row approximately 12 months (or 4 quarters) earlier
        let current = &rows[i];
        if let Some(prev) = find_yoy_match(&rows[..i], &current.date)
            && prev.value != 0.0
        {
            let change = (current.value - prev.value) / prev.value * 100.0;
            changes.push((current.date.clone(), change));
        }
    }

    Ok(changes)
}

/// Compute a simple moving average over the last `window` data points.
pub async fn moving_average(
    pool: &SqlitePool,
    indicator: &str,
    region: &str,
    window: usize,
) -> Result<Vec<TimeSeries>> {
    let rows = crate::models::time_series::query(pool, indicator, region, None, None).await?;

    let mut result = Vec::new();
    for i in 0..rows.len() {
        let start = i.saturating_sub(window - 1);
        let slice = &rows[start..=i];
        let avg = slice.iter().map(|r| r.value).sum::<f64>() / slice.len() as f64;

        let mut smoothed = rows[i].clone();
        smoothed.value = avg;
        result.push(smoothed);
    }

    Ok(result)
}

/// Compute national average from all regional data for a given indicator and date.
pub async fn national_average(
    pool: &SqlitePool,
    indicator: &str,
    date: &str,
) -> Result<Option<f64>> {
    let row: Option<(f64,)> = sqlx::query_as(
        "SELECT AVG(value) FROM time_series
         WHERE indicator = ? AND date = ? AND region != 'national'",
    )
    .bind(indicator)
    .bind(date)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|(v,)| v))
}

/// Find a record approximately one year before the given date.
fn find_yoy_match<'a>(rows: &'a [TimeSeries], current_date: &str) -> Option<&'a TimeSeries> {
    // Parse current date to find target year-ago date
    let target = year_ago(current_date)?;

    // Find closest match
    rows.iter()
        .filter(|r| date_distance(&r.date, &target) <= 45) // Within ~45 days
        .min_by_key(|r| date_distance(&r.date, &target))
}

/// Compute approximate number of days between two date strings.
fn date_distance(a: &str, b: &str) -> u32 {
    let parse = |s: &str| -> Option<i32> {
        if s.contains("-Q") {
            // Quarter format: "2023-Q1"
            let year: i32 = s[..4].parse().ok()?;
            let quarter: i32 = s[6..].parse().ok()?;
            Some(year * 365 + (quarter - 1) * 91)
        } else if s.len() >= 10 {
            // ISO date
            let year: i32 = s[..4].parse().ok()?;
            let month: i32 = s[5..7].parse().ok()?;
            let day: i32 = s[8..10].parse().ok()?;
            Some(year * 365 + (month - 1) * 30 + day)
        } else if s.len() == 4 {
            // Just year
            let year: i32 = s.parse().ok()?;
            Some(year * 365 + 182)
        } else {
            None
        }
    };

    match (parse(a), parse(b)) {
        (Some(da), Some(db)) => (da - db).unsigned_abs(),
        _ => u32::MAX,
    }
}

/// Return a date string approximately one year before the given date.
fn year_ago(date: &str) -> Option<String> {
    if date.contains("-Q") {
        // "2023-Q2" -> "2022-Q2"
        let year: i32 = date[..4].parse().ok()?;
        Some(format!("{}-{}", year - 1, &date[4..]))
    } else if date.len() >= 10 {
        let year: i32 = date[..4].parse().ok()?;
        Some(format!("{}{}", year - 1, &date[4..]))
    } else if date.len() == 4 {
        let year: i32 = date.parse().ok()?;
        Some((year - 1).to_string())
    } else {
        None
    }
}
