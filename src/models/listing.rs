#![allow(dead_code)]

use anyhow::Result;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

/// An example listing from Sreality, near the median price.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ExampleListing {
    pub id: Option<i64>,
    pub region: String,
    pub listing_type: String,
    pub name: String,
    pub price: i64,
    pub area_m2: Option<f64>,
    pub price_per_m2: Option<f64>,
    pub url: String,
    pub fetched_at: Option<String>,
}

/// Replace all listings for a region+type and insert new ones.
pub async fn upsert_batch(
    pool: &SqlitePool,
    region: &str,
    listing_type: &str,
    listings: &[ExampleListing],
) -> Result<usize> {
    let mut tx = pool.begin().await?;

    sqlx::query("DELETE FROM example_listings WHERE region = ? AND listing_type = ?")
        .bind(region)
        .bind(listing_type)
        .execute(&mut *tx)
        .await?;

    let mut count = 0;
    for l in listings {
        sqlx::query(
            "INSERT INTO example_listings
             (region, listing_type, name, price, area_m2, price_per_m2, url)
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&l.region)
        .bind(&l.listing_type)
        .bind(&l.name)
        .bind(l.price)
        .bind(l.area_m2)
        .bind(l.price_per_m2)
        .bind(&l.url)
        .execute(&mut *tx)
        .await?;
        count += 1;
    }

    tx.commit().await?;
    Ok(count)
}

/// Query example listings for a region and type, limited to `limit` results.
pub async fn query_by_region(
    pool: &SqlitePool,
    region: &str,
    listing_type: &str,
    limit: u32,
) -> Result<Vec<ExampleListing>> {
    let rows = sqlx::query_as::<_, ExampleListing>(
        "SELECT id, region, listing_type, name, price, area_m2, price_per_m2, url, fetched_at
         FROM example_listings
         WHERE region = ? AND listing_type = ?
         ORDER BY price ASC
         LIMIT ?",
    )
    .bind(region)
    .bind(listing_type)
    .bind(limit)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}
