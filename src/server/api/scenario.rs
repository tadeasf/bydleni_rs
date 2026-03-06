use axum::extract::State;
use axum::response::{Html, IntoResponse};

use crate::compute::scenario::{
    ScenarioInput, compute_scenario_all_regions, compute_scenario_for_region, scenario_summary,
};
use crate::server::AppState;
use crate::server::templates::fmt_thousands;

/// POST /api/scenario/summary — returns a summary strip HTML fragment.
pub(super) async fn scenario_summary_handler(
    axum::Form(input): axum::Form<ScenarioInput>,
) -> impl IntoResponse {
    if let Err(errors) = input.validate() {
        let msgs = errors.join("; ");
        return Html(format!(r#"<div class="scenario-errors">{msgs}</div>"#));
    }
    let summary = scenario_summary(&input);
    Html(format!(
        r#"<div class="scenario-summary"><span class="scenario-summary-label">Active scenario:</span> <span class="mono">{summary}</span></div>"#
    ))
}

/// POST /api/scenario/regions — returns personalized region cards.
pub(super) async fn scenario_regions(
    State(state): State<AppState>,
    axum::Form(input): axum::Form<ScenarioInput>,
) -> impl IntoResponse {
    if let Err(errors) = input.validate() {
        let msgs = errors.join("; ");
        return Html(format!(r#"<div class="scenario-errors">{msgs}</div>"#));
    }

    let rows: Vec<(String, Option<f64>)> = sqlx::query_as(
        "SELECT region, avg_price_m2 FROM affordability
         WHERE avg_price_m2 IS NOT NULL AND region != 'national'
           AND date = (SELECT MAX(date) FROM affordability AS a2
                       WHERE a2.region = affordability.region)",
    )
    .fetch_all(&state.pool)
    .await
    .unwrap_or_default();

    let region_prices: Vec<(String, f64)> =
        rows.into_iter().filter_map(|(slug, price)| price.map(|p| (slug, p))).collect();

    let results = compute_scenario_all_regions(&input, &region_prices);

    let mut html = String::from(r#"<div class="scenario-results-strip">"#);
    for (i, r) in results.iter().enumerate() {
        let flat_s = fmt_thousands(r.flat_price);
        let payment_s = fmt_thousands(r.monthly_payment);
        let deposit_gap_s = fmt_thousands(r.deposit_gap);
        let years_s = match r.years_to_deposit {
            Some(y) if y < 0.01 => "Ready!".to_string(),
            Some(y) => format!("{y:.1} yr"),
            None => "N/A".to_string(),
        };
        let pti_s = format!("{:.0}%", r.payment_to_income_pct);
        let best = if i == 0 { " best-fit" } else { "" };
        let aff_label = if r.affordable { "Affordable" } else { "Stretched" };
        let aff_icon = if r.affordable { "&#10003;" } else { "&#10007;" };

        html.push_str(&format!(
            r#"<a href="/region/{slug}" class="scenario-card{best}">
                <div class="scenario-card-head">
                    <span class="scenario-card-name">{name}</span>
                    <span class="scenario-card-aff {color}">{aff_icon} {aff_label}</span>
                </div>
                <div class="scenario-card-price mono">{flat_s} CZK</div>
                <div class="scenario-card-grid">
                    <div>
                        <div class="scenario-card-label">Monthly payment</div>
                        <div class="scenario-card-val mono">{payment_s}</div>
                    </div>
                    <div>
                        <div class="scenario-card-label">Payment / income</div>
                        <div class="scenario-card-val mono {color}">{pti_s}</div>
                    </div>
                    <div>
                        <div class="scenario-card-label">Deposit gap</div>
                        <div class="scenario-card-val mono">{deposit_gap_s}</div>
                    </div>
                    <div>
                        <div class="scenario-card-label">Time to deposit</div>
                        <div class="scenario-card-val mono {color}">{years_s}</div>
                    </div>
                </div>
            </a>"#,
            slug = r.region,
            name = r.region_name,
            color = r.color_class,
        ));
    }
    html.push_str("</div>");
    Html(html)
}

/// POST /api/scenario/region-detail — personalized hero stats for a single region.
pub(super) async fn scenario_region_detail(
    State(state): State<AppState>,
    axum::Form(form): axum::Form<ScenarioRegionForm>,
) -> impl IntoResponse {
    if let Err(errors) = form.input.validate() {
        let msgs = errors.join("; ");
        return Html(format!(r#"<div class="scenario-errors">{msgs}</div>"#));
    }

    let row: Option<(Option<f64>,)> = sqlx::query_as(
        "SELECT avg_price_m2 FROM affordability
         WHERE region = ? AND avg_price_m2 IS NOT NULL
         ORDER BY date DESC LIMIT 1",
    )
    .bind(&form.region)
    .fetch_optional(&state.pool)
    .await
    .unwrap_or(None);

    let avg_price_m2 = match row.and_then(|r| r.0) {
        Some(p) => p,
        None => {
            return Html(
                r#"<div class="scenario-errors">No price data for this region</div>"#.to_string(),
            );
        }
    };

    let r = compute_scenario_for_region(&form.input, &form.region, avg_price_m2);
    let flat_s = fmt_thousands(r.flat_price);
    let payment_s = fmt_thousands(r.monthly_payment);
    let deposit_s = fmt_thousands(r.deposit_needed);
    let gap_s = fmt_thousands(r.deposit_gap);
    let surplus_s = fmt_thousands(r.monthly_surplus);
    let years_s = match r.years_to_deposit {
        Some(y) if y < 0.01 => "Ready!".to_string(),
        Some(y) => format!("{y:.1}"),
        None => "N/A".to_string(),
    };
    let pti_s = format!("{:.0}", r.payment_to_income_pct);

    Html(format!(
        r#"<div class="chart-label">Your scenario</div>
        <div class="hero-metrics">
            <div class="mcard hero-stat">
                <div class="mcard-label">Your {size:.0}m&sup2; flat</div>
                <div class="mcard-val">{flat_s} CZK</div>
            </div>
            <div class="mcard hero-stat">
                <div class="mcard-label">Monthly payment</div>
                <div class="mcard-val">{payment_s} CZK</div>
                <div class="mcard-note">{pti_s}% of your income</div>
            </div>
            <div class="mcard hero-stat">
                <div class="mcard-label">Years to deposit</div>
                <div class="mcard-val {color}">{years_s}</div>
            </div>
        </div>
        <div class="calc-grid" style="margin-bottom:16px">
            <div class="calc-card">
                <div class="mcard-label">Deposit needed</div>
                <div class="mcard-val">{deposit_s} CZK</div>
            </div>
            <div class="calc-card">
                <div class="mcard-label">Deposit gap</div>
                <div class="mcard-val">{gap_s} CZK</div>
            </div>
            <div class="calc-card">
                <div class="mcard-label">Monthly surplus</div>
                <div class="mcard-val">{surplus_s} CZK</div>
            </div>
        </div>"#,
        size = form.input.flat_size_m2,
        color = r.color_class,
    ))
}

/// POST /api/scenario/compare — returns `<tr>` rows for the compare table.
pub(super) async fn scenario_compare(
    State(state): State<AppState>,
    axum::Form(input): axum::Form<ScenarioInput>,
) -> impl IntoResponse {
    if let Err(errors) = input.validate() {
        let msgs = errors.join("; ");
        return Html(format!(r#"<tr><td colspan="6" class="scenario-errors">{msgs}</td></tr>"#));
    }

    let rows: Vec<(String, Option<f64>)> = sqlx::query_as(
        "SELECT region, avg_price_m2 FROM affordability
         WHERE avg_price_m2 IS NOT NULL AND region != 'national'
           AND date = (SELECT MAX(date) FROM affordability AS a2
                       WHERE a2.region = affordability.region)",
    )
    .fetch_all(&state.pool)
    .await
    .unwrap_or_default();

    let region_prices: Vec<(String, f64)> =
        rows.into_iter().filter_map(|(slug, price)| price.map(|p| (slug, p))).collect();

    let results = compute_scenario_all_regions(&input, &region_prices);

    let mut html = String::new();
    for (i, r) in results.iter().enumerate() {
        let payment_s = fmt_thousands(r.monthly_payment);
        let pti_s = format!("{:.0}%", r.payment_to_income_pct);
        let years_s = match r.years_to_deposit {
            Some(y) if y < 0.01 => "Ready!".to_string(),
            Some(y) => format!("{y:.1} yr"),
            None => "N/A".to_string(),
        };
        let best = if i == 0 { r#" class="best-fit-row""# } else { "" };
        html.push_str(&format!(
            r#"<tr{best}>
                <td><a href="/region/{slug}" class="link">{name}</a></td>
                <td class="r mono">{flat_price}</td>
                <td class="r mono">{payment_s}</td>
                <td class="r mono {color}">{pti_s}</td>
                <td class="r mono {color}">{years_s}</td>
            </tr>"#,
            slug = r.region,
            name = r.region_name,
            flat_price = fmt_thousands(r.flat_price),
            color = r.color_class,
        ));
    }
    Html(html)
}

#[derive(serde::Deserialize)]
pub(super) struct ScenarioRegionForm {
    pub region: String,
    #[serde(flatten)]
    pub input: ScenarioInput,
}
