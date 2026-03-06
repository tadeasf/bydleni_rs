# Sreality

## Overview

Sreality is the largest Czech real estate portal. We use its internal JSON API to fetch current flat prices (sale and rent) across regions.

## API

Endpoint: `https://www.sreality.cz/api/cs/v2/estates`

Key parameters:
- `category_main_cb=1` -- flats
- `category_type_cb=1` -- sale (`2` for rent)
- `locality_region_id` -- for Praha (spans multiple districts)
- `locality_district_id` -- for other cities

## Challenges

- SPA with internal API (no official documentation)
- JSON uses `_embedded` field (requires serde rename)
- Listing names contain non-breaking spaces (`\u{a0}`) before "m2"
- Detail URLs constructed from `hash_id` + SEO locality slug

## Data extracted

- Average price per m2 (sale and rent)
- Median price per m2
- Example listings near median for display on region pages
