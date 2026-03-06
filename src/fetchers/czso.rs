use std::collections::HashMap;

use anyhow::{Context, Result};
use sqlx::SqlitePool;

use super::freshness;
use crate::config::Config;
use crate::models::time_series::{self, TimeSeries};

/// CZSO open data: package_show API for dataset metadata.
const CZSO_PACKAGE_API: &str = "https://vdb.czso.cz/pll/eweb/package_show";

/// Dataset IDs we fetch from CZSO.
/// 110080 = Average gross monthly wage and median by region (yearly, 2011+)
const WAGE_DATASET_ID: &str = "110080";

/// Map of CZSO uzemi_txt values to our region slugs.
fn region_slug_map() -> HashMap<&'static str, &'static str> {
    HashMap::from([
        ("Česká republika", "national"),
        ("Hlavní město Praha", "praha"),
        ("Středočeský kraj", "stredocesky"),
        ("Jihočeský kraj", "jihocesky"),
        ("Plzeňský kraj", "plzensky"),
        ("Karlovarský kraj", "karlovarsky"),
        ("Ústecký kraj", "ustecky"),
        ("Liberecký kraj", "liberecky"),
        ("Královéhradecký kraj", "kralovehradecky"),
        ("Pardubický kraj", "pardubicky"),
        ("Kraj Vysočina", "vysocina"),
        ("Jihomoravský kraj", "jihomoravsky"),
        ("Olomoucký kraj", "olomoucky"),
        ("Zlínský kraj", "zlinsky"),
        ("Moravskoslezský kraj", "moravskoslezsky"),
    ])
}

/// Fetch all CZSO datasets and store in the database.
pub async fn fetch_and_store(pool: &SqlitePool, _config: &Config, force: bool) -> Result<()> {
    if !force && freshness::is_fresh(pool, "czso", 24).await.unwrap_or(false) {
        tracing::info!("CZSO data is fresh, skipping fetch");
        return Ok(());
    }

    let client = reqwest::Client::builder().timeout(std::time::Duration::from_secs(60)).build()?;

    // Fetch wage data
    tracing::info!("Fetching CZSO wage data (dataset {WAGE_DATASET_ID})");
    match fetch_dataset(&client, WAGE_DATASET_ID).await {
        Ok(csv_url) => match fetch_wage_csv(&client, &csv_url).await {
            Ok(records) => {
                let count = time_series::upsert_batch(pool, &records).await?;
                tracing::info!("  Stored {count} wage records");
            }
            Err(e) => tracing::error!("  Failed to parse wage CSV: {e:#}"),
        },
        Err(e) => tracing::error!("  Failed to get wage dataset metadata: {e:#}"),
    }

    freshness::log_fetch(pool, "czso", None, "success", 0, None).await;

    Ok(())
}

/// Fetch dataset metadata from CZSO package_show API to get the CSV download URL.
async fn fetch_dataset(client: &reqwest::Client, dataset_id: &str) -> Result<String> {
    let resp = client
        .get(CZSO_PACKAGE_API)
        .query(&[("id", dataset_id)])
        .send()
        .await?
        .error_for_status()
        .with_context(|| format!("CZSO package_show failed for {dataset_id}"))?;

    let json: serde_json::Value = resp.json().await?;

    let success = json["success"].as_bool().unwrap_or(false);
    if !success {
        anyhow::bail!(
            "CZSO API returned error for dataset {dataset_id}: {}",
            json["error"]["message"].as_str().unwrap_or("unknown")
        );
    }

    // Find the CSV resource URL
    let resources = json["result"]["resources"].as_array().context("No resources in dataset")?;

    let csv_url = resources
        .iter()
        .find(|r| r["format"].as_str().is_some_and(|f| f.contains("csv")))
        .and_then(|r| r["url"].as_str())
        .context("No CSV resource found in dataset")?;

    Ok(csv_url.to_string())
}

/// Parse the CZSO wage CSV (dataset 110080).
///
/// Columns: idhod, hodnota, stapro_kod, SPKVANTIL_cis, SPKVANTIL_kod,
///          POHLAVI_cis, POHLAVI_kod, rok, uzemi_cis, uzemi_kod,
///          STAPRO_TXT, uzemi_txt, SPKVANTIL_txt, POHLAVI_txt
///
/// We want rows where:
/// - SPKVANTIL_kod is empty (average, not median)
/// - POHLAVI_kod is empty (both sexes combined)
async fn fetch_wage_csv(client: &reqwest::Client, csv_url: &str) -> Result<Vec<TimeSeries>> {
    let csv_text = client
        .get(csv_url)
        .send()
        .await?
        .text()
        .await
        .context("Failed to download CZSO wage CSV")?;

    let slug_map = region_slug_map();
    let mut records = Vec::new();

    let mut rdr = csv::ReaderBuilder::new().has_headers(true).from_reader(csv_text.as_bytes());

    let headers = rdr.headers().context("Failed to read CSV headers")?.clone();

    // Find column indices
    let hodnota_idx = col_index(&headers, "hodnota")?;
    let spkvantil_kod_idx = col_index(&headers, "SPKVANTIL_kod")?;
    let pohlavi_kod_idx = col_index(&headers, "POHLAVI_kod")?;
    let rok_idx = col_index(&headers, "rok")?;
    let uzemi_txt_idx = col_index(&headers, "uzemi_txt")?;

    for result in rdr.records() {
        let record = match result {
            Ok(r) => r,
            Err(_) => continue,
        };

        // Filter: average wage (not median), both sexes
        let spkvantil = record.get(spkvantil_kod_idx).unwrap_or("").trim();
        let pohlavi = record.get(pohlavi_kod_idx).unwrap_or("").trim();
        if !spkvantil.is_empty() || !pohlavi.is_empty() {
            continue;
        }

        let uzemi = record.get(uzemi_txt_idx).unwrap_or("").trim();
        let region = match slug_map.get(uzemi) {
            Some(slug) => slug.to_string(),
            None => continue,
        };

        let rok = record.get(rok_idx).unwrap_or("").trim();
        let hodnota = record.get(hodnota_idx).unwrap_or("").trim();

        let value: f64 = match hodnota.parse() {
            Ok(v) => v,
            Err(_) => continue,
        };

        records.push(TimeSeries {
            id: None,
            indicator: "avg_monthly_wage".to_string(),
            region,
            date: format!("{rok}-01-01"),
            value,
            unit: "CZK".to_string(),
            source: "czso".to_string(),
            fetched_at: None,
        });
    }

    Ok(records)
}

/// Find column index by exact header name.
fn col_index(headers: &csv::StringRecord, name: &str) -> Result<usize> {
    headers
        .iter()
        .position(|h| h == name)
        .with_context(|| format!("Column '{name}' not found in CSV headers"))
}
