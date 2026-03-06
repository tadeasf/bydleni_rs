# Bydleni.rs

**Czech housing affordability dashboard** built with Rust, Axum, Askama, HTMX, and Chart.js.

This application aggregates data from four sources (FRED, CNB, CZSO, Sreality) to answer a simple question:

> How many years of net salary does it take to buy a 60m2 flat in the Czech Republic?

## Key capabilities

- Regional affordability comparison across all 14 Czech regions
- Interactive SVG map with heat-colored data
- Affordability and rent-burden forecasts
- Mortgage and savings calculators
- Personalized "My Budget Scenario" mode (localStorage, no account)
- Automatic background data refresh every 6 hours

## Tech stack

| Layer | Technology |
|-------|-----------|
| Backend | Rust + Axum |
| Templates | Askama (Jinja2-like) |
| Interactivity | HTMX |
| Charts | Chart.js |
| Database | SQLite (WAL mode) |
| Scheduling | tokio-cron-scheduler |
