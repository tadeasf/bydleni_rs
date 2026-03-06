use serde::Serialize;

/// Result of a forecast computation, ready for Chart.js.
#[derive(Serialize)]
pub struct ForecastResult {
    pub labels: Vec<String>,
    pub datasets: Vec<ForecastDataset>,
}

#[derive(Serialize)]
pub struct ForecastDataset {
    pub label: String,
    pub data: Vec<Option<f64>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "borderColor")]
    pub border_color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "borderDash")]
    pub border_dash: Option<Vec<u32>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "yAxisID")]
    pub y_axis_id: Option<String>,
}

/// Ordinary least squares linear regression.
/// Returns (intercept, slope) or None if not enough data.
#[allow(dead_code)]
pub fn linear_regression(points: &[(f64, f64)]) -> Option<(f64, f64)> {
    let n = points.len() as f64;
    if n < 2.0 {
        return None;
    }

    let sum_x: f64 = points.iter().map(|(x, _)| x).sum();
    let sum_y: f64 = points.iter().map(|(_, y)| y).sum();
    let sum_xy: f64 = points.iter().map(|(x, y)| x * y).sum();
    let sum_x2: f64 = points.iter().map(|(x, _)| x * x).sum();

    let denom = n * sum_x2 - sum_x * sum_x;
    if denom.abs() < f64::EPSILON {
        return None;
    }

    let slope = (n * sum_xy - sum_x * sum_y) / denom;
    let intercept = (sum_y - slope * sum_x) / n;

    Some((intercept, slope))
}

/// Extrapolate using linear coefficients.
#[allow(dead_code)]
pub fn extrapolate(intercept: f64, slope: f64, x: f64) -> f64 {
    intercept + slope * x
}

/// Compute slope from the last two data points (recent trend).
/// This captures the most recent acceleration rather than averaging across all history.
fn recent_slope(points: &[(f64, f64)]) -> f64 {
    if points.len() < 2 {
        return 0.0;
    }
    let (x1, y1) = points[points.len() - 2];
    let (x2, y2) = points[points.len() - 1];
    let dx = x2 - x1;
    if dx.abs() < f64::EPSILON {
        return 0.0;
    }
    (y2 - y1) / dx
}

/// Extrapolate from the last actual value using recent slope.
fn extrapolate_recent(last_value: f64, slope: f64, last_x: f64, x: f64) -> f64 {
    last_value + slope * (x - last_x)
}

/// A historical affordability snapshot for forecast computation.
pub struct AffordabilitySnapshot {
    pub year: i32,
    pub avg_price_m2: f64,
    pub avg_monthly_wage_net: f64,
    pub months_to_buy: f64,
    pub monthly_rent_60m2: Option<f64>,
}

