//! Methodology transparency layer — inline disclosure notes for key metrics.
//!
//! Each [`MethodologyNote`] renders as a `<details class="mcard-explain">` element
//! that visitors can expand to see how a metric was calculated, what data sources
//! were used, and when the data was last refreshed.

/// A single expandable disclosure.
pub struct MethodologyNote {
    pub summary: String,
    pub body: String,
}

impl MethodologyNote {
    /// Render to an HTML `<details>` element.
    pub fn to_html(&self) -> String {
        format!(
            "<details class=\"mcard-explain\"><summary>{}</summary><p>{}</p></details>",
            self.summary, self.body
        )
    }
}

/// Disclosures for the index (landing) page.
pub struct IndexMethodology {
    pub years_to_buy: String,
    pub forecast: String,
    pub rent_burden: String,
}

/// Disclosures for the region detail page.
pub struct RegionMethodology {
    pub months_to_buy: String,
    pub price_m2: String,
    pub mortgage: String,
    pub rent_vs_mortgage: String,
    pub savings: String,
}

fn freshness_line(last_refresh: &str) -> String {
    if last_refresh.is_empty() {
        String::new()
    } else {
        format!("<br><em>Last data refresh: {last_refresh}.</em>")
    }
}

/// Build all methodology disclosures for the index page.
pub fn build_index_methodology(last_refresh: &str) -> IndexMethodology {
    let fresh = freshness_line(last_refresh);

    let years_to_buy = MethodologyNote {
        summary: "How this is calculated".into(),
        body: format!(
            "Years = (avg_price_m\u{b2} \u{d7} 60) \u{f7} (net_monthly_wage \u{d7} 12). \
             Net wage uses Czech 2025 tax rules: 12.2% social/health insurance + \
             15%/23% income tax brackets, minus 2,570 CZK/month personal tax credit. \
             Sources: CZSO (wages by region), Sreality (asking prices).{fresh}"
        ),
    };

    let forecast = MethodologyNote {
        summary: "About this forecast".into(),
        body: format!(
            "Price and wage indices are extrapolated 5 years forward using the slope \
             between the two most recent data points (recent-slope method). \
             This is a trend projection, not a prediction \u{2014} it assumes recent \
             momentum continues unchanged. Sources: FRED (property price index), \
             CZSO (wage index).{fresh}"
        ),
    };

    let rent_burden = MethodologyNote {
        summary: "About this forecast".into(),
        body: format!(
            "Rent burden = (monthly_rent_60m\u{b2} \u{f7} net_monthly_wage) \u{d7} 100%. \
             Projected via recent-slope extrapolation of the last two data points over \
             5 years. Sources: Sreality (rent listings), CZSO (wages).{fresh}"
        ),
    };

    IndexMethodology {
        years_to_buy: years_to_buy.to_html(),
        forecast: forecast.to_html(),
        rent_burden: rent_burden.to_html(),
    }
}

/// Build all methodology disclosures for a region detail page.
pub fn build_region_methodology(last_refresh: &str) -> RegionMethodology {
    let fresh = freshness_line(last_refresh);

    let months_to_buy = MethodologyNote {
        summary: "How this is calculated".into(),
        body: format!(
            "Months = (avg_price_m\u{b2} \u{d7} 60) \u{f7} net_monthly_wage. \
             Net wage uses Czech 2025 tax rules: 12.2% social/health insurance + \
             15%/23% income tax brackets, minus 2,570 CZK/month personal tax credit. \
             Gross-salary variant shown for comparison. \
             Sources: CZSO (wages by kraj), Sreality (asking prices).{fresh}"
        ),
    };

    let price_m2 = MethodologyNote {
        summary: "About this data".into(),
        body: format!(
            "Average asking price per m\u{b2} from current Sreality listings \
             for the region. National figure = average across all tracked regions. \
             Source: Sreality.{fresh}"
        ),
    };

    let mortgage = MethodologyNote {
        summary: "How this is calculated".into(),
        body: format!(
            "Standard amortization formula: M = P[r(1+r)\u{207f}] \u{f7} [(1+r)\u{207f}\u{2212}1]. \
             Defaults: 80% LTV, 30-year term. Interest rate from CNB MFI survey \
             (mortgage_rate_avg); if unavailable, falls back to CNB 2-week repo rate \
             + 2.5 percentage-point spread. Source: CNB.{fresh}"
        ),
    };

    let rent_vs_mortgage = MethodologyNote {
        summary: "How this is calculated".into(),
        body: format!(
            "Ratio = (avg_rent_m\u{b2} \u{d7} 60) \u{f7} monthly_mortgage_payment. \
             Values above 1.0 mean renting is more expensive than the mortgage payment. \
             Sources: Sreality (rent listings), CNB (mortgage rates).{fresh}"
        ),
    };

    let savings = MethodologyNote {
        summary: "How this is calculated".into(),
        body: format!(
            "Monthly savings = net_wage \u{2212} living_expenses. Living expenses \
             use CZSO consumer-basket data, region-specific where available. \
             Years to save for a 20% down payment via compound annuity formula \
             at 7% annual investment return. Source: CZSO.{fresh}"
        ),
    };

    RegionMethodology {
        months_to_buy: months_to_buy.to_html(),
        price_m2: price_m2.to_html(),
        mortgage: mortgage.to_html(),
        rent_vs_mortgage: rent_vs_mortgage.to_html(),
        savings: savings.to_html(),
    }
}

