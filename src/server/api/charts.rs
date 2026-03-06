use axum::extract::{Query, State};
use axum::response::Json;
use serde::{Deserialize, Serialize};

use crate::server::AppState;
use crate::server::error::AppError;

/// Chart.js-compatible response.
#[derive(Serialize)]
pub(super) struct ChartData {
    labels: Vec<String>,
    datasets: Vec<ChartDataset>,
}

#[derive(Serialize)]
pub(super) struct ChartDataset {
    label: String,
    data: Vec<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "backgroundColor")]
    background_color: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "borderColor")]
    border_color: Option<String>,
}

#[derive(Deserialize)]
pub(super) struct RegionQuery {
    region: Option<String>,
}

pub(super) fn display_name(slug: &str) -> String {
    crate::server::templates::slug_to_display_name(slug)
}

pub(super) fn severity_hex(months: f64) -> String {
    if months > 160.0 {
        "#e8524a".to_string()
    } else if months > 130.0 {
        "#f2845c".to_string()
    } else if months > 110.0 {
        "#f2b854".to_string()
    } else if months > 100.0 {
        "#7ec4a0".to_string()
    } else {
        "#52b788".to_string()
    }
}

/// Bar chart: months-to-buy per region.
pub(super) async fn chart_affordability(
    State(state): State<AppState>,
    Query(_q): Query<RegionQuery>,
) -> Result<Json<ChartData>, AppError> {
    let rows: Vec<(String, f64)> = sqlx::query_as(
        "SELECT region, months_to_buy FROM affordability
         WHERE months_to_buy IS NOT NULL AND region != 'national'
           AND date = (SELECT MAX(date) FROM affordability AS a2
                       WHERE a2.region = affordability.region)
         ORDER BY months_to_buy DESC",
    )
    .fetch_all(&state.pool)
    .await?;

    let labels: Vec<String> = rows.iter().map(|(r, _)| display_name(r)).collect();
    let data: Vec<f64> = rows.iter().map(|(_, m)| *m / 12.0).collect();
    let colors: Vec<String> = rows.iter().map(|(_, m)| severity_hex(*m)).collect();

    Ok(Json(ChartData {
        labels,
        datasets: vec![ChartDataset {
            label: "Years of net salary".to_string(),
            data,
            background_color: Some(colors),
            border_color: None,
        }],
    }))
}

/// Line chart: price/m2 time series for a region.
pub(super) async fn chart_prices(
    State(state): State<AppState>,
    Query(q): Query<RegionQuery>,
) -> Result<Json<ChartData>, AppError> {
    let region = q.region.as_deref().unwrap_or("praha");

    let rows: Vec<(String, f64)> = sqlx::query_as(
        "SELECT date, value FROM time_series
         WHERE indicator IN ('avg_asking_price_m2_flat', 'avg_price_m2_flat')
           AND region = ?
         ORDER BY date ASC",
    )
    .bind(region)
    .fetch_all(&state.pool)
    .await?;

    let labels: Vec<String> = rows.iter().map(|(d, _)| d.clone()).collect();
    let data: Vec<f64> = rows.iter().map(|(_, v)| *v).collect();

    Ok(Json(ChartData {
        labels,
        datasets: vec![ChartDataset {
            label: "Price/m\u{b2} (CZK)".to_string(),
            data,
            background_color: None,
            border_color: Some("#3b82f6".to_string()),
        }],
    }))
}

/// Grouped bar: monthly rent vs mortgage for a region.
pub(super) async fn chart_rent_vs_buy(
    State(state): State<AppState>,
    Query(q): Query<RegionQuery>,
) -> Result<Json<ChartData>, AppError> {
    let region = q.region.as_deref().unwrap_or("praha");

    let row: Option<(Option<f64>, Option<f64>)> = sqlx::query_as(
        "SELECT monthly_rent_60m2, monthly_payment_30y FROM affordability
         WHERE region = ? ORDER BY date DESC LIMIT 1",
    )
    .bind(region)
    .fetch_optional(&state.pool)
    .await?;

    let (rent, mortgage) = row.unwrap_or((None, None));

    Ok(Json(ChartData {
        labels: vec!["Rent (60m\u{b2})".to_string(), "Mortgage (30y)".to_string()],
        datasets: vec![ChartDataset {
            label: display_name(region),
            data: vec![rent.unwrap_or(0.0), mortgage.unwrap_or(0.0)],
            background_color: Some(vec!["#8b5cf6".to_string(), "#3b82f6".to_string()]),
            border_color: None,
        }],
    }))
}

/// Line chart: repo rate history (national).
pub(super) async fn chart_mortgage(
    State(state): State<AppState>,
) -> Result<Json<ChartData>, AppError> {
    let rows: Vec<(String, f64)> = sqlx::query_as(
        "SELECT date, value FROM time_series
         WHERE indicator = 'repo_rate_2w' AND region = 'national'
         ORDER BY date ASC",
    )
    .fetch_all(&state.pool)
    .await?;

    let labels: Vec<String> = rows.iter().map(|(d, _)| d.clone()).collect();
    let data: Vec<f64> = rows.iter().map(|(_, v)| *v).collect();

    Ok(Json(ChartData {
        labels,
        datasets: vec![ChartDataset {
            label: "CNB repo rate (%)".to_string(),
            data,
            background_color: None,
            border_color: Some("#ef4444".to_string()),
        }],
    }))
}

/// Bar chart: wages per region.
pub(super) async fn chart_wages(
    State(state): State<AppState>,
    Query(_q): Query<RegionQuery>,
) -> Result<Json<ChartData>, AppError> {
    let rows: Vec<(String, f64)> = sqlx::query_as(
        "SELECT region, avg_monthly_wage FROM affordability
         WHERE avg_monthly_wage IS NOT NULL AND region != 'national'
           AND date = (SELECT MAX(date) FROM affordability AS a2
                       WHERE a2.region = affordability.region)
         ORDER BY avg_monthly_wage DESC",
    )
    .fetch_all(&state.pool)
    .await?;

    let labels: Vec<String> = rows.iter().map(|(r, _)| display_name(r)).collect();
    let data: Vec<f64> = rows.iter().map(|(_, w)| *w).collect();
    let count = labels.len();

    Ok(Json(ChartData {
        labels,
        datasets: vec![ChartDataset {
            label: "Avg monthly wage (CZK)".to_string(),
            data,
            background_color: Some(vec!["#10b981".to_string(); count]),
            border_color: None,
        }],
    }))
}

/// Line chart: historical months-to-buy for a region across snapshot years.
pub(super) async fn chart_history(
    State(state): State<AppState>,
    Query(q): Query<RegionQuery>,
) -> Result<Json<ChartData>, AppError> {
    let region = q.region.as_deref().unwrap_or("praha");

    let rows: Vec<(String, f64)> = sqlx::query_as(
        "SELECT date, months_to_buy FROM affordability
         WHERE region = ? AND months_to_buy IS NOT NULL
         ORDER BY date ASC",
    )
    .bind(region)
    .fetch_all(&state.pool)
    .await?;

    // Extract just the year for labels
    let labels: Vec<String> = rows.iter().map(|(d, _)| d[..4].to_string()).collect();
    let data: Vec<f64> = rows.iter().map(|(_, m)| *m).collect();

    Ok(Json(ChartData {
        labels,
        datasets: vec![ChartDataset {
            label: "Months of net salary".to_string(),
            data,
            background_color: None,
            border_color: Some("#f2845c".to_string()),
        }],
    }))
}
