# Calculators

## Mortgage calculator

Available on each region detail page. Computes:

- **Monthly payment** using standard amortization formula
- **Total paid** over the loan term
- **Total interest** paid

Default values are prefilled from the region's data (flat price, current mortgage rate). If a scenario is active, values are prefilled from the scenario instead.

### Formula

```
P = principal * (r * (1+r)^n) / ((1+r)^n - 1)
```

Where `r` = monthly rate, `n` = number of months.

## Savings calculator

Also on each region detail page. Computes years to save the full flat price by investing monthly savings at a given return rate.

Uses the future value of annuity formula:

```
n = ln(1 + target * r / pmt) / ln(1 + r)
```

Where `r` = monthly rate, `pmt` = monthly savings amount.

## HTMX integration

Both calculators use `hx-post` to submit the form and replace the result div with an HTML fragment returned by the server. No JavaScript framework needed.
