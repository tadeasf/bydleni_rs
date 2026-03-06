# Bydleni.rs

Czech housing affordability dashboard built with Rust, Axum, Askama, HTMX, and Chart.js.

Answers the question: **How many years of net salary does it take to buy a 60m2 flat in the Czech Republic?**

## Features

- **Regional comparison** across 14 Czech regions + national average
- **Interactive SVG map** with heat-colored affordability data
- **Affordability forecasts** using linear regression on historical trends
- **Mortgage & savings calculators** with HTMX-powered real-time results
- **My Budget Scenario** mode -- enter your income, savings, and preferences to see personalized affordability across all regions (localStorage-backed, no account needed)
- **Auto-refresh** -- background data fetch every 6 hours via tokio-cron-scheduler

## Data Sources

| Source | Data | Records |
|--------|------|---------|
| [FRED](https://fred.stlouisfed.org) | Property price index, interest rates | ~1060 |
| [CNB](https://www.cnb.cz) | Repo rate, mortgage rate proxy | ~306 |
| [CZSO](https://www.czso.cz) | Average wages per region | ~210 |
| [Sreality](https://www.sreality.cz) | Flat sale/rent prices, listings | per region |

## Quick Start

```bash
# Clone and configure
cp .env.example .env
# Edit .env with your FRED API key

# Fetch data, compute metrics, and serve
cargo run -- fetch --all
cargo run -- compute
cargo run -- serve
# Open http://localhost:3000
```

## Commands

| Command | Description |
|---------|-------------|
| `cargo run -- fetch --all` | Fetch from all data sources |
| `cargo run -- compute` | Compute affordability metrics |
| `cargo run -- compute --historical` | Include historical snapshots (2010, 2015, 2020) |
| `cargo run -- serve` | Start web server on port 3000 |
| `cargo test` | Run unit tests |
| `cargo clippy -- -D warnings` | Lint |
| `cargo fmt` | Format code |

## Architecture

```
src/
  config.rs          -- env-based configuration
  db.rs              -- SQLite pool + migrations
  fetchers/          -- fred.rs, cnb.rs, czso.rs, sreality.rs
  models/            -- time_series.rs, affordability.rs, listing.rs
  compute/
    affordability.rs -- core metrics, mortgage calc, savings calc
    czech_tax.rs     -- gross-to-net wage conversion
    historical.rs    -- historical snapshot computation
    forecast.rs      -- linear regression + forecast charts
    scenario.rs      -- personalized budget scenario engine
    aggregation.rs   -- data aggregation helpers
  server/
    mod.rs           -- AppState, router, serve
    routes/          -- page handlers (index, region, compare)
    api/             -- JSON/HTMX endpoints (charts, forecast, scenario, status)
    templates.rs     -- Askama template structs + display helpers
    scheduler.rs     -- background refresh (startup + 6h cron)
    error.rs         -- AppError enum
    htmx.rs          -- HTMX request helpers
templates/           -- Askama HTML templates
static/              -- CSS, JS (charts.js, scenario.js), SVG map
migrations/          -- SQLite schema migrations
```

## Documentation

Full documentation is available at the [GitHub Pages site](https://tadeasf.github.io/bydleni_rs/) (built with mdBook).

To build locally:

```bash
mdbook serve docs
```

## License

MIT
