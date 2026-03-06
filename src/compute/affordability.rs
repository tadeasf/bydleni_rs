use anyhow::Result;
use sqlx::SqlitePool;

use crate::compute::aggregation;
use crate::compute::czech_tax;
use crate::models::affordability::{self, Affordability};

/// Map city slugs (from Sreality) to CZSO kraj slugs (for wage lookup).
/// Praha maps to itself since CZSO uses "praha" for "Hlavní město Praha".
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

/// All regions we compute affordability for.
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

/// Compute and materialize affordability metrics for all regions.
pub async fn compute_all(pool: &SqlitePool) -> Result<()> {
    tracing::info!("Computing affordability metrics for all regions...");

    for &region in REGIONS {
        if let Err(e) = compute_region(pool, region).await {
            tracing::error!("Failed to compute affordability for {region}: {e:#}");
        }
    }

    tracing::info!("Affordability computation complete");
    Ok(())
}

/// Compute affordability metrics for a single region.
///
/// Joins the latest available data from multiple sources:
/// - CZSO/Sreality: avg price per m2
/// - CZSO: avg monthly wage
/// - CNB: mortgage rate
/// - Sreality: avg rent per m2
async fn compute_region(pool: &SqlitePool, region: &str) -> Result<()> {
    // Get latest price per m2 and rent per m2
    let (avg_price_m2, avg_rent_m2) = if region == "national" {
        // National: average all regional values
        let price = get_national_average(pool, "avg_asking_price_m2_flat").await?;
        let rent = get_national_average(pool, "avg_rent_m2_flat").await?;
        (price, rent)
    } else {
        let price = get_latest_value(pool, "avg_asking_price_m2_flat", region)
            .await?
            .or(get_latest_value(pool, "avg_price_m2_flat", region).await?);
        let rent = get_latest_value(pool, "avg_rent_m2_flat", region).await?;
        (price, rent)
    };

    // Get latest wage: try kraj slug first, then region directly, then national fallback
    let wage_region = city_to_kraj(region).unwrap_or(region);
    let avg_monthly_wage = get_latest_value(pool, "avg_monthly_wage", wage_region)
        .await?
        .or(get_latest_value(pool, "avg_monthly_wage", region).await?)
        .or(get_latest_value(pool, "avg_monthly_wage", "national").await?);

    // Get latest mortgage rate: prefer real CNB MFI data, fallback to repo + spread
    let mortgage_rate = match get_latest_value(pool, "mortgage_rate_avg", "national").await? {
        Some(rate) => Some(rate),
        None => get_latest_value(pool, "repo_rate_2w", "national").await?.map(|repo| repo + 2.5),
    };

    // Derive metrics
    let flat_60m2_price = avg_price_m2.map(|p| p * 60.0);

    // Net wage via Czech tax formula
    let avg_monthly_wage_net = avg_monthly_wage.map(czech_tax::gross_to_net_monthly);

    // Primary metric: months of NET salary to buy
    let months_to_buy = match (flat_60m2_price, avg_monthly_wage_net) {
        (Some(price), Some(wage)) if wage > 0.0 => Some(price / wage),
        _ => None,
    };

    // Secondary: months of GROSS salary (for comparison)
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

    // Living expenses and savings
    let expenses = living_expenses(region);
    let monthly_savings = avg_monthly_wage_net.map(|net| (net - expenses).max(0.0));
    let years_to_save_investing = match (flat_60m2_price, monthly_savings) {
        (Some(target), Some(savings)) if savings > 0.0 => {
            Some(years_to_save_with_investment(target, savings, 7.0))
        }
        _ => None,
    };

    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();

    let record = Affordability {
        id: None,
        region: region.to_string(),
        date: today,
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
        years_to_save_investing,
    };

    affordability::upsert(pool, &record).await?;

    if let Some(mtb) = months_to_buy {
        tracing::info!(
            "  {region}: {mtb:.1} months (net) to buy 60m2 flat (price: {}, net wage: {})",
            flat_60m2_price.map_or("N/A".to_string(), |p| format!("{p:.0} CZK")),
            avg_monthly_wage_net.map_or("N/A".to_string(), |w| format!("{w:.0} CZK")),
        );
    } else {
        tracing::info!("  {region}: insufficient data to compute months_to_buy");
    }

    Ok(())
}

/// Get the latest value for an indicator+region from time_series.
async fn get_latest_value(pool: &SqlitePool, indicator: &str, region: &str) -> Result<Option<f64>> {
    let row: Option<(f64,)> = sqlx::query_as(
        "SELECT value FROM time_series
         WHERE indicator = ? AND region = ?
         ORDER BY date DESC LIMIT 1",
    )
    .bind(indicator)
    .bind(region)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|(v,)| v))
}

