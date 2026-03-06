# Affordability Metrics

## Primary metric

**Months of net salary to buy a 60m2 flat** -- computed as:

```
months = flat_60m2_price / avg_monthly_wage_net
```

On the homepage and compare page, this is displayed as **years** (months / 12).

## Net wage calculation

Gross wages from CZSO are converted to net using Czech 2025 tax rules:
- 12.2% social/health insurance
- 15% / 23% income tax (progressive brackets)
- 2,570 CZK/month personal tax credit

Implemented in `compute/czech_tax.rs`.

## Additional metrics

| Metric | Formula |
|--------|---------|
| Flat price | `avg_price_m2 * 60` |
| Mortgage payment | Standard amortization formula (default: 80% LTV, 30y) |
| Payment-to-wage ratio | `monthly_payment / net_wage * 100` |
| Rent vs mortgage | `monthly_rent / monthly_payment` |
| Monthly savings | `net_wage - living_expenses` |
| Years to save | Future value of annuity formula (default: 7% annual return) |

## Severity colors

Based on months-to-buy:
- **heat-1** (green): < 100 months
- **heat-2**: 100-110
- **heat-3**: 110-130
- **heat-4**: 130-160
- **heat-5** (red): > 160 months
