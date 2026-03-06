use std::time::Duration;

use anyhow::{Context, Result};
use serde::Deserialize;
use sqlx::SqlitePool;
use tokio::time::sleep;

use super::freshness;
use crate::config::Config;
use crate::models::listing::{self, ExampleListing};
use crate::models::time_series::{self, TimeSeries};

/// Sreality API base URL (discovered from their SPA).
const SREALITY_API_BASE: &str = "https://www.sreality.cz/api/cs/v2/estates";

/// Delay between API requests to be respectful.
const REQUEST_DELAY: Duration = Duration::from_millis(1500);

/// Locality filter for Sreality API.
enum LocalityFilter {
    /// Use `locality_region_id` param (for Praha which spans multiple districts).
    Region(i64),
    /// Use `locality_district_id` param (for all other cities).
    District(i64),
}

/// Regional capitals with their Sreality locality filters.
/// IDs discovered via the Sreality suggest API.
const REGIONS: &[(&str, &str, LocalityFilter)] = &[
    ("praha", "Praha", LocalityFilter::Region(10)),
    ("brno", "Brno", LocalityFilter::District(72)),
    ("ostrava", "Ostrava", LocalityFilter::District(65)),
    ("plzen", "Plzeň", LocalityFilter::District(12)),
    ("liberec", "Liberec", LocalityFilter::District(22)),
    ("olomouc", "Olomouc", LocalityFilter::District(42)),
    ("hradec_kralove", "Hradec Králové", LocalityFilter::District(28)),
    ("ceske_budejovice", "České Budějovice", LocalityFilter::District(1)),
    ("usti_nad_labem", "Ústí nad Labem", LocalityFilter::District(27)),
    ("pardubice", "Pardubice", LocalityFilter::District(32)),
    ("zlin", "Zlín", LocalityFilter::District(38)),
    ("karlovy_vary", "Karlovy Vary", LocalityFilter::District(10)),
    ("jihlava", "Jihlava", LocalityFilter::District(67)),
    ("stredocesky", "Středočeský kraj", LocalityFilter::Region(11)),
];

/// Sreality estate categories and types.
const CATEGORY_FLAT: u32 = 1;
const TYPE_SALE: u32 = 1;
const TYPE_RENT: u32 = 2;

#[derive(Debug, Deserialize)]
struct SrealityResponse {
    #[serde(default, rename = "_embedded")]
    embedded: SrealityEmbedded,
    result_size: Option<u64>,
}

#[derive(Debug, Default, Deserialize)]
struct SrealityEmbedded {
    #[serde(default)]
    estates: Vec<SrealityEstate>,
}

#[derive(Debug, Deserialize)]
struct SrealityEstate {
    #[serde(default)]
    price: i64,
    #[serde(default)]
    price_czk: Option<PriceCzk>,
    #[serde(default)]
    name: String,
    #[serde(default)]
    hash_id: Option<i64>,
    #[serde(default)]
    seo: Option<SreaSeo>,
}