/// Build affordability forecast with price index, wage index, and years-to-buy.
///
/// Normalizes price and wage to indices (base year = first snapshot = 100),
/// fits linear regressions, and extrapolates `forecast_years` into the future.
/// Returns paired datasets: solid for historical, dashed for forecast.
pub fn build_affordability_forecast(
    snapshots: &[AffordabilitySnapshot],
    forecast_years: u32,
) -> Option<ForecastResult> {
    if snapshots.len() < 2 {
        return None;
    }

    let base_price = snapshots[0].avg_price_m2;
    let base_wage = snapshots[0].avg_monthly_wage_net;
    if base_price <= 0.0 || base_wage <= 0.0 {
        return None;
    }

    // Build historical indexed data
    let price_points: Vec<(f64, f64)> =
        snapshots.iter().map(|s| (s.year as f64, s.avg_price_m2 / base_price * 100.0)).collect();
    let wage_points: Vec<(f64, f64)> = snapshots
        .iter()
        .map(|s| (s.year as f64, s.avg_monthly_wage_net / base_wage * 100.0))
        .collect();
    let years_points: Vec<(f64, f64)> =
        snapshots.iter().map(|s| (s.year as f64, s.months_to_buy / 12.0)).collect();

    // Use recent slope (last 2 points) for forecast — captures recent trend
    let price_slope = recent_slope(&price_points);
    let wage_slope = recent_slope(&wage_points);
    let years_slope = recent_slope(&years_points);

    let last_year = snapshots.last().unwrap().year;
    let first_year = snapshots[0].year;
    let last_price = price_points.last().unwrap().1;
    let last_wage = wage_points.last().unwrap().1;
    let last_years = years_points.last().unwrap().1;
    let last_year_f = last_year as f64;

    // Build labels: all historical years + forecast years
    let mut labels = Vec::new();
    for s in snapshots {
        labels.push(s.year.to_string());
    }
    for i in 1..=forecast_years as i32 {
        labels.push((last_year + i).to_string());
    }

    let total_len = labels.len();
    let hist_len = snapshots.len();

    // Price index: historical (solid)
    let mut price_hist: Vec<Option<f64>> = price_points.iter().map(|(_, v)| Some(*v)).collect();
    price_hist.resize(total_len, None);

    // Price index: forecast (dashed) — starts at last historical point for continuity
    let mut price_forecast: Vec<Option<f64>> = vec![None; hist_len - 1];
    price_forecast.push(Some(last_price));
    for i in 1..=forecast_years as i32 {
        let year = (last_year + i) as f64;
        price_forecast.push(Some(extrapolate_recent(last_price, price_slope, last_year_f, year)));
    }

    // Wage index: historical (solid)
    let mut wage_hist: Vec<Option<f64>> = wage_points.iter().map(|(_, v)| Some(*v)).collect();
    wage_hist.resize(total_len, None);

    // Wage index: forecast (dashed)
    let mut wage_forecast: Vec<Option<f64>> = vec![None; hist_len - 1];
    wage_forecast.push(Some(last_wage));
    for i in 1..=forecast_years as i32 {
        let year = (last_year + i) as f64;
        wage_forecast.push(Some(extrapolate_recent(last_wage, wage_slope, last_year_f, year)));
    }

    // Years to buy: historical (solid)
    let mut years_hist: Vec<Option<f64>> = years_points.iter().map(|(_, v)| Some(*v)).collect();
    years_hist.resize(total_len, None);

    // Years to buy: forecast (dashed)
    let mut years_forecast: Vec<Option<f64>> = vec![None; hist_len - 1];
    years_forecast.push(Some(last_years));
    for i in 1..=forecast_years as i32 {
        let year = (last_year + i) as f64;
        years_forecast.push(Some(extrapolate_recent(last_years, years_slope, last_year_f, year)));
    }

    let base_year_label = first_year.to_string();

    Some(ForecastResult {
        labels,
        datasets: vec![
            ForecastDataset {
                label: format!("Property price index ({base_year_label}=100)"),
                data: price_hist,
                border_color: Some("#e8524a".to_string()),
                border_dash: None,
                y_axis_id: Some("y".to_string()),
            },
            ForecastDataset {
                label: "Property price (projected)".to_string(),
                data: price_forecast,
                border_color: Some("#e8524a".to_string()),
                border_dash: Some(vec![6, 4]),
                y_axis_id: Some("y".to_string()),
            },
            ForecastDataset {
                label: format!("Wage index ({base_year_label}=100)"),
                data: wage_hist,
                border_color: Some("#10b981".to_string()),
                border_dash: None,
                y_axis_id: Some("y".to_string()),
            },
            ForecastDataset {
                label: "Wage (projected)".to_string(),
                data: wage_forecast,
                border_color: Some("#10b981".to_string()),
                border_dash: Some(vec![6, 4]),
                y_axis_id: Some("y".to_string()),
            },
            ForecastDataset {
                label: "Years to buy".to_string(),
                data: years_hist,
                border_color: Some("#f2845c".to_string()),
                border_dash: None,
                y_axis_id: Some("y1".to_string()),
            },
            ForecastDataset {
                label: "Years to buy (projected)".to_string(),
                data: years_forecast,
                border_color: Some("#f2845c".to_string()),
                border_dash: Some(vec![6, 4]),
                y_axis_id: Some("y1".to_string()),
            },
        ],
    })
}

