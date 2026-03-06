# FRED (Federal Reserve Economic Data)

## Overview

FRED provides historical property price indices and interest rate data used for trend analysis and historical comparisons.

## API

Standard JSON REST API at `https://api.stlouisfed.org/fred/series/observations`.

Requires a free API key (set via `FRED_API_KEY` environment variable).

## Data used

- Property price index for Czech Republic
- Used to scale current prices backward for historical snapshots
- Approximately 1,060 records
