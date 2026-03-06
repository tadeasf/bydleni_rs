# CNB (Czech National Bank)

## Overview

CNB provides monetary policy data, including the 2-week repo rate used to estimate mortgage rates.

## Data access

The ARAD REST API requires registration. Instead, we use freely available plain-text history files at `cnb.cz/cs/casto-kladene-dotazy/.galleries/`.

## Format

- Pipe-delimited text files
- Czech decimal commas (`,` instead of `.`)
- Approximately 306 records

## Mortgage rate

The mortgage rate is synthesized as `repo_rate_2w + 2.5 percentage points` (spread). If real MFI data (`mortgage_rate_avg`) is available, it takes priority.
