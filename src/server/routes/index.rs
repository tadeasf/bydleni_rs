use axum::extract::State;
use axum::response::IntoResponse;

use crate::compute::stories::{derive_stories, top_stories};
use crate::server::AppState;
use crate::server::error::AppError;
use crate::server::methodology;
use crate::server::templates::{HtmlTemplate, IndexTemplate, RegionCard, story_to_view};

use super::{AffordabilityRow, load_story_inputs, row_to_card};

/// Landing page handler.
pub(super) async fn index(State(state): State<AppState>) -> Result<impl IntoResponse, AppError> {
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

    let regions_json = serde_json::to_string(&regions).unwrap_or_else(|_| "[]".to_string());

    // Derive top 3 stories across all regions
    let story_inputs = load_story_inputs(&state.pool, None).await.unwrap_or_default();
    let all_stories: Vec<_> = story_inputs.iter().flat_map(derive_stories).collect();
    let top = top_stories(&all_stories, 3);
    let top_story_views = top.iter().map(story_to_view).collect();

    let method = methodology::build_index_methodology(&last_refresh);
    let tpl = IndexTemplate {
        regions,
        regions_json,
        refreshing,
        last_refresh,
        explain_years: method.years_to_buy,
        explain_forecast: method.forecast,
        explain_rent_burden: method.rent_burden,
        top_stories: top_story_views,
    };
    Ok(HtmlTemplate(tpl))
}
