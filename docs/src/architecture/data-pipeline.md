# Data Pipeline

## Fetchers

Each fetcher implements a similar pattern: HTTP request, parse response, insert into `time_series` table.

| Fetcher | Source | Format | Key challenges |
|---------|--------|--------|----------------|
| FRED | JSON API | Standard REST | Rate limiting |
| CNB | Plain text files | Pipe-delimited, Czech decimals | Encoding, number parsing |
| CZSO | CSV via package API | Standard CSV | Finding the right dataset ID |
| Sreality | Internal JSON API | `_embedded` field | Locality mapping, non-breaking spaces |

## Compute pipeline

The `compute` subcommand runs `compute_all()` which iterates over all regions:

1. Look up latest `avg_price_m2` from Sreality data
2. Look up latest wage from CZSO data, mapped via `city_to_kraj()`
3. Look up mortgage rate (CNB or fallback synthesis)
4. Calculate: flat price, mortgage payment, months-to-buy, net wage, savings timeline
5. Write one row per region to `affordability` table

## Historical snapshots

`compute --historical` generates affordability rows for 2010, 2015, and 2020 by:

- Using FRED property price index to scale current prices backward
- Using historical CZSO wage data
- Estimating rent by scaling current rent with price index ratio
