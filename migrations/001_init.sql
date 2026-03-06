-- Time series table (EAV-style for all indicators)
CREATE TABLE IF NOT EXISTS time_series (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    indicator TEXT NOT NULL,       -- e.g. 'nominal_property_price_index', 'avg_price_m2_flat'
    region TEXT NOT NULL,          -- e.g. 'praha', 'brno', 'national'
    date TEXT NOT NULL,            -- ISO date: 'YYYY-MM-DD' or 'YYYY-QQ'
    value REAL NOT NULL,
    unit TEXT NOT NULL DEFAULT '', -- e.g. 'index', 'CZK', 'CZK/m2', '%'
    source TEXT NOT NULL,          -- e.g. 'fred', 'cnb', 'czso', 'sreality'
    fetched_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(indicator, region, date, source)
);

CREATE INDEX IF NOT EXISTS idx_ts_indicator_region ON time_series(indicator, region);
CREATE INDEX IF NOT EXISTS idx_ts_date ON time_series(date);
CREATE INDEX IF NOT EXISTS idx_ts_source ON time_series(source);

-- Precomputed affordability metrics
CREATE TABLE IF NOT EXISTS affordability (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    region TEXT NOT NULL,
    date TEXT NOT NULL,                    -- quarter or month
    avg_price_m2 REAL,                    -- CZK per m2
    flat_60m2_price REAL,                 -- avg_price_m2 * 60
    avg_monthly_wage REAL,                -- CZK
    months_to_buy REAL,                   -- flat_60m2_price / avg_monthly_wage
    mortgage_rate_pct REAL,               -- current avg mortgage rate
    monthly_payment_30y REAL,             -- mortgage payment for 80% LTV, 30yr
    payment_to_wage_pct REAL,             -- monthly_payment / avg_monthly_wage * 100
    avg_rent_m2 REAL,                     -- average rent per m2
    monthly_rent_60m2 REAL,               -- avg_rent_m2 * 60
    rent_vs_mortgage_ratio REAL,          -- monthly_rent_60m2 / monthly_payment_30y
    computed_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(region, date)
);

CREATE INDEX IF NOT EXISTS idx_aff_region ON affordability(region);
CREATE INDEX IF NOT EXISTS idx_aff_date ON affordability(date);

-- Fetch log for data freshness tracking
CREATE TABLE IF NOT EXISTS fetch_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    source TEXT NOT NULL,
    indicator TEXT,
    status TEXT NOT NULL,          -- 'success', 'error'
    records_count INTEGER DEFAULT 0,
    error_message TEXT,
    started_at TEXT NOT NULL DEFAULT (datetime('now')),
    finished_at TEXT
);

CREATE INDEX IF NOT EXISTS idx_fl_source ON fetch_log(source);
