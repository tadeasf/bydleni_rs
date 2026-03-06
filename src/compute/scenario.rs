use serde::{Deserialize, Serialize};

use crate::compute::affordability::{mortgage_monthly_payment, years_to_save_with_investment};
use crate::server::templates::slug_to_display_name;

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct ScenarioInput {
    pub net_income: f64,
    #[serde(default = "default_savings")]
    pub current_savings: f64,
    #[serde(default = "default_flat_size")]
    pub flat_size_m2: f64,
    #[serde(default = "default_mortgage_rate")]
    pub mortgage_rate_pct: f64,
    #[serde(default = "default_ltv")]
    pub ltv_pct: f64,
    #[serde(default = "default_mortgage_years")]
    pub mortgage_years: u32,
    #[serde(default = "default_expenses")]
    pub monthly_expenses: f64,
    #[serde(default = "default_investment_return")]
    pub investment_return_pct: Option<f64>,
}

fn default_savings() -> f64 {
    0.0
}
fn default_flat_size() -> f64 {
    60.0
}
fn default_mortgage_rate() -> f64 {
    5.0
}
fn default_ltv() -> f64 {
    80.0
}
fn default_mortgage_years() -> u32 {
    30
}
fn default_expenses() -> f64 {
    17_000.0
}
fn default_investment_return() -> Option<f64> {
    Some(7.0)
}

#[derive(Serialize, Clone, Debug)]
pub struct ScenarioResult {
    pub region: String,
    pub region_name: String,
    pub flat_price: f64,
    pub deposit_needed: f64,
    pub deposit_gap: f64,
    pub mortgage_principal: f64,
    pub monthly_payment: f64,
    pub payment_to_income_pct: f64,
    pub monthly_surplus: f64,
    pub months_to_deposit: Option<f64>,
    pub years_to_deposit: Option<f64>,
    pub affordable: bool,
    pub color_class: &'static str,
}

impl ScenarioInput {
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();
        if self.net_income <= 0.0 {
            errors.push("Net income must be positive".to_string());
        }
        if self.current_savings < 0.0 {
            errors.push("Savings cannot be negative".to_string());
        }
        if self.flat_size_m2 <= 0.0 || self.flat_size_m2 > 500.0 {
            errors.push("Flat size must be between 1 and 500 m\u{b2}".to_string());
        }
        if self.mortgage_rate_pct < 0.0 || self.mortgage_rate_pct > 30.0 {
            errors.push("Mortgage rate must be between 0% and 30%".to_string());
        }
        if self.ltv_pct <= 0.0 || self.ltv_pct > 100.0 {
            errors.push("LTV must be between 1% and 100%".to_string());
        }
        if self.mortgage_years == 0 || self.mortgage_years > 50 {
            errors.push("Mortgage term must be between 1 and 50 years".to_string());
        }
        if self.monthly_expenses < 0.0 {
            errors.push("Monthly expenses cannot be negative".to_string());
        }
        if let Some(r) = self.investment_return_pct
            && !(0.0..=50.0).contains(&r)
        {
            errors.push("Investment return must be between 0% and 50%".to_string());
        }
        if errors.is_empty() { Ok(()) } else { Err(errors) }
    }
}

pub fn compute_scenario_for_region(
    input: &ScenarioInput,
    region_slug: &str,
    avg_price_m2: f64,
) -> ScenarioResult {
    let flat_price = avg_price_m2 * input.flat_size_m2;
    let deposit_needed = flat_price * (1.0 - input.ltv_pct / 100.0);
    let deposit_gap = (deposit_needed - input.current_savings).max(0.0);
    let mortgage_principal = flat_price * input.ltv_pct / 100.0;
    let months = input.mortgage_years * 12;
    let monthly_payment =
        mortgage_monthly_payment(mortgage_principal, input.mortgage_rate_pct, months);
    let payment_to_income_pct = if input.net_income > 0.0 {
        monthly_payment / input.net_income * 100.0
    } else {
        f64::INFINITY
    };
    let monthly_surplus = (input.net_income - input.monthly_expenses).max(0.0);
    let return_pct = input.investment_return_pct.unwrap_or(7.0);

    let (months_to_deposit, years_to_deposit) = if deposit_gap <= 0.0 {
        (Some(0.0), Some(0.0))
    } else if monthly_surplus <= 0.0 {
        (None, None)
    } else {
        let years = years_to_save_with_investment(deposit_gap, monthly_surplus, return_pct);
        if years.is_infinite() { (None, None) } else { (Some(years * 12.0), Some(years)) }
    };

    let affordable = payment_to_income_pct < 40.0;
    let color_class = scenario_severity_color(years_to_deposit);

    ScenarioResult {
        region: region_slug.to_string(),
        region_name: slug_to_display_name(region_slug),
        flat_price,
        deposit_needed,
        deposit_gap,
        mortgage_principal,
        monthly_payment,
        payment_to_income_pct,
        monthly_surplus,
        months_to_deposit,
        years_to_deposit,
        affordable,
        color_class,
    }
}

pub fn compute_scenario_all_regions(
    input: &ScenarioInput,
    region_prices: &[(String, f64)],
) -> Vec<ScenarioResult> {
    let mut results: Vec<ScenarioResult> = region_prices
        .iter()
        .map(|(slug, price)| compute_scenario_for_region(input, slug, *price))
        .collect();
    results.sort_by(|a, b| {
        let ya = a.years_to_deposit.unwrap_or(f64::MAX);
        let yb = b.years_to_deposit.unwrap_or(f64::MAX);
        ya.partial_cmp(&yb).unwrap_or(std::cmp::Ordering::Equal)
    });
    results
}

