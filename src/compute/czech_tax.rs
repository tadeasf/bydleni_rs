//! Czech 2025/2026 employee tax deduction calculator.
//!
//! Formula:
//! - Social insurance: 7.1%, Health insurance: 4.5%, Sickness: 0.6% (total employee: 12.2%)
//! - Income tax: 15% up to CZK 1,676,052/yr (139,671/mo), 23% above
//! - Personal tax credit: CZK 30,840/yr (2,570/mo)
//! - Net = gross - insurance - max(tax - credit, 0)

const INSURANCE_RATE: f64 = 0.122; // 7.1% social + 4.5% health + 0.6% sickness
const TAX_RATE_LOW: f64 = 0.15;
const TAX_RATE_HIGH: f64 = 0.23;
const MONTHLY_BRACKET_LIMIT: f64 = 139_671.0; // CZK 1,676,052 / 12
const MONTHLY_CREDIT: f64 = 2_570.0; // CZK 30,840 / 12

/// Convert gross monthly wage to net monthly wage using Czech 2025 tax rules.
pub fn gross_to_net_monthly(gross: f64) -> f64 {
    if gross <= 0.0 {
        return 0.0;
    }

    let insurance = gross * INSURANCE_RATE;

    // Tax base = gross (super-gross was abolished in 2021)
    let tax = if gross <= MONTHLY_BRACKET_LIMIT {
        gross * TAX_RATE_LOW
    } else {
        MONTHLY_BRACKET_LIMIT * TAX_RATE_LOW + (gross - MONTHLY_BRACKET_LIMIT) * TAX_RATE_HIGH
    };

    let tax_after_credit = (tax - MONTHLY_CREDIT).max(0.0);

    gross - insurance - tax_after_credit
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_typical_wage() {
        // 45,000 gross:
        // insurance: 45000 * 0.122 = 5490
        // tax: 45000 * 0.15 = 6750
        // tax after credit: 6750 - 2570 = 4180
        // net: 45000 - 5490 - 4180 = 35330
        let net = gross_to_net_monthly(45_000.0);
        assert!((net - 35_330.0).abs() < 10.0, "45k gross should yield ~35,330 net, got {net:.0}");
    }

    #[test]
    fn test_high_wage_23pct_bracket() {
        // 150,000 gross -> tests the 23% bracket
        let net = gross_to_net_monthly(150_000.0);
        // insurance: 150000 * 0.122 = 18300
        // tax: 139671 * 0.15 + (150000 - 139671) * 0.23 = 20950.65 + 2375.67 = 23326.32
        // tax after credit: 23326.32 - 2570 = 20756.32
        // net: 150000 - 18300 - 20756.32 = 110943.68
        assert!(
            (net - 110_944.0).abs() < 100.0,
            "150k gross should yield ~110,944 net, got {net:.0}"
        );
    }

    #[test]
    fn test_minimum_wage() {
        // 20,000 gross -> credit may exceed tax
        let net = gross_to_net_monthly(20_000.0);
        // insurance: 20000 * 0.122 = 2440
        // tax: 20000 * 0.15 = 3000
        // tax after credit: max(3000 - 2570, 0) = 430
        // net: 20000 - 2440 - 430 = 17130
        assert!((net - 17_130.0).abs() < 50.0, "20k gross should yield ~17,130 net, got {net:.0}");
    }

    #[test]
    fn test_zero_wage() {
        assert_eq!(gross_to_net_monthly(0.0), 0.0);
        assert_eq!(gross_to_net_monthly(-1000.0), 0.0);
    }
}
