mod compare;
mod index;
mod region;
mod stories;

use axum::Router;
use axum::routing::get;
use sqlx::SqlitePool;

use crate::compute::stories::{SnapshotData, StoryInput};

use super::AppState;
use super::templates::{RegionCard, fmt_value, severity_color, slug_to_display_name};

/// Define all application routes.
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", get(index::index))
        .route("/stories", get(stories::stories_page))
        .route("/region/{slug}", get(region::region))
        .route("/compare", get(compare::compare))
        .route("/api/health", get(health))
}

/// Row for loading story input data from affordability table.
#[derive(sqlx::FromRow)]
pub(super) struct StoryRow {
    pub region: String,
    pub date: String,
    pub months_to_buy: Option<f64>,
    pub avg_price_m2: Option<f64>,
    pub avg_monthly_wage_net: Option<f64>,
    pub monthly_rent_60m2: Option<f64>,
    pub monthly_payment_30y: Option<f64>,
    pub rent_vs_mortgage_ratio: Option<f64>,
    pub years_to_save_investing: Option<f64>,
}

/// Load story inputs from affordability table, optionally filtered by region.
pub(super) async fn load_story_inputs(
    pool: &SqlitePool,
    region_filter: Option<&str>,
) -> Result<Vec<StoryInput>, sqlx::Error> {
    let rows: Vec<StoryRow> = if let Some(region) = region_filter {
        sqlx::query_as(
            "SELECT region, date, months_to_buy, avg_price_m2, avg_monthly_wage_net,
                    monthly_rent_60m2, monthly_payment_30y, rent_vs_mortgage_ratio,
                    years_to_save_investing
             FROM affordability
             WHERE region = ? AND months_to_buy IS NOT NULL
             ORDER BY date ASC",
        )
        .bind(region)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query_as(
            "SELECT region, date, months_to_buy, avg_price_m2, avg_monthly_wage_net,
                    monthly_rent_60m2, monthly_payment_30y, rent_vs_mortgage_ratio,
                    years_to_save_investing
             FROM affordability
             WHERE months_to_buy IS NOT NULL
             ORDER BY region, date ASC",
        )
        .fetch_all(pool)
        .await?
    };

    // Group by region
    let mut map: std::collections::BTreeMap<String, Vec<SnapshotData>> =
        std::collections::BTreeMap::new();
    for row in &rows {
        let year: i32 = row.date[..4].parse().unwrap_or(0);
        map.entry(row.region.clone()).or_default().push(SnapshotData {
            year,
            months_to_buy: row.months_to_buy,
            avg_price_m2: row.avg_price_m2,
            avg_monthly_wage_net: row.avg_monthly_wage_net,
            monthly_rent_60m2: row.monthly_rent_60m2,
            monthly_payment_30y: row.monthly_payment_30y,
            rent_vs_mortgage_ratio: row.rent_vs_mortgage_ratio,
            years_to_save_investing: row.years_to_save_investing,
        });
    }

    Ok(map
        .into_iter()
        .map(|(region, snapshots)| StoryInput {
            region_name: slug_to_display_name(&region),
            region,
            snapshots,
        })
        .collect())
}

/// Region data fetched from the affordability table.
#[derive(sqlx::FromRow)]
pub(super) struct AffordabilityRow {
    pub region: String,
    pub months_to_buy: Option<f64>,
    pub avg_price_m2: Option<f64>,
    pub flat_60m2_price: Option<f64>,
    pub avg_monthly_wage: Option<f64>,
    pub mortgage_rate_pct: Option<f64>,
    pub monthly_payment_30y: Option<f64>,
    pub payment_to_wage_pct: Option<f64>,
    pub avg_rent_m2: Option<f64>,
    pub monthly_rent_60m2: Option<f64>,
    pub rent_vs_mortgage_ratio: Option<f64>,
    pub avg_monthly_wage_net: Option<f64>,
    pub months_to_buy_gross: Option<f64>,
    pub avg_living_expenses: Option<f64>,
    pub monthly_savings: Option<f64>,
    pub years_to_save_investing: Option<f64>,
}

pub(super) fn row_to_card(row: &AffordabilityRow) -> RegionCard {
    let months = row.months_to_buy.unwrap_or(0.0);
    RegionCard {
        slug: row.region.clone(),
        name: slug_to_display_name(&row.region),
        years: row.months_to_buy.map_or("N/A".to_string(), |m| format!("{:.1}", m / 12.0)),
        years_raw: months / 12.0,
        price_m2: fmt_value(row.avg_price_m2, " CZK"),
        wage: fmt_value(row.avg_monthly_wage, " CZK"),
        color_class: severity_color(months),
    }
}

/// Health check endpoint.
async fn health() -> &'static str {
    "ok"
}
