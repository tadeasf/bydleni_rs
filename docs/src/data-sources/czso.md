# CZSO (Czech Statistical Office)

## Overview

CZSO provides average wage data by region (kraj), essential for computing affordability ratios.

## Data access

Use the package API: `https://vdb.czso.cz/pll/eweb/package_show?id=DATASET_ID` to get CSV download URLs.

Dataset 110080 contains wage data.

## Key details

- Wage data is per **kraj** (region), not per city
- `city_to_kraj()` mapping connects Sreality city slugs to CZSO region slugs
- Approximately 210 records

## Mapping example

| City (Sreality) | Kraj (CZSO) |
|----------------|-------------|
| brno | jihomoravsky |
| ostrava | moravskoslezsky |
| plzen | plzensky |
