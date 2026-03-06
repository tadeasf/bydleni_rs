# Architecture Overview

Bydleni.rs is a CLI application with three subcommands:

- **`fetch`** -- download data from external sources into SQLite
- **`compute`** -- calculate affordability metrics from raw data
- **`serve`** -- start the Axum web server

## Module structure

```
src/
  main.rs            -- clap CLI entry point
  config.rs          -- env-based Config struct
  db.rs              -- SQLite connection pool + migrations
  fetchers/          -- data acquisition (FRED, CNB, CZSO, Sreality)
  models/            -- database row types
  compute/           -- business logic (affordability, tax, forecast, scenario)
  server/            -- web server (routes, API, templates, scheduler)
```

## Data flow

1. **Fetch**: Each fetcher downloads data from its source and stores raw time-series rows in the `time_series` table (EAV model).
2. **Compute**: The affordability module reads time-series data, applies Czech tax rules, and writes pre-computed rows to the `affordability` table.
3. **Serve**: The web server reads from `affordability` and renders pages. Background scheduler re-runs fetch + compute every 6 hours.

## Database

SQLite with WAL mode for concurrent reads. Two migration files:

- `001_init.sql` -- `time_series`, `affordability`, `fetch_log` tables
- `002_phase3.sql` -- net wage columns, savings columns, `example_listings` table