#[derive(Debug, Default, Deserialize)]
struct SreaSeo {
    #[serde(default)]
    locality: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PriceCzk {
    #[serde(default)]
    value_raw: Option<i64>,
}

/// Fetch sale and rental data from Sreality and store aggregates.
pub async fn fetch_and_store(pool: &SqlitePool, _config: &Config, force: bool) -> Result<()> {
    if !force && freshness::is_fresh(pool, "sreality", 6).await.unwrap_or(false) {
        tracing::info!("Sreality data is fresh, skipping fetch");
        return Ok(());
    }

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .user_agent("Mozilla/5.0 (X11; Linux x86_64) bydleni_rs/0.1.0")
        .build()?;

    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();

    for (slug, name, locality_filter) in REGIONS {
        tracing::info!("Fetching Sreality data for {name}");

        // Fetch sale prices
        match fetch_prices(&client, locality_filter, TYPE_SALE).await {
            Ok((prices, estates)) if !prices.is_empty() => {
                let avg = prices.iter().sum::<f64>() / prices.len() as f64;
                let record = TimeSeries {
                    id: None,
                    indicator: "avg_asking_price_m2_flat".to_string(),
                    region: slug.to_string(),
                    date: today.clone(),
                    value: avg,
                    unit: "CZK/m2".to_string(),
                    source: "sreality".to_string(),
                    fetched_at: None,
                };
                time_series::upsert(pool, &record).await?;
                tracing::info!("  {name} sale: avg {avg:.0} CZK/m2 from {} listings", prices.len());

                // Store 3 example listings near median
                let examples = select_near_median(&estates, 3);
                let listings: Vec<ExampleListing> = examples
                    .iter()
                    .map(|e| ExampleListing {
                        id: None,
                        region: slug.to_string(),
                        listing_type: "sale".to_string(),
                        name: e.name.clone(),
                        price: e.price,
                        area_m2: Some(e.area),
                        price_per_m2: Some(e.price_per_m2),
                        url: build_listing_url(e, TYPE_SALE, slug),
                        fetched_at: None,
                    })
                    .collect();
                if let Err(e) = listing::upsert_batch(pool, slug, "sale", &listings).await {
                    tracing::warn!("  Failed to store sale listings for {name}: {e:#}");
                }
            }
            Ok(_) => tracing::warn!("  {name} sale: no listings found"),
            Err(e) => tracing::error!("  {name} sale fetch failed: {e:#}"),
        }

        sleep(REQUEST_DELAY).await;

        // Fetch rental prices
        match fetch_prices(&client, locality_filter, TYPE_RENT).await {
            Ok((prices, estates)) if !prices.is_empty() => {
                let avg = prices.iter().sum::<f64>() / prices.len() as f64;
                let record = TimeSeries {
                    id: None,
                    indicator: "avg_rent_m2_flat".to_string(),
                    region: slug.to_string(),
                    date: today.clone(),
                    value: avg,
                    unit: "CZK/m2".to_string(),
                    source: "sreality".to_string(),
                    fetched_at: None,
                };
                time_series::upsert(pool, &record).await?;
                tracing::info!("  {name} rent: avg {avg:.0} CZK/m2 from {} listings", prices.len());

                // Store 3 example rent listings near median
                let examples = select_near_median(&estates, 3);
                let listings: Vec<ExampleListing> = examples
                    .iter()
                    .map(|e| ExampleListing {
                        id: None,
                        region: slug.to_string(),
                        listing_type: "rent".to_string(),
                        name: e.name.clone(),
                        price: e.price,
                        area_m2: Some(e.area),
                        price_per_m2: Some(e.price_per_m2),
                        url: build_listing_url(e, TYPE_RENT, slug),
                        fetched_at: None,
                    })
                    .collect();
                if let Err(e) = listing::upsert_batch(pool, slug, "rent", &listings).await {
                    tracing::warn!("  Failed to store rent listings for {name}: {e:#}");
                }
            }
            Ok(_) => tracing::warn!("  {name} rent: no listings found"),
            Err(e) => tracing::error!("  {name} rent fetch failed: {e:#}"),
        }

        sleep(REQUEST_DELAY).await;
    }

    freshness::log_fetch(pool, "sreality", None, "success", 0, None).await;

    Ok(())
}

/// A parsed estate with computed per-m2 price.
struct ParsedEstate {
    name: String,
    price: i64,
    area: f64,
    price_per_m2: f64,
    hash_id: Option<i64>,
    seo_locality: Option<String>,
}

/// Fetch estates for a region and listing type.
/// Returns per-m2 prices and parsed estate data for example listing selection.
async fn fetch_prices(
    client: &reqwest::Client,
    locality_filter: &LocalityFilter,
    listing_type: u32,
) -> Result<(Vec<f64>, Vec<ParsedEstate>)> {
    let mut all_prices_per_m2 = Vec::new();
    let mut all_estates = Vec::new();
    let per_page = 60;
    let max_pages = 5; // Cap at 300 listings per region

    let (locality_param, locality_value) = match locality_filter {
        LocalityFilter::Region(id) => ("locality_region_id", id.to_string()),
        LocalityFilter::District(id) => ("locality_district_id", id.to_string()),
    };

    for page in 0..max_pages {
        let resp = client
            .get(SREALITY_API_BASE)
            .query(&[
                ("category_main_cb", CATEGORY_FLAT.to_string()),
                ("category_type_cb", listing_type.to_string()),
                (locality_param, locality_value.clone()),
                ("per_page", per_page.to_string()),
                ("page", (page + 1).to_string()),
                ("sort", "0".to_string()),
            ])
            .send()
            .await
            .context("Sreality HTTP request failed")?;

        if !resp.status().is_success() {
            tracing::warn!("Sreality returned status {} for page {}", resp.status(), page + 1);
            break;
        }

        let text = resp.text().await.context("Failed to read Sreality response")?;
        let data: SrealityResponse =
            serde_json::from_str(&text).context("Failed to parse Sreality JSON")?;

        let total = data.result_size.unwrap_or(0);
        if data.embedded.estates.is_empty() {
            break;
        }

        for estate in &data.embedded.estates {
            let price = estate.price_czk.as_ref().and_then(|p| p.value_raw).unwrap_or(estate.price);

            if price <= 0 {
                continue;
            }

            // Extract area from estate name (e.g., "Prodej bytu 3+kk 75 m²")
            if let Some(area) = extract_area_from_name(&estate.name)
                && area > 0.0
            {
                let ppm2 = price as f64 / area;
                all_prices_per_m2.push(ppm2);
                all_estates.push(ParsedEstate {
                    name: estate.name.clone(),
                    price,
                    area,
                    price_per_m2: ppm2,
                    hash_id: estate.hash_id,
                    seo_locality: estate.seo.as_ref().and_then(|s| s.locality.clone()),
                });
            }
        }

        // Stop if we've fetched all available listings
        if (page + 1) * per_page as u64 >= total {
            break;
        }

        sleep(REQUEST_DELAY).await;
    }

    Ok((all_prices_per_m2, all_estates))
}

/// Select up to `count` listings closest to the median price per m2.
fn select_near_median(estates: &[ParsedEstate], count: usize) -> Vec<&ParsedEstate> {
    if estates.is_empty() {
        return Vec::new();
    }
    let mut prices: Vec<f64> = estates.iter().map(|e| e.price_per_m2).collect();
    prices.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let median = prices[prices.len() / 2];

    let mut sorted: Vec<(f64, usize)> =
        estates.iter().enumerate().map(|(i, e)| ((e.price_per_m2 - median).abs(), i)).collect();
    sorted.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

    sorted.iter().take(count).map(|(_, i)| &estates[*i]).collect()
}

/// Extract layout type from listing name (e.g. "3+kk", "2+1").
fn extract_layout_from_name(name: &str) -> Option<String> {
    let normalized = name.replace('\u{a0}', " ");
    // Look for patterns like "3+kk", "2+1", "1+kk"
    for word in normalized.split_whitespace() {
        if word.contains('+')
            && word.len() >= 3
            && word.chars().next().is_some_and(|c| c.is_ascii_digit())
        {
            return Some(word.to_lowercase());
        }
    }
    None
}

/// Build a Sreality detail URL for a listing, or fall back to search URL.
fn build_listing_url(estate: &ParsedEstate, listing_type: u32, region_slug: &str) -> String {
    let type_slug = if listing_type == TYPE_SALE { "prodej" } else { "pronajem" };
    let layout = extract_layout_from_name(&estate.name).unwrap_or_else(|| "byt".to_string());
    let sreality_slug = region_slug.replace('_', "-");

    if let (Some(hash_id), Some(locality)) = (estate.hash_id, &estate.seo_locality) {
        format!("https://www.sreality.cz/detail/{type_slug}/byt/{layout}/{locality}/{hash_id}")
    } else if let Some(hash_id) = estate.hash_id {
        // No SEO locality, use region slug as fallback locality
        format!("https://www.sreality.cz/detail/{type_slug}/byt/{layout}/{sreality_slug}/{hash_id}")
    } else {
        // No hash_id at all — fall back to search page
        format!("https://www.sreality.cz/hledani/{type_slug}/byty/{sreality_slug}")
    }
}

/// Try to extract flat area in m2 from Sreality listing name.
///
/// Names look like: "Prodej bytu 3+kk 75\u{a0}m²" (note: non-breaking space \xa0)
fn extract_area_from_name(name: &str) -> Option<f64> {
    // Normalize non-breaking spaces to regular spaces, then find "N m" pattern
    let normalized = name.replace('\u{a0}', " ");

    // Find "m²" or "m2" and look for the number before it
    for pattern in &["m²", "m2", "m "] {
        if let Some(m_pos) = normalized.find(pattern) {
            let before = normalized[..m_pos].trim_end();
            // Extract the last "word" which should be the number
            let num_str = before.rsplit_once(|c: char| !c.is_ascii_digit() && c != '.');
            let num_str = match num_str {
                Some((_, n)) => n,
                None => before,
            };
            if let Ok(area) = num_str.parse::<f64>()
                && (10.0..=500.0).contains(&area)
            {
                return Some(area);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_area_from_name() {
        assert_eq!(extract_area_from_name("Prodej bytu 3+kk 75 m²"), Some(75.0));
        assert_eq!(extract_area_from_name("Pronájem bytu 2+1 55 m²"), Some(55.0));
        assert_eq!(extract_area_from_name("Prodej bytu 120 m²"), Some(120.0));
        // Non-breaking space (\u00a0) — actual Sreality format
        assert_eq!(extract_area_from_name("Prodej bytu 3+1\u{a0}67\u{a0}m²"), Some(67.0));
        assert_eq!(extract_area_from_name("Prodej bytu 2+kk 44\u{a0}m²"), Some(44.0));
        assert_eq!(extract_area_from_name("No area here"), None);
    }

    #[test]
    fn test_extract_layout_from_name() {
        assert_eq!(extract_layout_from_name("Prodej bytu 3+kk 75 m²"), Some("3+kk".to_string()));
        assert_eq!(extract_layout_from_name("Pronájem bytu 2+1 55 m²"), Some("2+1".to_string()));
        assert_eq!(extract_layout_from_name("Prodej bytu 4+KK 120 m²"), Some("4+kk".to_string()));
        assert_eq!(extract_layout_from_name("Prodej domu"), None);
    }
}
