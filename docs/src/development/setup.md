# Development Setup

## Prerequisites

- Rust toolchain (stable, edition 2024)
- SQLite3

## Environment

Copy `.env.example` to `.env` and set:

```
DATABASE_URL=sqlite:data.db
FRED_API_KEY=your_key_here
```

Get a free FRED API key at [https://fred.stlouisfed.org/docs/api/api_key.html](https://fred.stlouisfed.org/docs/api/api_key.html).

## First run

```bash
# Fetch data from all sources
cargo run -- fetch --all

# Compute affordability metrics
cargo run -- compute

# Start development server
cargo run -- serve
```

The server starts on port 3000. Open [http://localhost:3000](http://localhost:3000).

## Development workflow

The server auto-refreshes data every 6 hours. For development, you can re-run `fetch` and `compute` manually.

```bash
# Re-fetch a single source
cargo run -- fetch --source sreality

# Recompute with historical snapshots
cargo run -- compute --historical
```
