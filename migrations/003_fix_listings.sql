-- Fix: remove UNIQUE constraints that cause failures when listings share names.
-- DELETE-then-INSERT in upsert_batch handles deduplication, no constraint needed.

CREATE TABLE IF NOT EXISTS example_listings_new (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    region TEXT NOT NULL,
    listing_type TEXT NOT NULL,
    name TEXT NOT NULL,
    price INTEGER NOT NULL,
    area_m2 REAL,
    price_per_m2 REAL,
    url TEXT NOT NULL,
    fetched_at TEXT NOT NULL DEFAULT (datetime('now'))
);

INSERT OR IGNORE INTO example_listings_new (region, listing_type, name, price, area_m2, price_per_m2, url, fetched_at)
    SELECT region, listing_type, name, price, area_m2, price_per_m2, url, fetched_at FROM example_listings;

DROP TABLE example_listings;
ALTER TABLE example_listings_new RENAME TO example_listings;

CREATE INDEX IF NOT EXISTS idx_el_region_type ON example_listings(region, listing_type);
