#![allow(dead_code)]

use anyhow::Result;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

/// Precomputed affordability metrics for a region and time period.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Affordability {
    pub id: Option<i64>,
    pub region: String,
    pub date: String,
    pub avg_price_m2: Option<f64>,
    pub flat_60m2_price: Option<f64>,
    pub avg_monthly_wage: Option<f64>,
    pub months_to_buy: Option<f64>,
    pub mortgage_rate_pct: Option<f64>,
    pub monthly_payment_30y: Option<f64>,
    pub payment_to_wage_pct: Option<f64>,
    pub avg_rent_m2: Option<f64>,
    pub monthly_rent_60m2: Option<f64>,
    pub rent_vs_mortgage_ratio: Option<f64>,
    pub computed_at: Option<String>,
    // Phase 3 fields
    pub avg_monthly_wage_net: Option<f64>,
    pub months_to_buy_gross: Option<f64>,
    pub avg_living_expenses: Option<f64>,
    pub monthly_savings: Option<f64>,
    pub years_to_save_investing: Option<f64>,
}

/// Insert or replace an affordability record.
pub async fn upsert(pool: &SqlitePool, record: &Affordability) -> Result<()> {
    sqlx::query(
        "INSERT OR REPLACE INTO affordability
         (region, date, avg_price_m2, flat_60m2_price, avg_monthly_wage,
          months_to_buy, mortgage_rate_pct, monthly_payment_30y,
          payment_to_wage_pct, avg_rent_m2, monthly_rent_60m2,
          rent_vs_mortgage_ratio, avg_monthly_wage_net, months_to_buy_gross,
          avg_living_expenses, monthly_savings, years_to_save_investing)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&record.region)
    .bind(&record.date)
    .bind(record.avg_price_m2)
    .bind(record.flat_60m2_price)
    .bind(record.avg_monthly_wage)
    .bind(record.months_to_buy)
    .bind(record.mortgage_rate_pct)
    .bind(record.monthly_payment_30y)
    .bind(record.payment_to_wage_pct)
    .bind(record.avg_rent_m2)
    .bind(record.monthly_rent_60m2)
    .bind(record.rent_vs_mortgage_ratio)
    .bind(record.avg_monthly_wage_net)
    .bind(record.months_to_buy_gross)
    .bind(record.avg_living_expenses)
    .bind(record.monthly_savings)
    .bind(record.years_to_save_investing)
    .execute(pool)
    .await?;
    Ok(())
}

/// Query affordability data by region and optional date range.
pub async fn query(
    pool: &SqlitePool,
    region: &str,
    date_from: Option<&str>,
    date_to: Option<&str>,
) -> Result<Vec<Affordability>> {
    let mut sql = String::from(
        "SELECT id, region, date, avg_price_m2, flat_60m2_price, avg_monthly_wage,
                months_to_buy, mortgage_rate_pct, monthly_payment_30y,
                payment_to_wage_pct, avg_rent_m2, monthly_rent_60m2,
                rent_vs_mortgage_ratio, computed_at,
                avg_monthly_wage_net, months_to_buy_gross,
                avg_living_expenses, monthly_savings, years_to_save_investing
         FROM affordability WHERE region = ?",
    );
    if date_from.is_some() {
        sql.push_str(" AND date >= ?");
    }
    if date_to.is_some() {
        sql.push_str(" AND date <= ?");
    }
    sql.push_str(" ORDER BY date ASC");

    let mut q = sqlx::query_as::<_, Affordability>(&sql).bind(region);
    if let Some(from) = date_from {
        q = q.bind(from);
    }
    if let Some(to) = date_to {
        q = q.bind(to);
    }

    let rows = q.fetch_all(pool).await?;
    Ok(rows)
}
