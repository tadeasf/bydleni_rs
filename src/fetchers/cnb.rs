use anyhow::{Context, Result};
use sqlx::SqlitePool;

use super::freshness;
use crate::config::Config;
use crate::models::time_series::{self, TimeSeries};

/// CNB plain-text history files (freely available, no API key needed).
/// Format: "YYYYMMDD|value" with Czech decimal comma.
const CNB_REPO_RATE_URL: &str =
    "https://www.cnb.cz/cs/casto-kladene-dotazy/.galleries/vyvoj_repo_historie.txt";
const CNB_DISCOUNT_RATE_URL: &str =
    "https://www.cnb.cz/cs/casto-kladene-dotazy/.galleries/vyvoj_diskontni_historie.txt";
const CNB_LOMBARD_RATE_URL: &str =
    "https://www.cnb.cz/cs/casto-kladene-dotazy/.galleries/vyvoj_lombard_historie.txt";

/// CNB FX rate endpoint (plain text, daily).
const CNB_FX_URL: &str = "https://www.cnb.cz/cs/financni-trhy/devizovy-trh/\
    kurzy-devizoveho-trhu/kurzy-devizoveho-trhu/denni_kurz.txt";

/// Rate text files to fetch: (URL, indicator_name)
const RATE_FILES: &[(&str, &str)] = &[
    (CNB_REPO_RATE_URL, "repo_rate_2w"),
    (CNB_DISCOUNT_RATE_URL, "discount_rate"),
    (CNB_LOMBARD_RATE_URL, "lombard_rate"),
];

/// Fetch all CNB data and store in the database.
pub async fn fetch_and_store(pool: &SqlitePool, _config: &Config, force: bool) -> Result<()> {
    if !force && freshness::is_fresh(pool, "cnb", 12).await.unwrap_or(false) {
        tracing::info!("CNB data is fresh, skipping fetch");
        return Ok(());
    }

    let client = reqwest::Client::new();

    // Fetch CNB rate history text files
    for &(url, indicator) in RATE_FILES {
        tracing::info!("Fetching CNB {indicator}");
        match fetch_rate_history(&client, url, indicator).await {
            Ok(records) => {
                let count = time_series::upsert_batch(pool, &records).await?;
                tracing::info!("  Stored {count} records for {indicator}");
            }
            Err(e) => tracing::error!("  Failed to fetch {indicator}: {e:#}"),
        }
    }

    // Synthesize mortgage rates from repo_rate_2w + 2.5pp spread
    // (standard banking approximation; CNB doesn't publish mortgage rate history as plain text)
    tracing::info!("Synthesizing mortgage rates from repo_rate_2w + 2.5pp spread");
    match synthesize_mortgage_rates(pool).await {
        Ok(count) => tracing::info!("  Synthesized {count} mortgage rate records"),
        Err(e) => tracing::error!("  Failed to synthesize mortgage rates: {e:#}"),
    }

    // Fetch FX rates
    tracing::info!("Fetching CNB FX rates");
    match fetch_fx_rates(&client).await {
        Ok(records) => {
            let count = time_series::upsert_batch(pool, &records).await?;
            tracing::info!("  Stored {count} FX rate records");
        }
        Err(e) => tracing::error!("  Failed to fetch FX rates: {e:#}"),
    }

    freshness::log_fetch(pool, "cnb", None, "success", 0, None).await;

    Ok(())
}

