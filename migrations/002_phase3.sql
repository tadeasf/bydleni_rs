-- Phase 3: Real-world affordability enhancements

-- Net wage + gross comparison
ALTER TABLE affordability ADD COLUMN avg_monthly_wage_net REAL;
ALTER TABLE affordability ADD COLUMN months_to_buy_gross REAL;

-- Savings/investment calculation
ALTER TABLE affordability ADD COLUMN avg_living_expenses REAL;
ALTER TABLE affordability ADD COLUMN monthly_savings REAL;
ALTER TABLE affordability ADD COLUMN years_to_save_investing REAL;

-- Example listings
CREATE TABLE IF NOT EXISTS example_listings (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    region TEXT NOT NULL,
    listing_type TEXT NOT NULL,  -- 'sale' or 'rent'
    name TEXT NOT NULL,
    price INTEGER NOT NULL,
    area_m2 REAL,
    price_per_m2 REAL,
    url TEXT NOT NULL,
    fetched_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(region, url)
);
CREATE INDEX IF NOT EXISTS idx_el_region_type ON example_listings(region, listing_type);