/// Get national average by averaging all regional values at the latest date.
async fn get_national_average(pool: &SqlitePool, indicator: &str) -> Result<Option<f64>> {
    let date_row: Option<(String,)> = sqlx::query_as(
        "SELECT MAX(date) FROM time_series
         WHERE indicator = ? AND region != 'national'",
    )
    .bind(indicator)
    .fetch_optional(pool)
    .await?;

    if let Some((date,)) = date_row {
        aggregation::national_average(pool, indicator, &date).await
    } else {
        Ok(None)
    }
}

/// Regional living expenses (CZK/month, single person, excluding rent).
/// Based on CZSO consumer basket data (spotrebitelsky kos).
pub fn living_expenses(region: &str) -> f64 {
    match region {
        "praha" => 22_000.0,
        "brno" => 18_500.0,
        "ostrava" => 16_500.0,
        "plzen" => 17_500.0,
        "liberec" => 16_500.0,
        "olomouc" => 16_000.0,
        "hradec_kralove" => 16_500.0,
        "ceske_budejovice" => 16_500.0,
        "usti_nad_labem" => 16_000.0,
        "pardubice" => 16_000.0,
        "zlin" => 16_000.0,
        "karlovy_vary" => 16_500.0,
        "jihlava" => 16_000.0,
        "stredocesky" => 17_000.0,
        "national" => 17_000.0,
        _ => 17_000.0,
    }
}

/// Calculate years to save a target amount by investing monthly at a given annual return.
///
/// Uses the future value of annuity formula solved for n:
/// n = ln(1 + target * r / pmt) / ln(1 + r)
/// where r = monthly rate, pmt = monthly savings.
pub fn years_to_save_with_investment(
    target: f64,
    monthly_savings: f64,
    annual_return_pct: f64,
) -> f64 {
    if monthly_savings <= 0.0 || target <= 0.0 {
        return f64::INFINITY;
    }

    let monthly_rate = annual_return_pct / 100.0 / 12.0;

    if monthly_rate <= 0.0 {
        // No returns: simple division
        return target / monthly_savings / 12.0;
    }

    let n_months = (1.0 + target * monthly_rate / monthly_savings).ln() / (1.0 + monthly_rate).ln();
    n_months / 12.0
}

/// Calculate monthly mortgage payment using standard amortization formula.
///
/// P = principal * (r * (1+r)^n) / ((1+r)^n - 1)
///
/// # Arguments
/// * `principal` - Loan amount in CZK
/// * `annual_rate_pct` - Annual interest rate in percent (e.g. 5.5 for 5.5%)
/// * `months` - Number of monthly payments (e.g. 360 for 30 years)
pub fn mortgage_monthly_payment(principal: f64, annual_rate_pct: f64, months: u32) -> f64 {
    let monthly_rate = annual_rate_pct / 100.0 / 12.0;
    if monthly_rate <= 0.0 {
        return principal / months as f64;
    }
    let n = months as f64;
    let factor = (1.0 + monthly_rate).powf(n);
    principal * (monthly_rate * factor) / (factor - 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mortgage_monthly_payment() {
        // 3,000,000 CZK loan at 5.5% for 30 years
        let payment = mortgage_monthly_payment(3_000_000.0, 5.5, 360);
        // Expected ~17,033 CZK/month
        assert!((payment - 17_033.0).abs() < 50.0, "got {payment}");
    }

    #[test]
    fn test_mortgage_zero_rate() {
        let payment = mortgage_monthly_payment(360_000.0, 0.0, 360);
        assert!((payment - 1000.0).abs() < 0.01);
    }

    #[test]
    fn test_years_to_save_typical() {
        // Save 5M CZK with 15k/month at 7% -> should be ~14-16 years
        let years = years_to_save_with_investment(5_000_000.0, 15_000.0, 7.0);
        assert!((12.0..=18.0).contains(&years), "expected ~14-16 years, got {years:.1}");
    }

    #[test]
    fn test_years_to_save_zero_return() {
        // Without investment returns: 5M / (15k * 12) = 27.8 years
        let years = years_to_save_with_investment(5_000_000.0, 15_000.0, 0.0);
        assert!((years - 27.78).abs() < 0.1, "expected ~27.78 years, got {years:.2}");
    }

    #[test]
    fn test_years_to_save_zero_savings() {
        let years = years_to_save_with_investment(5_000_000.0, 0.0, 7.0);
        assert!(years.is_infinite());
    }

    #[test]
    fn test_living_expenses_ranges() {
        assert!(living_expenses("praha") > living_expenses("brno"));
        assert!(living_expenses("brno") > living_expenses("ostrava"));
        assert!(living_expenses("national") > 15_000.0);
    }
}
