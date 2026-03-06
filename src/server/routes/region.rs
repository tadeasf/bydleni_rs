use axum::extract::{Path, State};
use axum::response::IntoResponse;

use crate::compute::stories::derive_stories;
use crate::models::listing::{self, ExampleListing};
use crate::server::AppState;
use crate::server::error::AppError;
use crate::server::methodology;
use crate::server::templates::{
    HtmlTemplate, ListingView, NotFoundTemplate, RegionTemplate, fmt_pct, fmt_ratio, fmt_thousands,
    fmt_value, fmt_years, slug_to_display_name, story_to_view,
};

use super::{AffordabilityRow, load_story_inputs};

/// Per-region deep dive page.
pub(super) async fn region(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let row: Option<AffordabilityRow> = sqlx::query_as(
        "SELECT region, months_to_buy, avg_price_m2, flat_60m2_price, avg_monthly_wage,
                mortgage_rate_pct, monthly_payment_30y, payment_to_wage_pct,
                avg_rent_m2, monthly_rent_60m2, rent_vs_mortgage_ratio,
                avg_monthly_wage_net, months_to_buy_gross,
                avg_living_expenses, monthly_savings, years_to_save_investing
         FROM affordability WHERE region = ?
         ORDER BY date DESC LIMIT 1",
    )
    .bind(&slug)
    .fetch_optional(&state.pool)
    .await?;

    match row {
        Some(data) => {
            let refreshing = state.refreshing.load(std::sync::atomic::Ordering::Relaxed);
            let last_refresh = state.last_refresh.read().await.clone().unwrap_or_default();
            let method = methodology::build_region_methodology(&last_refresh);

            // Fetch example listings
            let sale_raw =
                listing::query_by_region(&state.pool, &slug, "sale", 3).await.unwrap_or_default();
            let sale_median_price_m2 = median_price_per_m2(&sale_raw);
            let sale_listings = sale_raw
                .iter()
                .map(|l| ListingView {
                    name: l.name.clone(),
                    price: fmt_value(Some(l.price as f64), " CZK"),
                    area: l.area_m2.map_or("N/A".to_string(), |a| format!("{a:.0} m\u{b2}")),
                    price_per_m2: l
                        .price_per_m2
                        .map_or("N/A".to_string(), |p| format!("{p:.0} CZK/m\u{b2}")),
                    url: l.url.clone(),
                })
                .collect();

            let rent_raw =
                listing::query_by_region(&state.pool, &slug, "rent", 3).await.unwrap_or_default();
            let rent_median_price_m2 = median_price_per_m2(&rent_raw);
            let rent_listings = rent_raw
                .iter()
                .map(|l| ListingView {
                    name: l.name.clone(),
                    price: fmt_value(Some(l.price as f64), " CZK/mo"),
                    area: l.area_m2.map_or("N/A".to_string(), |a| format!("{a:.0} m\u{b2}")),
                    price_per_m2: l
                        .price_per_m2
                        .map_or("N/A".to_string(), |p| format!("{p:.0} CZK/m\u{b2}")),
                    url: l.url.clone(),
                })
                .collect();

            // Derive region stories
            let story_inputs =
                load_story_inputs(&state.pool, Some(&slug)).await.unwrap_or_default();
            let story_views: Vec<_> = story_inputs
                .iter()
                .flat_map(derive_stories)
                .take(4)
                .map(|s| story_to_view(&s))
                .collect();

            let tpl = RegionTemplate {
                slug: data.region.clone(),
                name: slug_to_display_name(&data.region),
                months_to_buy: fmt_value(data.months_to_buy, ""),
                months_to_buy_gross: fmt_value(data.months_to_buy_gross, ""),
                price_m2: fmt_value(data.avg_price_m2, " CZK"),
                flat_price: fmt_value(data.flat_60m2_price, " CZK"),
                wage: fmt_value(data.avg_monthly_wage, " CZK"),
                wage_net: fmt_value(data.avg_monthly_wage_net, " CZK"),
                mortgage_rate: fmt_pct(data.mortgage_rate_pct),
                monthly_payment: fmt_value(data.monthly_payment_30y, " CZK"),
                payment_to_wage: fmt_pct(data.payment_to_wage_pct),
                rent_m2: fmt_value(data.avg_rent_m2, " CZK"),
                monthly_rent: fmt_value(data.monthly_rent_60m2, " CZK"),
                rent_vs_mortgage: fmt_ratio(data.rent_vs_mortgage_ratio),
                living_expenses: fmt_value(data.avg_living_expenses, " CZK"),
                living_expenses_raw: data.avg_living_expenses.unwrap_or(17_000.0),
                monthly_savings: fmt_value(data.monthly_savings, " CZK"),
                years_to_save: fmt_years(data.years_to_save_investing),
                wage_net_raw: data.avg_monthly_wage_net.unwrap_or(0.0),
                flat_price_raw: data.flat_60m2_price.unwrap_or(0.0),
                mortgage_rate_raw: data.mortgage_rate_pct.unwrap_or(0.0),
                refreshing,
                stories: story_views,
                sale_listings,
                rent_listings,
                sale_median_price_m2,
                rent_median_price_m2,
                explain_months: method.months_to_buy,
                explain_price: method.price_m2,
                explain_mortgage: method.mortgage,
                explain_rent: method.rent_vs_mortgage,
                explain_savings: method.savings,
            };
            Ok(HtmlTemplate(tpl).into_response())
        }
        None => {
            let tpl = NotFoundTemplate { slug };
            Ok(HtmlTemplate(tpl).into_response())
        }
    }
}

/// Compute median price per m2 from a list of listings, formatted as string.
fn median_price_per_m2(listings: &[ExampleListing]) -> String {
    let mut prices: Vec<f64> = listings.iter().filter_map(|l| l.price_per_m2).collect();
    if prices.is_empty() {
        return String::new();
    }
    prices.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let median = prices[prices.len() / 2];
    fmt_thousands(median)
}
