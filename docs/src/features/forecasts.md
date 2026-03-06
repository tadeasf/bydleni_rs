# Forecasts

## Method

Forecasts use **recent-slope extrapolation** based on the last 2 data points, projecting 5 years forward.

Two forecast charts are displayed on the homepage:

### Affordability forecast
- Shows housing affordability index over time
- Solid line = historical data, dashed line = projection
- Y-axis: index (base year = 100)

### Rent burden forecast
- Shows what percentage of net salary rent costs
- Solid line = historical data, dashed line = projection
- Y-axis: % of net salary

## Implementation

`src/compute/forecast.rs` contains:

- `linear_regression()` -- full dataset regression (kept but unused)
- `recent_slope()` -- slope from last 2 data points
- `extrapolate_recent()` -- project values forward
- `build_affordability_forecast()` -- Chart.js-ready data
- `build_rent_burden_forecast()` -- Chart.js-ready data

## Chart rendering

Forecast data is served as JSON from `/api/chart/forecast` and `/api/chart/rent-burden`, rendered client-side by `loadForecastChart()` in `charts.js`.
