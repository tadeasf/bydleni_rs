use anyhow::{Context, Result, bail};
use serde::Deserialize;
use sqlx::SqlitePool;

use super::freshness;
use crate::config::Config;
use crate::models::time_series::{self, TimeSeries};

const FRED_BASE_URL: &str = "https://api.stlouisfed.org/fred/series/observations";

/// FRED series we track, with their indicator names and units.
const SERIES: &[(&str, &str, &str)] = &[
    ("QCZN628BIS", "nominal_property_price_index", "index"),
    ("QCZR628BIS", "real_property_price_index", "index"),
    ("CPALTT01CZM657N", "cpi", "index"),
    ("LRHUTTTTCZM156S", "unemployment_rate", "%"),
    ("CLVMEURNSAB1GQCZ", "real_gdp_per_capita", "EUR"),
];

#[derive(Debug, Deserialize)]
struct FredResponse {
    observations: Vec<FredObservation>,
}

#[derive(Debug, Deserialize)]
struct FredObservation {
    date: String,
    value: String,
}

/// Fetch all configured FRED series and store in the database.
pub async fn fetch_and_store(pool: &SqlitePool, config: &Config, force: bool) -> Result<()> {
    if !force && freshness::is_fresh(pool, "fred", 24).await.unwrap_or(false) {
        tracing::info!("FRED data is fresh, skipping fetch");
        return Ok(());
    }

    if config.fred_api_key.is_empty() {
        bail!("FRED_API_KEY is not set — skipping FRED fetch");
    }

    let client = reqwest::Client::new();
    let mut total_count: usize = 0;

    for &(series_id, indicator, unit) in SERIES {
        tracing::info!("Fetching FRED series {series_id} ({indicator})");

        let records = fetch_series(&client, &config.fred_api_key, series_id, indicator, unit)
            .await
            .with_context(|| format!("Failed to fetch FRED series {series_id}"))?;

        let count = time_series::upsert_batch(pool, &records).await?;
        tracing::info!("  Stored {count} records for {indicator}");
        total_count += count;
    }

    freshness::log_fetch(pool, "fred", None, "success", total_count as i64, None).await;

    Ok(())
}

/// Fetch a single FRED series and convert to TimeSeries records.
async fn fetch_series(
    client: &reqwest::Client,
    api_key: &str,
    series_id: &str,
    indicator: &str,
    unit: &str,
) -> Result<Vec<TimeSeries>> {
    let resp = client
        .get(FRED_BASE_URL)
        .query(&[("series_id", series_id), ("api_key", api_key), ("file_type", "json")])
        .send()
        .await?
        .error_for_status()?;

    let fred: FredResponse = resp.json().await?;

    let records: Vec<TimeSeries> = fred
        .observations
        .into_iter()
        .filter_map(|obs| {
            let value: f64 = obs.value.parse().ok()?;
            Some(TimeSeries {
                id: None,
                indicator: indicator.to_string(),
                region: "national".to_string(),
                date: obs.date,
                value,
                unit: unit.to_string(),
                source: "fred".to_string(),
                fetched_at: None,
            })
        })
        .collect();

    Ok(records)
}