/// Parse CNB rate history text files.
///
/// Format:
/// ```text
/// PLATNA_OD|CNB_REPO_SAZBA_V_%
/// 19951208|11,30
/// 19960329|11,50
/// ```
async fn fetch_rate_history(
    client: &reqwest::Client,
    url: &str,
    indicator: &str,
) -> Result<Vec<TimeSeries>> {
    let text = client
        .get(url)
        .send()
        .await?
        .text()
        .await
        .with_context(|| format!("Failed to read CNB {indicator} response"))?;

    let mut records = Vec::new();
    for line in text.lines().skip(1) {
        // skip header
        let parts: Vec<&str> = line.split('|').collect();
        if parts.len() < 2 {
            continue;
        }

        let raw_date = parts[0].trim();
        let raw_value = parts[1].trim();

        // Parse date from YYYYMMDD to YYYY-MM-DD
        if raw_date.len() != 8 {
            continue;
        }
        let date = format!("{}-{}-{}", &raw_date[..4], &raw_date[4..6], &raw_date[6..8]);

        let value: f64 = match raw_value.replace(',', ".").parse() {
            Ok(v) => v,
            Err(_) => continue,
        };

        records.push(TimeSeries {
            id: None,
            indicator: indicator.to_string(),
            region: "national".to_string(),
            date,
            value,
            unit: "%".to_string(),
            source: "cnb".to_string(),
            fetched_at: None,
        });
    }

    Ok(records)
}

/// Synthesize mortgage_rate_avg from repo_rate_2w + 2.5pp spread.
/// Standard banking approximation; CNB doesn't publish mortgage rate history as plain text.
async fn synthesize_mortgage_rates(pool: &SqlitePool) -> Result<usize> {
    let repo_rows: Vec<(String, f64)> = sqlx::query_as(
        "SELECT date, value FROM time_series
         WHERE indicator = 'repo_rate_2w' AND region = 'national'
         ORDER BY date ASC",
    )
    .fetch_all(pool)
    .await?;

    let records: Vec<TimeSeries> = repo_rows
        .into_iter()
        .map(|(date, value)| TimeSeries {
            id: None,
            indicator: "mortgage_rate_avg".to_string(),
            region: "national".to_string(),
            date,
            value: value + 2.5,
            unit: "%".to_string(),
            source: "cnb_derived".to_string(),
            fetched_at: None,
        })
        .collect();

    let count = time_series::upsert_batch(pool, &records).await?;
    Ok(count)
}

/// Fetch daily FX rates from the CNB plain-text endpoint.
///
/// Format:
/// ```text
/// 06.03.2026 #046
/// země|měna|množství|kód|kurz
/// Austrálie|dolar|1|AUD|14,788
/// ```
async fn fetch_fx_rates(client: &reqwest::Client) -> Result<Vec<TimeSeries>> {
    let text = client
        .get(CNB_FX_URL)
        .send()
        .await?
        .text()
        .await
        .context("Failed to read CNB FX response")?;

    let mut lines = text.lines();
    let header_line = lines.next().context("Empty FX response")?;

    // Parse date from "06.03.2026 #046"
    let date_part = header_line.split(' ').next().unwrap_or("");
    let date_parts: Vec<&str> = date_part.split('.').collect();
    let iso_date = if date_parts.len() == 3 {
        format!("{}-{}-{}", date_parts[2], date_parts[1], date_parts[0])
    } else {
        chrono::Utc::now().format("%Y-%m-%d").to_string()
    };

    // Skip column header line
    lines.next();

    let mut records = Vec::new();
    for line in lines {
        let parts: Vec<&str> = line.split('|').collect();
        if parts.len() < 5 {
            continue;
        }
        let code = parts[3];
        let amount: f64 = parts[2].parse().unwrap_or(1.0);
        let rate: f64 = parts[4].replace(',', ".").parse().unwrap_or(0.0);
        if rate == 0.0 {
            continue;
        }

        // Store rate per 1 unit of foreign currency
        let rate_per_unit = rate / amount;
        records.push(TimeSeries {
            id: None,
            indicator: format!("fx_{}", code.to_lowercase()),
            region: "national".to_string(),
            date: iso_date.clone(),
            value: rate_per_unit,
            unit: "CZK".to_string(),
            source: "cnb".to_string(),
            fetched_at: None,
        });
    }

    Ok(records)
}
