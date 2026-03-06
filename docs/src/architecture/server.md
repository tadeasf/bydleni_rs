# Server Architecture

## Router structure

The Axum server has two route groups:

### Page routes (`server/routes/`)
- `GET /` -- index page (SVG map, chart, data table)
- `GET /region/:slug` -- region detail page
- `GET /compare` -- regional comparison page

### API routes (`server/api/`)
- `GET /api/chart/*` -- Chart.js JSON data endpoints
- `POST /api/mortgage-calc` -- HTMX mortgage calculator
- `POST /api/recalc-savings` -- HTMX savings recalculator
- `POST /api/scenario/*` -- personalized scenario endpoints
- `GET /api/status` -- refresh status

## AppState

Shared state contains:
- `pool: SqlitePool` -- database connection
- `refreshing: AtomicBool` -- whether a background fetch is running
- `last_refresh: RwLock<Option<String>>` -- timestamp of last successful refresh

## Templates

Askama templates with a custom `HtmlTemplate<T>` wrapper that implements `IntoResponse`. Display helpers (`fmt_thousands`, `severity_color`, `slug_to_display_name`) are in `templates.rs`.

## Background scheduler

On startup, the server triggers an immediate fetch+compute, then schedules a cron job every 6 hours via `tokio-cron-scheduler`. The server starts serving immediately with whatever data exists in SQLite.
