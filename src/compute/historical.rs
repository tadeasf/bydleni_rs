use std::collections::HashMap;

use anyhow::Result;
use sqlx::SqlitePool;

use crate::compute::affordability::{
    living_expenses, mortgage_monthly_payment, years_to_save_with_investment,
};
use crate::compute::czech_tax;
use crate::models::affordability::{self, Affordability};

/// Historical snapshot years to compute.
const SNAPSHOT_YEARS: &[i32] = &[2010, 2015, 2020, 2025];

/// All regions we compute for (same as in affordability.rs).
const REGIONS: &[&str] = &[
    "national",
    "praha",
    "brno",
    "ostrava",
    "plzen",
    "liberec",
    "olomouc",
    "hradec_kralove",
    "ceske_budejovice",
    "usti_nad_labem",
    "pardubice",
    "zlin",
    "karlovy_vary",
    "jihlava",
    "stredocesky",
];

/// Map city slugs to CZSO kraj slugs (duplicated from affordability.rs for independence).
fn city_to_kraj(city: &str) -> Option<&'static str> {
    match city {
        "praha" => Some("praha"),
        "brno" => Some("jihomoravsky"),
        "ostrava" => Some("moravskoslezsky"),
        "plzen" => Some("plzensky"),
        "liberec" => Some("liberecky"),
        "olomouc" => Some("olomoucky"),
        "hradec_kralove" => Some("kralovehradecky"),
        "ceske_budejovice" => Some("jihocesky"),
        "usti_nad_labem" => Some("ustecky"),
        "pardubice" => Some("pardubicky"),
        "zlin" => Some("zlinsky"),
        "karlovy_vary" => Some("karlovarsky"),
        "jihlava" => Some("vysocina"),
        "stredocesky" => Some("stredocesky"),
        _ => None,
    }
}

