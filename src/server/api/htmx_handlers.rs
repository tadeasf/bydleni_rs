use axum::response::{Html, IntoResponse};
use serde::Deserialize;

use crate::compute::affordability::{mortgage_monthly_payment, years_to_save_with_investment};
use crate::server::AppState;
use crate::server::templates::fmt_thousands;

use axum::extract::State;

#[derive(Deserialize)]
pub(super) struct MortgageCalcForm {
    price: Option<f64>,
    rate: Option<f64>,
    years: Option<u32>,
    ltv: Option<f64>,
}

/// HTMX mortgage calculator handler — returns HTML fragment.
pub(super) async fn mortgage_calc(
    axum::Form(form): axum::Form<MortgageCalcForm>,
) -> impl IntoResponse {
    let price = form.price.unwrap_or(5_000_000.0);
    let rate = form.rate.unwrap_or(5.0);
    let years = form.years.unwrap_or(30);
    let ltv = form.ltv.unwrap_or(80.0);

    let principal = price * (ltv / 100.0);
    let months = years * 12;
    let payment = mortgage_monthly_payment(principal, rate, months);
    let total = payment * months as f64;
    let total_interest = total - principal;

    let payment_s = fmt_thousands(payment);
    let total_s = fmt_thousands(total);
    let interest_s = fmt_thousands(total_interest);

    Html(format!(
        r#"<div class="calc-grid">
            <div class="calc-card primary">
                <div class="mcard-label">Monthly payment</div>
                <div class="mcard-val ts">{payment_s} CZK</div>
            </div>
            <div class="calc-card">
                <div class="mcard-label">Total paid</div>
                <div class="mcard-val">{total_s} CZK</div>
            </div>
            <div class="calc-card">
                <div class="mcard-label">Total interest</div>
                <div class="mcard-val">{interest_s} CZK</div>
            </div>
        </div>"#
    ))
}

#[derive(Deserialize)]
pub(super) struct RecalcSavingsForm {
    region: Option<String>,
    net_income: Option<f64>,
    expenses: Option<f64>,
    return_pct: Option<f64>,
}

/// HTMX handler: recalculate years-to-save with user-provided income, expenses and return rate.
pub(super) async fn recalc_savings(
    State(state): State<AppState>,
    axum::Form(form): axum::Form<RecalcSavingsForm>,
) -> impl IntoResponse {
    let region = form.region.as_deref().unwrap_or("praha");
    let expenses = form.expenses.unwrap_or(17_000.0);
    let return_pct = form.return_pct.unwrap_or(7.0);

    // Get latest data for region (used as fallback if no net_income provided)
    let row: Option<(Option<f64>, Option<f64>)> = sqlx::query_as(
        "SELECT avg_monthly_wage_net, flat_60m2_price FROM affordability
         WHERE region = ? ORDER BY date DESC LIMIT 1",
    )
    .bind(region)
    .fetch_optional(&state.pool)
    .await
    .unwrap_or(None);

    let (db_net_wage, flat_price) = row.unwrap_or((None, None));
    let net_income = form.net_income.or(db_net_wage).unwrap_or(0.0);

    let savings = (net_income - expenses).max(0.0);
    let years = match flat_price {
        Some(target) if savings > 0.0 => years_to_save_with_investment(target, savings, return_pct),
        _ => f64::INFINITY,
    };

    let years_str = if years.is_infinite() { "N/A".to_string() } else { format!("{years:.1}") };
    let savings_str = fmt_thousands(savings);
    let income_str = fmt_thousands(net_income);

    Html(format!(
        r#"<div class="calc-grid">
            <div class="calc-card primary">
                <div class="mcard-label">Years to save (investing)</div>
                <div class="mcard-val ts">{years_str} years</div>
            </div>
            <div class="calc-card">
                <div class="mcard-label">Monthly net income</div>
                <div class="mcard-val">{income_str} CZK</div>
            </div>
            <div class="calc-card">
                <div class="mcard-label">Monthly savings</div>
                <div class="mcard-val">{savings_str} CZK</div>
            </div>
            <div class="calc-card">
                <div class="mcard-label">Annual return</div>
                <div class="mcard-val">{return_pct:.1}%</div>
            </div>
        </div>"#
    ))
}
