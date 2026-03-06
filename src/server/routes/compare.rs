use axum::extract::State;
use axum::response::IntoResponse;

use crate::server::AppState;
use crate::server::error::AppError;
use crate::server::methodology;
use crate::server::templates::{CompareTemplate, HtmlTemplate, RegionCard};

use super::{AffordabilityRow, row_to_card};

/// Compare page handler.
pub(super) async fn compare(State(state): State<AppState>) -> Result<impl IntoResponse, AppError> {
    let rows: Vec<AffordabilityRow> = sqlx::query_as(
        "SELECT region, months_to_buy, avg_price_m2, flat_60m2_price, avg_monthly_wage,
                mortgage_rate_pct, monthly_payment_30y, payment_to_wage_pct,
                avg_rent_m2, monthly_rent_60m2, rent_vs_mortgage_ratio,
                avg_monthly_wage_net, months_to_buy_gross,
                avg_living_expenses, monthly_savings, years_to_save_investing
         FROM affordability
         WHERE months_to_buy IS NOT NULL AND region != 'national'
           AND date = (SELECT MAX(date) FROM affordability AS a2
                       WHERE a2.region = affordability.region)
         ORDER BY months_to_buy DESC",
    )
    .fetch_all(&state.pool)
    .await?;

    let regions: Vec<RegionCard> = rows.iter().map(row_to_card).collect();
    let refreshing = state.refreshing.load(std::sync::atomic::Ordering::Relaxed);
    let last_refresh = state.last_refresh.read().await.clone().unwrap_or_default();
    let method = methodology::build_compare_methodology(&last_refresh);

    let tpl = CompareTemplate { regions, refreshing, explain_comparison: method.to_html() };
    Ok(HtmlTemplate(tpl))
}
