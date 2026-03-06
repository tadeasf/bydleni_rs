# Methodology Transparency

The dashboard displays inline expandable disclosures next to key metrics so visitors can understand how numbers are calculated, which data sources are used, and when data was last refreshed.

## How It Works

Each metric group has a `<details>` disclosure rendered server-side. Clicking the summary label expands a paragraph with the formula, assumptions, and source attribution. No JavaScript required.

## Metric Groups

### Years/Months to Buy

**Formula:** `(avg_price_m2 x 60) / net_monthly_wage` (months variant), divided by 12 for years.

**Net wage:** Czech 2025 tax rules — 12.2% social/health insurance + 15%/23% income tax brackets, minus 2,570 CZK/month personal tax credit.

**Sources:** CZSO (wages by region), Sreality (asking prices).

### Price per m2

Average asking price from current Sreality listings for the region. National figure is the average across all tracked regions.

**Source:** Sreality.

### Mortgage Payment

**Formula:** Standard amortization `M = P[r(1+r)^n] / [(1+r)^n - 1]`.

**Defaults:** 80% LTV, 30-year term. Interest rate from CNB MFI survey (`mortgage_rate_avg`); fallback: CNB 2-week repo rate + 2.5pp spread.

**Source:** CNB.

### Rent vs Mortgage

**Formula:** `(avg_rent_m2 x 60) / monthly_mortgage_payment`. Values above 1.0 mean renting is more expensive.

**Sources:** Sreality (rent listings), CNB (mortgage rates).

### Savings & Time to Ownership

**Formula:** `monthly_savings = net_wage - living_expenses`. Years to save for a 20% down payment via compound annuity formula at 7% annual investment return.

**Source:** CZSO (consumer basket, region-specific).

### Affordability Forecast

Price and wage indices extrapolated 5 years forward using the slope between the two most recent data points (recent-slope method). This is a trend projection, not a prediction.

**Sources:** FRED (property price index), CZSO (wage index).

### Rent Burden Forecast

Rent as percentage of net wage, projected via recent-slope extrapolation over 5 years.

**Sources:** Sreality (rent listings), CZSO (wages).

## Data Freshness

Data is refreshed automatically every 6 hours (startup fetch + cron). The `last_refresh` timestamp is displayed in the hero section on the index page and appended to each methodology disclosure when available.

## Assumptions & Limitations

- Prices are **asking prices**, not transaction prices
- Wage data is per **kraj** (region), not per city
- Mortgage rate fallback (repo + 2.5pp) is an approximation
- Forecasts assume recent momentum continues unchanged
- Living expenses use national/regional consumer baskets, not individual spending

## Extending

To add a new disclosure:

1. Add a field to the relevant `*Methodology` struct in `src/server/methodology.rs`
2. Build the `MethodologyNote` in the corresponding `build_*_methodology()` function
3. Add the `explain_*: String` field to the template struct in `src/server/templates.rs`
4. Pass the value in the route handler
5. Emit `{{ explain_*|safe }}` in the Askama template