/// Compute historical affordability snapshots for all target years.
pub async fn compute_historical_snapshots(pool: &SqlitePool) -> Result<()> {
    tracing::info!("Computing historical affordability snapshots...");

    // Get current national avg price as average of all regional Sreality prices
    let current_national_price_m2: Option<(f64,)> = sqlx::query_as(
        "SELECT AVG(value) FROM time_series
         WHERE indicator = 'avg_asking_price_m2_flat'
           AND region != 'national'
           AND date = (SELECT MAX(date) FROM time_series
                       WHERE indicator = 'avg_asking_price_m2_flat')",
    )
    .fetch_optional(pool)
    .await?;

    // Build a map of current regional prices (latest date for each region)
    let regional_rows: Vec<(String, f64)> = sqlx::query_as(
        "SELECT region, value FROM time_series
         WHERE indicator = 'avg_asking_price_m2_flat'
           AND region != 'national'
           AND date = (SELECT MAX(date) FROM time_series
                       WHERE indicator = 'avg_asking_price_m2_flat')",
    )
    .fetch_all(pool)
    .await?;

    let current_regional_prices: HashMap<String, f64> = regional_rows.into_iter().collect();

    // Get property price index time series (FRED, base 2010=100)
    let price_index_rows: Vec<(String, f64)> = sqlx::query_as(
        "SELECT date, value FROM time_series
         WHERE indicator = 'nominal_property_price_index' AND region = 'national'
         ORDER BY date ASC",
    )
    .fetch_all(pool)
    .await?;

    // Find the latest index value (to scale current prices backward)
    let latest_index = price_index_rows.last().map(|(_, v)| *v);

    for &year in SNAPSHOT_YEARS {
        let date = format!("{year}-01-01");

        for &region in REGIONS {
            if let Err(e) = compute_snapshot(
                pool,
                region,
                year,
                &date,
                current_national_price_m2.map(|(v,)| v),
                &current_regional_prices,
                &price_index_rows,
                latest_index,
            )
            .await
            {
                tracing::warn!("Failed to compute {year} snapshot for {region}: {e:#}");
            }
        }
    }

    tracing::info!("Historical snapshots complete");
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn compute_snapshot(
    pool: &SqlitePool,
    region: &str,
    year: i32,
    date: &str,
    current_national_price_m2: Option<f64>,
    current_regional_prices: &HashMap<String, f64>,
    price_index_rows: &[(String, f64)],
    latest_index: Option<f64>,
) -> Result<()> {
    let year_str = year.to_string();

    // --- Wage: find closest to target year ---
    let wage_region = city_to_kraj(region).unwrap_or(region);
    let avg_monthly_wage = find_closest_value(pool, "avg_monthly_wage", wage_region, &year_str)
        .await?
        .or(find_closest_value(pool, "avg_monthly_wage", "national", &year_str).await?);

    // --- Price per m2: scale current price backward using FRED property price index ---
    let avg_price_m2 = if region == "national" {
        // National: scale the average of all regional prices
        match (current_national_price_m2, latest_index) {
            (Some(current_price), Some(latest_idx)) => find_closest_index(price_index_rows, year)
                .map(|idx| current_price * idx / latest_idx),
            _ => None,
        }
    } else {
        // Regional: scale the current regional price
        let current_price = current_regional_prices.get(region).copied();
        match (current_price.or(current_national_price_m2), latest_index) {
            (Some(price), Some(latest_idx)) => {
                find_closest_index(price_index_rows, year).map(|idx| price * idx / latest_idx)
            }
            _ => None,
        }
    };

    // --- Mortgage rate ---
    let mortgage_rate = find_closest_value(pool, "mortgage_rate_avg", "national", &year_str)
        .await?
        .or(find_closest_value(pool, "repo_rate_2w", "national", &year_str)
            .await?
            .map(|repo| repo + 2.5));

    // --- Rent: try direct data, fallback to scaling current rent with price index ---
    let avg_rent_m2 = match find_closest_value(pool, "avg_rent_m2_flat", region, &year_str).await? {
        Some(v) => Some(v),
        None => {
            // Estimate historical rent by scaling current rent with property price index
            let current_rent = get_current_rent_m2(pool, region).await?;
            match (current_rent, latest_index) {
                (Some(rent), Some(latest_idx)) if latest_idx > 0.0 => {
                    let target_idx =
                        find_closest_index(price_index_rows, year).unwrap_or(latest_idx);
                    Some(rent * target_idx / latest_idx)
                }
                _ => None,
            }
        }
    };

    // --- Derive metrics ---
    let flat_60m2_price = avg_price_m2.map(|p| p * 60.0);
    let avg_monthly_wage_net = avg_monthly_wage.map(czech_tax::gross_to_net_monthly);

    let months_to_buy = match (flat_60m2_price, avg_monthly_wage_net) {
        (Some(price), Some(wage)) if wage > 0.0 => Some(price / wage),
        _ => None,
    };

    let months_to_buy_gross = match (flat_60m2_price, avg_monthly_wage) {
        (Some(price), Some(wage)) if wage > 0.0 => Some(price / wage),
        _ => None,
    };

    let monthly_payment = match (flat_60m2_price, mortgage_rate) {
        (Some(price), Some(rate)) => Some(mortgage_monthly_payment(price * 0.8, rate, 360)),
        _ => None,
    };

    let payment_to_wage_pct = match (monthly_payment, avg_monthly_wage_net) {
        (Some(payment), Some(wage)) if wage > 0.0 => Some(payment / wage * 100.0),
        _ => None,
    };

    let monthly_rent_60m2 = avg_rent_m2.map(|r| r * 60.0);
    let rent_vs_mortgage_ratio = match (monthly_rent_60m2, monthly_payment) {
        (Some(rent), Some(mortgage)) if mortgage > 0.0 => Some(rent / mortgage),
        _ => None,
    };

    let expenses = living_expenses(region);
    let monthly_savings = avg_monthly_wage_net.map(|net| (net - expenses).max(0.0));
    let years_to_save = match (flat_60m2_price, monthly_savings) {
        (Some(target), Some(savings)) if savings > 0.0 => {
            Some(years_to_save_with_investment(target, savings, 7.0))
        }
        _ => None,
    };

    // Only store if we have at least some data
    if months_to_buy.is_none() && avg_price_m2.is_none() && avg_monthly_wage.is_none() {
        return Ok(());
    }

    let record = Affordability {
        id: None,
        region: region.to_string(),
        date: date.to_string(),
        avg_price_m2,
        flat_60m2_price,
        avg_monthly_wage,
        months_to_buy,
        mortgage_rate_pct: mortgage_rate,
        monthly_payment_30y: monthly_payment,
        payment_to_wage_pct,
        avg_rent_m2,
        monthly_rent_60m2,
        rent_vs_mortgage_ratio,
        computed_at: None,
        avg_monthly_wage_net,
        months_to_buy_gross,
        avg_living_expenses: Some(expenses),
        monthly_savings,
        years_to_save_investing: years_to_save,
    };

    affordability::upsert(pool, &record).await?;

    tracing::info!(
        "  {region} ({year}): months_to_buy={}",
        months_to_buy.map_or("N/A".to_string(), |m| format!("{m:.1}"))
    );

    Ok(())
}

/// Find the closest time_series value to a target year.
async fn find_closest_value(
    pool: &SqlitePool,
    indicator: &str,
    region: &str,
    year: &str,
) -> Result<Option<f64>> {
    // Try exact year first, then +/- 1 year
    let row: Option<(f64,)> = sqlx::query_as(
        "SELECT value FROM time_series
         WHERE indicator = ? AND region = ? AND date LIKE ?
         ORDER BY date DESC LIMIT 1",
    )
    .bind(indicator)
    .bind(region)
    .bind(format!("{year}%"))
    .fetch_optional(pool)
    .await?;

    if let Some((v,)) = row {
        return Ok(Some(v));
    }

    // Try +/- 1 year
    let year_num: i32 = year[..4].parse().unwrap_or(0);
    for offset in [1, -1, 2, -2] {
        let nearby = format!("{}%", year_num + offset);
        let row: Option<(f64,)> = sqlx::query_as(
            "SELECT value FROM time_series
             WHERE indicator = ? AND region = ? AND date LIKE ?
             ORDER BY date DESC LIMIT 1",
        )
        .bind(indicator)
        .bind(region)
        .bind(&nearby)
        .fetch_optional(pool)
        .await?;
        if let Some((v,)) = row {
            return Ok(Some(v));
        }
    }

    Ok(None)
}

/// Get the current (latest) rent per m2 for a region from time_series.
/// For "national", averages all regional rents (no national entry in time_series).
async fn get_current_rent_m2(pool: &SqlitePool, region: &str) -> Result<Option<f64>> {
    if region == "national" {
        let row: Option<(f64,)> = sqlx::query_as(
            "SELECT AVG(value) FROM time_series
             WHERE indicator = 'avg_rent_m2_flat'
               AND date = (SELECT MAX(date) FROM time_series
                           WHERE indicator = 'avg_rent_m2_flat')",
        )
        .fetch_optional(pool)
        .await?;
        return Ok(row.map(|(v,)| v));
    }
    let row: Option<(f64,)> = sqlx::query_as(
        "SELECT value FROM time_series
         WHERE indicator = 'avg_rent_m2_flat' AND region = ?
         ORDER BY date DESC LIMIT 1",
    )
    .bind(region)
    .fetch_optional(pool)
    .await?;
    Ok(row.map(|(v,)| v))
}

/// Find the price index value closest to a target year.
fn find_closest_index(rows: &[(String, f64)], year: i32) -> Option<f64> {
    let year_str = year.to_string();
    // Exact year match
    if let Some((_, v)) = rows.iter().find(|(d, _)| d.starts_with(&year_str)) {
        return Some(*v);
    }
    // Nearest year
    rows.iter()
        .filter_map(|(d, v)| d[..4].parse::<i32>().ok().map(|y| ((y - year).abs(), *v)))
        .min_by_key(|(dist, _)| *dist)
        .map(|(_, v)| v)
}
