use axum::extract::State;
use axum::response::Json;

use crate::compute::forecast::{self, AffordabilitySnapshot, ForecastResult};
use crate::server::AppState;
use crate::server::error::AppError;

/// Query row for forecast endpoints.
#[derive(sqlx::FromRow)]
struct ForecastRow {
    date: String,
    avg_price_m2: Option<f64>,
    avg_monthly_wage_net: Option<f64>,
    months_to_buy: Option<f64>,
    monthly_rent_60m2: Option<f64>,
}

impl ForecastRow {
    fn to_snapshot(&self) -> Option<AffordabilitySnapshot> {
        let year: i32 = self.date[..4].parse().ok()?;
        Some(AffordabilitySnapshot {
            year,
            avg_price_m2: self.avg_price_m2?,
            avg_monthly_wage_net: self.avg_monthly_wage_net?,
            months_to_buy: self.months_to_buy?,
            monthly_rent_60m2: self.monthly_rent_60m2,
        })
    }

    fn to_rent_snapshot(&self) -> Option<AffordabilitySnapshot> {
        let year: i32 = self.date[..4].parse().ok()?;
        Some(AffordabilitySnapshot {
            year,
            avg_price_m2: self.avg_price_m2.unwrap_or(0.0),
            avg_monthly_wage_net: self.avg_monthly_wage_net?,
            months_to_buy: self.months_to_buy.unwrap_or(0.0),
            monthly_rent_60m2: self.monthly_rent_60m2,
        })
    }
}

/// Forecast chart: national affordability trends + 5-year projection.
pub(super) async fn chart_forecast(
    State(state): State<AppState>,
) -> Result<Json<ForecastResult>, AppError> {
    let rows: Vec<ForecastRow> = sqlx::query_as(
        "SELECT date, avg_price_m2, avg_monthly_wage_net, months_to_buy, monthly_rent_60m2
         FROM affordability
         WHERE region = 'national' AND months_to_buy IS NOT NULL
           AND avg_price_m2 IS NOT NULL AND avg_monthly_wage_net IS NOT NULL
         ORDER BY date ASC",
    )
    .fetch_all(&state.pool)
    .await?;

    let snapshots: Vec<AffordabilitySnapshot> =
        rows.iter().filter_map(ForecastRow::to_snapshot).collect();

    match forecast::build_affordability_forecast(&snapshots, 5) {
        Some(result) => Ok(Json(result)),
        None => Ok(Json(ForecastResult { labels: vec![], datasets: vec![] })),
    }
}

/// Rent burden chart: % of net salary spent on rent, with projection.
pub(super) async fn chart_rent_burden(
    State(state): State<AppState>,
) -> Result<Json<ForecastResult>, AppError> {
    let rows: Vec<ForecastRow> = sqlx::query_as(
        "SELECT date, avg_price_m2, avg_monthly_wage_net, months_to_buy, monthly_rent_60m2
         FROM affordability
         WHERE region = 'national' AND avg_monthly_wage_net IS NOT NULL
           AND monthly_rent_60m2 IS NOT NULL
         ORDER BY date ASC",
    )
    .fetch_all(&state.pool)
    .await?;

    let snapshots: Vec<AffordabilitySnapshot> =
        rows.iter().filter_map(ForecastRow::to_rent_snapshot).collect();

    match forecast::build_rent_burden_forecast(&snapshots, 5) {
        Some(result) => Ok(Json(result)),
        None => Ok(Json(ForecastResult { labels: vec![], datasets: vec![] })),
    }
}