/// Build rent burden forecast (% of net salary spent on rent for 60m2).
pub fn build_rent_burden_forecast(
    snapshots: &[AffordabilitySnapshot],
    forecast_years: u32,
) -> Option<ForecastResult> {
    // Filter to snapshots that have rent data
    let rent_data: Vec<(f64, f64)> = snapshots
        .iter()
        .filter_map(|s| {
            s.monthly_rent_60m2.map(|rent| {
                let burden = rent / s.avg_monthly_wage_net * 100.0;
                (s.year as f64, burden)
            })
        })
        .collect();

    if rent_data.is_empty() {
        return None;
    }

    let slope = recent_slope(&rent_data);
    let last_year = rent_data.last().unwrap().0 as i32;
    let last_value = rent_data.last().unwrap().1;
    let last_year_f = last_year as f64;

    let mut labels = Vec::new();
    for (y, _) in &rent_data {
        labels.push((*y as i32).to_string());
    }
    for i in 1..=forecast_years as i32 {
        labels.push((last_year + i).to_string());
    }

    let total_len = labels.len();
    let hist_len = rent_data.len();

    // Historical burden (solid)
    let mut burden_hist: Vec<Option<f64>> = rent_data.iter().map(|(_, v)| Some(*v)).collect();
    burden_hist.resize(total_len, None);

    // Forecast burden (dashed) — recent slope extrapolation from last actual value
    let mut burden_forecast: Vec<Option<f64>> = vec![None; hist_len - 1];
    burden_forecast.push(Some(last_value));
    for i in 1..=forecast_years as i32 {
        let year = (last_year + i) as f64;
        burden_forecast.push(Some(extrapolate_recent(last_value, slope, last_year_f, year)));
    }

    Some(ForecastResult {
        labels,
        datasets: vec![
            ForecastDataset {
                label: "Rent burden (% of net salary)".to_string(),
                data: burden_hist,
                border_color: Some("#8b5cf6".to_string()),
                border_dash: None,
                y_axis_id: None,
            },
            ForecastDataset {
                label: "Rent burden (projected)".to_string(),
                data: burden_forecast,
                border_color: Some("#8b5cf6".to_string()),
                border_dash: Some(vec![6, 4]),
                y_axis_id: None,
            },
        ],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_linear_regression() {
        // Perfect line: y = 2x + 1
        let points = vec![(1.0, 3.0), (2.0, 5.0), (3.0, 7.0), (4.0, 9.0)];
        let (a, b) = linear_regression(&points).unwrap();
        assert!((a - 1.0).abs() < 1e-10, "intercept: {a}");
        assert!((b - 2.0).abs() < 1e-10, "slope: {b}");
    }

    #[test]
    fn test_linear_regression_insufficient_data() {
        assert!(linear_regression(&[(1.0, 2.0)]).is_none());
        assert!(linear_regression(&[]).is_none());
    }

    #[test]
    fn test_extrapolate() {
        // y = 1 + 2*x
        assert!((extrapolate(1.0, 2.0, 5.0) - 11.0).abs() < 1e-10);
    }

    #[test]
    fn test_recent_slope() {
        let points = vec![(2020.0, 100.0), (2025.0, 150.0)];
        let slope = recent_slope(&points);
        assert!((slope - 10.0).abs() < 1e-10);
    }

    #[test]
    fn test_recent_slope_insufficient() {
        assert!((recent_slope(&[(1.0, 2.0)]) - 0.0).abs() < 1e-10);
        assert!((recent_slope(&[]) - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_extrapolate_recent() {
        // From (2025, 150) with slope 10 per year
        let val = extrapolate_recent(150.0, 10.0, 2025.0, 2027.0);
        assert!((val - 170.0).abs() < 1e-10);
    }

    #[test]
    fn test_build_forecast_with_mock_data() {
        let snapshots = vec![
            AffordabilitySnapshot {
                year: 2010,
                avg_price_m2: 50_000.0,
                avg_monthly_wage_net: 20_000.0,
                months_to_buy: 150.0,
                monthly_rent_60m2: Some(12_000.0),
            },
            AffordabilitySnapshot {
                year: 2015,
                avg_price_m2: 65_000.0,
                avg_monthly_wage_net: 24_000.0,
                months_to_buy: 162.5,
                monthly_rent_60m2: Some(14_000.0),
            },
            AffordabilitySnapshot {
                year: 2020,
                avg_price_m2: 90_000.0,
                avg_monthly_wage_net: 28_000.0,
                months_to_buy: 192.9,
                monthly_rent_60m2: Some(17_000.0),
            },
            AffordabilitySnapshot {
                year: 2025,
                avg_price_m2: 110_000.0,
                avg_monthly_wage_net: 33_000.0,
                months_to_buy: 200.0,
                monthly_rent_60m2: Some(20_000.0),
            },
        ];

        let result = build_affordability_forecast(&snapshots, 5).unwrap();

        // Should have 4 historical + 5 forecast = 9 labels
        assert_eq!(result.labels.len(), 9);
        assert_eq!(result.labels[0], "2010");
        assert_eq!(result.labels[8], "2030");

        // Should have 6 datasets (3 series x 2: historical + forecast)
        assert_eq!(result.datasets.len(), 6);

        // Historical price index should start at 100 (base year)
        assert_eq!(result.datasets[0].data[0], Some(100.0));

        // Forecast datasets should have dashed borders
        assert!(result.datasets[1].border_dash.is_some());
        assert!(result.datasets[3].border_dash.is_some());
        assert!(result.datasets[5].border_dash.is_some());

        // Forecast years-to-buy should continue upward from last actual value
        // Last historical: 200/12 = 16.67 years
        // Recent slope from 2020->2025: (200/12 - 192.9/12) / 5 = 0.1183/yr
        // So 2026 forecast should be > 16.67 (going up, not down)
        let forecast_2026 = result.datasets[5].data[4].unwrap(); // index 4 = first forecast year
        let last_hist = 200.0 / 12.0;
        assert!(forecast_2026 > last_hist, "Forecast should go up: {forecast_2026} > {last_hist}");
    }

    #[test]
    fn test_rent_burden_forecast() {
        let snapshots = vec![
            AffordabilitySnapshot {
                year: 2020,
                avg_price_m2: 90_000.0,
                avg_monthly_wage_net: 28_000.0,
                months_to_buy: 192.9,
                monthly_rent_60m2: Some(17_000.0),
            },
            AffordabilitySnapshot {
                year: 2025,
                avg_price_m2: 110_000.0,
                avg_monthly_wage_net: 33_000.0,
                months_to_buy: 200.0,
                monthly_rent_60m2: Some(20_000.0),
            },
        ];

        let result = build_rent_burden_forecast(&snapshots, 5).unwrap();

        // 2 historical + 5 forecast = 7 labels
        assert_eq!(result.labels.len(), 7);
        assert_eq!(result.datasets.len(), 2);

        // First historical burden: 17000/28000*100 ≈ 60.7%
        let first_burden = result.datasets[0].data[0].unwrap();
        assert!((first_burden - 60.7).abs() < 0.5, "first burden: {first_burden}");

        // Forecast should continue from last actual value (not regressed value)
        let last_hist = result.datasets[0].data[1].unwrap();
        let first_forecast = result.datasets[1].data[1].unwrap();
        assert!(
            (first_forecast - last_hist).abs() < 0.01,
            "Forecast should start at last historical value"
        );
    }
}
