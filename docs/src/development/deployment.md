# Deployment

## Build

```bash
cargo build --release
```

The binary is at `target/release/bydleni_rs`.

## Running in production

```bash
DATABASE_URL=sqlite:data.db FRED_API_KEY=xxx ./bydleni_rs serve
```

The server listens on port 3000. Use a reverse proxy (Caddy, nginx) for:
- TLS termination
- Compression (gzip/brotli) -- the app does not compress responses itself
- Static file caching

## Caddy example

```
bydleni.example.com {
    reverse_proxy localhost:3000
    encode gzip
}
```

## Data persistence

All data is in a single SQLite file (`data.db` by default). The background scheduler fetches and recomputes every 6 hours, so the database stays current without manual intervention.

## Documentation

Documentation is built with mdBook and deployed to GitHub Pages via the `.github/workflows/docs.yml` workflow. It triggers on pushes to `master` that change files in `docs/`.