pub fn scenario_severity_color(years: Option<f64>) -> &'static str {
    match years {
        None => "heat-5",
        Some(y) if y > 15.0 => "heat-5",
        Some(y) if y > 10.0 => "heat-4",
        Some(y) if y > 6.0 => "heat-3",
        Some(y) if y > 3.0 => "heat-2",
        Some(_) => "heat-1",
    }
}

pub fn scenario_summary(input: &ScenarioInput) -> String {
    let income_k = input.net_income / 1000.0;
    let savings_k = input.current_savings / 1000.0;
    format!(
        "{income_k:.0}k CZK income | {savings_k:.0}k saved | {size:.0} m\u{b2} | {ltv:.0}% LTV {term}y",
        size = input.flat_size_m2,
        ltv = input.ltv_pct,
        term = input.mortgage_years,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn typical_input() -> ScenarioInput {
        ScenarioInput {
            net_income: 52_000.0,
            current_savings: 500_000.0,
            flat_size_m2: 60.0,
            mortgage_rate_pct: 5.0,
            ltv_pct: 80.0,
            mortgage_years: 30,
            monthly_expenses: 17_000.0,
            investment_return_pct: Some(7.0),
        }
    }

    #[test]
    fn test_valid_input() {
        assert!(typical_input().validate().is_ok());
    }

    #[test]
    fn test_invalid_negative_income() {
        let mut input = typical_input();
        input.net_income = -1.0;
        let err = input.validate().unwrap_err();
        assert!(err.iter().any(|e| e.contains("income")));
    }

    #[test]
    fn test_invalid_ltv_over_100() {
        let mut input = typical_input();
        input.ltv_pct = 110.0;
        let err = input.validate().unwrap_err();
        assert!(err.iter().any(|e| e.contains("LTV")));
    }

    #[test]
    fn test_invalid_zero_mortgage_years() {
        let mut input = typical_input();
        input.mortgage_years = 0;
        let err = input.validate().unwrap_err();
        assert!(err.iter().any(|e| e.contains("term")));
    }

    #[test]
    fn test_typical_scenario() {
        let input = typical_input();
        let result = compute_scenario_for_region(&input, "praha", 120_000.0);
        // flat_price = 120k * 60 = 7.2M
        assert!((result.flat_price - 7_200_000.0).abs() < 0.01);
        // deposit = 7.2M * 0.2 = 1.44M
        assert!((result.deposit_needed - 1_440_000.0).abs() < 0.01);
        // gap = 1.44M - 500k = 940k
        assert!((result.deposit_gap - 940_000.0).abs() < 0.01);
        // principal = 7.2M * 0.8 = 5.76M
        assert!((result.mortgage_principal - 5_760_000.0).abs() < 0.01);
        // payment should be ~30k-ish for 5.76M at 5% over 30y
        assert!(result.monthly_payment > 25_000.0 && result.monthly_payment < 40_000.0);
        assert_eq!(result.region_name, "Praha");
    }

    #[test]
    fn test_zero_deposit_gap() {
        let mut input = typical_input();
        input.current_savings = 2_000_000.0;
        let result = compute_scenario_for_region(&input, "brno", 60_000.0);
        // flat_price = 3.6M, deposit = 720k, savings = 2M > 720k
        assert!(result.deposit_gap < 0.01);
        assert_eq!(result.years_to_deposit, Some(0.0));
    }

    #[test]
    fn test_zero_surplus() {
        let mut input = typical_input();
        input.monthly_expenses = 60_000.0;
        let result = compute_scenario_for_region(&input, "praha", 120_000.0);
        assert_eq!(result.monthly_surplus, 0.0);
        assert!(result.months_to_deposit.is_none());
        assert!(result.years_to_deposit.is_none());
    }

    #[test]
    fn test_investment_return_effect() {
        let input = typical_input();
        let r7 = compute_scenario_for_region(&input, "praha", 100_000.0);

        let mut input_no_return = typical_input();
        input_no_return.investment_return_pct = Some(0.0);
        let r0 = compute_scenario_for_region(&input_no_return, "praha", 100_000.0);

        // With 7% return, should take fewer years than 0% return
        assert!(r7.years_to_deposit.unwrap() < r0.years_to_deposit.unwrap());
    }

    #[test]
    fn test_all_regions_sorting() {
        let input = typical_input();
        let prices = vec![
            ("praha".to_string(), 120_000.0),
            ("brno".to_string(), 70_000.0),
            ("ostrava".to_string(), 40_000.0),
        ];
        let results = compute_scenario_all_regions(&input, &prices);
        // Should be sorted ascending by years_to_deposit
        let years: Vec<f64> = results.iter().map(|r| r.years_to_deposit.unwrap()).collect();
        for w in years.windows(2) {
            assert!(w[0] <= w[1]);
        }
        assert_eq!(results[0].region, "ostrava");
    }

    #[test]
    fn test_summary_format() {
        let input = typical_input();
        let s = scenario_summary(&input);
        assert!(s.contains("52k"));
        assert!(s.contains("500k"));
        assert!(s.contains("60 m\u{b2}"));
        assert!(s.contains("80% LTV"));
        assert!(s.contains("30y"));
    }

    #[test]
    fn test_severity_colors() {
        assert_eq!(scenario_severity_color(None), "heat-5");
        assert_eq!(scenario_severity_color(Some(20.0)), "heat-5");
        assert_eq!(scenario_severity_color(Some(12.0)), "heat-4");
        assert_eq!(scenario_severity_color(Some(8.0)), "heat-3");
        assert_eq!(scenario_severity_color(Some(4.0)), "heat-2");
        assert_eq!(scenario_severity_color(Some(1.0)), "heat-1");
    }
}