/// Build methodology disclosure for the compare page.
pub fn build_compare_methodology(last_refresh: &str) -> MethodologyNote {
    let fresh = freshness_line(last_refresh);
    MethodologyNote {
        summary: "How these numbers are calculated".into(),
        body: format!(
            "Years of net salary = (avg_price_m\u{b2} \u{d7} 60) \u{f7} (net_monthly_wage \u{d7} 12). \
             Net wage uses Czech 2025 tax: 12.2% insurance + 15%/23% tax \u{2212} 2,570 CZK credit. \
             Prices are average asking prices from Sreality. Wages from CZSO, mapped \
             by kraj (region). All regions ranked highest (least affordable) first. \
             Sources: CZSO, Sreality.{fresh}"
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn to_html_produces_details_element() {
        let note = MethodologyNote { summary: "Test summary".into(), body: "Test body".into() };
        let html = note.to_html();
        assert!(html.starts_with("<details class=\"mcard-explain\">"));
        assert!(html.contains("<summary>Test summary</summary>"));
        assert!(html.contains("<p>Test body</p>"));
        assert!(html.ends_with("</details>"));
    }

    #[test]
    fn index_methodology_fields_non_empty() {
        let m = build_index_methodology("");
        assert!(!m.years_to_buy.is_empty());
        assert!(!m.forecast.is_empty());
        assert!(!m.rent_burden.is_empty());
    }

    #[test]
    fn region_methodology_fields_non_empty() {
        let m = build_region_methodology("");
        assert!(!m.months_to_buy.is_empty());
        assert!(!m.price_m2.is_empty());
        assert!(!m.mortgage.is_empty());
        assert!(!m.rent_vs_mortgage.is_empty());
        assert!(!m.savings.is_empty());
    }

    #[test]
    fn freshness_appears_when_provided() {
        let m = build_index_methodology("2025-06-01 12:00");
        assert!(m.years_to_buy.contains("2025-06-01 12:00"));
    }

    #[test]
    fn freshness_omitted_when_empty() {
        let m = build_index_methodology("");
        assert!(!m.years_to_buy.contains("Last data refresh"));
    }

    #[test]
    fn sources_mentioned_in_index() {
        let m = build_index_methodology("");
        assert!(m.years_to_buy.contains("CZSO"));
        assert!(m.years_to_buy.contains("Sreality"));
        assert!(m.forecast.contains("FRED"));
        assert!(m.rent_burden.contains("Sreality"));
    }

    #[test]
    fn sources_mentioned_in_region() {
        let m = build_region_methodology("");
        assert!(m.months_to_buy.contains("CZSO"));
        assert!(m.price_m2.contains("Sreality"));
        assert!(m.mortgage.contains("CNB"));
        assert!(m.rent_vs_mortgage.contains("CNB"));
        assert!(m.savings.contains("CZSO"));
    }

    #[test]
    fn compare_methodology_populated() {
        let note = build_compare_methodology("2025-01-01");
        let html = note.to_html();
        assert!(html.contains("CZSO"));
        assert!(html.contains("Sreality"));
        assert!(html.contains("2025-01-01"));
    }
}
