mod charts;
mod forecast;
mod htmx_handlers;
mod scenario;
mod status;

use axum::Router;
use axum::routing::{get, post};

use super::AppState;

/// Register all API routes.
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/chart/affordability", get(charts::chart_affordability))
        .route("/api/chart/prices", get(charts::chart_prices))
        .route("/api/chart/rent-vs-buy", get(charts::chart_rent_vs_buy))
        .route("/api/chart/mortgage", get(charts::chart_mortgage))
        .route("/api/chart/wages", get(charts::chart_wages))
        .route("/api/chart/history", get(charts::chart_history))
        .route("/api/chart/forecast", get(forecast::chart_forecast))
        .route("/api/chart/rent-burden", get(forecast::chart_rent_burden))
        .route("/api/mortgage-calc", post(htmx_handlers::mortgage_calc))
        .route("/api/recalc-savings", post(htmx_handlers::recalc_savings))
        .route("/api/scenario/summary", post(scenario::scenario_summary_handler))
        .route("/api/scenario/regions", post(scenario::scenario_regions))
        .route("/api/scenario/region-detail", post(scenario::scenario_region_detail))
        .route("/api/scenario/compare", post(scenario::scenario_compare))
        .route("/api/status", get(status::status))
}
