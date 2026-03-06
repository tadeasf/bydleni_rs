# My Budget Scenario

## Overview

The scenario mode lets visitors enter their personal financial details and see personalized affordability across all regions. No account needed -- data is stored in the browser's localStorage.

## Inputs

| Field | Default | Description |
|-------|---------|-------------|
| Net income | (required) | Monthly net income in CZK |
| Current savings | 0 | Existing savings in CZK |
| Flat size | 60 m2 | Target flat size |
| Mortgage rate | 5.0% | Annual mortgage interest rate |
| LTV | 80% | Loan-to-value ratio |
| Mortgage term | 30 years | Loan duration |
| Monthly expenses | 17,000 CZK | Living expenses (excl. housing) |
| Investment return | 7% | Expected annual return on savings |

## Outputs per region

- **Flat price**: `avg_price_m2 * flat_size_m2`
- **Deposit needed**: `flat_price * (1 - LTV/100)`
- **Deposit gap**: `max(deposit_needed - current_savings, 0)`
- **Monthly payment**: Standard mortgage amortization
- **Payment/income %**: Affordability ratio (< 40% = affordable)
- **Monthly surplus**: `max(net_income - expenses, 0)`
- **Years to deposit**: Time to save the deposit gap (with investment returns)

## Cross-page behavior

- **Index page**: Full scenario form + personalized region cards sorted by affordability
- **Region page**: Personalized hero stats + prefilled calculators
- **Compare page**: Toggle between default view and personalized ranking table
- **Persistence**: Form values auto-restore on page reload from localStorage

## Implementation

- Backend: `src/compute/scenario.rs` (engine) + `src/server/api/scenario.rs` (4 HTMX endpoints)
- Frontend: `static/scenario.js` (localStorage + form management + cross-page sync)
