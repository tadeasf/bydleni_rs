//! Deterministic editorial story derivation from affordability data.
//! Pure functions — no async, no DB access.

/// The kind of insight a story represents.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoryKind {
    HistoricalChange,
    WagesVsPrices,
    RentMortgageCrossover,
    ForecastDirection,
    SavingsTimeline,
}

impl StoryKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::HistoricalChange => "Historical",
            Self::WagesVsPrices => "Wages vs Prices",
            Self::RentMortgageCrossover => "Rent vs Mortgage",
            Self::ForecastDirection => "Forecast",
            Self::SavingsTimeline => "Savings",
        }
    }
}

/// Severity level — maps to existing heat-1..heat-5 CSS classes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    Good,
    Moderate,
    Concerning,
    Bad,
    Critical,
}

/// Map severity to CSS heat class.
pub fn severity_to_color(s: Severity) -> &'static str {
    match s {
        Severity::Good => "heat-1",
        Severity::Moderate => "heat-2",
        Severity::Concerning => "heat-3",
        Severity::Bad => "heat-4",
        Severity::Critical => "heat-5",
    }
}

/// A derived story card.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Story {
    pub kind: StoryKind,
    pub severity: Severity,
    pub headline: String,
    pub body: String,
    pub region: String,
    pub region_name: String,
    pub color_class: &'static str,
    pub impact_score: f64,
}

/// Input data for story derivation.
pub struct StoryInput {
    pub region: String,
    pub region_name: String,
    pub snapshots: Vec<SnapshotData>,
}

/// A single historical snapshot for a region.
#[allow(dead_code)]
pub struct SnapshotData {
    pub year: i32,
    pub months_to_buy: Option<f64>,
    pub avg_price_m2: Option<f64>,
    pub avg_monthly_wage_net: Option<f64>,
    pub monthly_rent_60m2: Option<f64>,
    pub monthly_payment_30y: Option<f64>,
    pub rent_vs_mortgage_ratio: Option<f64>,
    pub years_to_save_investing: Option<f64>,
}

/// Compare 2020 vs 2025 months_to_buy.
pub fn derive_historical_change(input: &StoryInput) -> Option<Story> {
    let s2020 = input.snapshots.iter().find(|s| s.year == 2020)?;
    let s2025 = input.snapshots.iter().find(|s| s.year == 2025)?;
    let m2020 = s2020.months_to_buy?;
    let m2025 = s2025.months_to_buy?;

    let change_pct = (m2025 - m2020) / m2020 * 100.0;
    let abs_change = change_pct.abs();

    let (severity, direction) = if change_pct > 20.0 {
        (Severity::Critical, "surged")
    } else if change_pct > 10.0 {
        (Severity::Bad, "worsened significantly")
    } else if change_pct > 0.0 {
        (Severity::Concerning, "worsened slightly")
    } else if change_pct > -10.0 {
        (Severity::Moderate, "improved slightly")
    } else {
        (Severity::Good, "improved significantly")
    };

    let headline = format!(
        "Affordability {} since 2020",
        if change_pct > 0.0 { "worsened" } else { "improved" }
    );
    let body = format!(
        "In {}, it took {:.0} months of net salary to buy a 60m\u{b2} flat in 2020. \
         By 2025, that {} to {:.0} months — a {:.1}% {}.",
        input.region_name,
        m2020,
        direction,
        m2025,
        abs_change,
        if change_pct > 0.0 { "increase" } else { "decrease" }
    );

    Some(Story {
        kind: StoryKind::HistoricalChange,
        severity,
        headline,
        body,
        region: input.region.clone(),
        region_name: input.region_name.clone(),
        color_class: severity_to_color(severity),
        impact_score: abs_change,
    })
}

/// Compare price growth vs wage growth since 2020.
pub fn derive_wages_vs_prices(input: &StoryInput) -> Option<Story> {
    let s2020 = input.snapshots.iter().find(|s| s.year == 2020)?;
    let s2025 = input.snapshots.iter().find(|s| s.year == 2025)?;

    let p2020 = s2020.avg_price_m2?;
    let p2025 = s2025.avg_price_m2?;
    let w2020 = s2020.avg_monthly_wage_net?;
    let w2025 = s2025.avg_monthly_wage_net?;

    if p2020 <= 0.0 || w2020 <= 0.0 {
        return None;
    }

    let price_growth = (p2025 - p2020) / p2020 * 100.0;
    let wage_growth = (w2025 - w2020) / w2020 * 100.0;
    let gap = price_growth - wage_growth;

    let severity = if gap > 30.0 {
        Severity::Critical
    } else if gap > 15.0 {
        Severity::Bad
    } else if gap > 5.0 {
        Severity::Concerning
    } else if gap > -5.0 {
        Severity::Moderate
    } else {
        Severity::Good
    };

    let headline = if gap > 5.0 {
        "Prices outpacing wages".to_string()
    } else if gap < -5.0 {
        "Wages catching up to prices".to_string()
    } else {
        "Prices and wages growing in step".to_string()
    };

    let body = format!(
        "Since 2020, property prices in {} grew {:.0}% while net wages grew {:.0}%. \
         {}",
        input.region_name,
        price_growth,
        wage_growth,
        if gap > 5.0 {
            format!("The {:.0}pp gap means buying power is eroding.", gap)
        } else if gap < -5.0 {
            format!("Wages are closing the gap by {:.0}pp.", gap.abs())
        } else {
            "They're roughly keeping pace.".to_string()
        }
    );

    Some(Story {
        kind: StoryKind::WagesVsPrices,
        severity,
        headline,
        body,
        region: input.region.clone(),
        region_name: input.region_name.clone(),
        color_class: severity_to_color(severity),
        impact_score: gap.abs(),
    })
}

/// Story about rent vs mortgage crossover.
pub fn derive_rent_mortgage_crossover(input: &StoryInput) -> Option<Story> {
    let latest = input.snapshots.last()?;
    let ratio = latest.rent_vs_mortgage_ratio?;

    let (severity, headline, body) = if ratio > 1.2 {
        (
            Severity::Bad,
            "Renting costs more than a mortgage".to_string(),
            format!(
                "In {}, monthly rent for 60m\u{b2} is {:.0}% higher than a mortgage payment. \
                 Buying is cheaper month-to-month — if you can get the down payment.",
                input.region_name,
                (ratio - 1.0) * 100.0
            ),
        )
    } else if ratio > 1.0 {
        (
            Severity::Concerning,
            "Rent slightly exceeds mortgage".to_string(),
            format!(
                "In {}, renting costs about {:.0}% more than a mortgage payment per month.",
                input.region_name,
                (ratio - 1.0) * 100.0
            ),
        )
    } else if ratio > 0.7 {
        (
            Severity::Moderate,
            "Rent and mortgage roughly equal".to_string(),
            format!(
                "In {}, renting costs about {:.0}% of what a mortgage payment would be.",
                input.region_name,
                ratio * 100.0
            ),
        )
    } else {
        (
            Severity::Good,
            "Renting is significantly cheaper".to_string(),
            format!(
                "In {}, rent is only {:.0}% of a typical mortgage payment — \
                 renting and investing the difference could be a better strategy.",
                input.region_name,
                ratio * 100.0
            ),
        )
    };

    // Higher impact for extreme ratios
    let impact = (ratio - 1.0).abs() * 50.0;

    Some(Story {
        kind: StoryKind::RentMortgageCrossover,
        severity,
        headline,
        body,
        region: input.region.clone(),
        region_name: input.region_name.clone(),
        color_class: severity_to_color(severity),
        impact_score: impact,
    })
}

/// Forecast direction based on recent slope of months_to_buy.
pub fn derive_forecast_direction(input: &StoryInput) -> Option<Story> {
    let points: Vec<(f64, f64)> = input
        .snapshots
        .iter()
        .filter_map(|s| s.months_to_buy.map(|m| (s.year as f64, m)))
        .collect();

    if points.len() < 2 {
        return None;
    }

    let (x1, y1) = points[points.len() - 2];
    let (x2, y2) = points[points.len() - 1];
    let dx = x2 - x1;
    if dx.abs() < f64::EPSILON {
        return None;
    }
    let slope = (y2 - y1) / dx; // months per year

    let abs_slope = slope.abs();
    let (severity, headline) = if slope > 3.0 {
        (Severity::Critical, "Affordability deteriorating rapidly")
    } else if slope > 1.0 {
        (Severity::Bad, "Affordability getting worse")
    } else if slope > 0.0 {
        (Severity::Concerning, "Affordability slowly declining")
    } else if slope > -1.0 {
        (Severity::Moderate, "Affordability stabilizing")
    } else {
        (Severity::Good, "Affordability improving")
    };

    let current_months = y2;
    let projected = current_months + slope * 5.0;
    let body = format!(
        "At the current trend in {}, a 60m\u{b2} flat could take {:.0} months of net salary \
         by 2030 (currently {:.0}). That's {:.1} months per year of {}.",
        input.region_name,
        projected.max(0.0),
        current_months,
        abs_slope,
        if slope > 0.0 { "worsening" } else { "improvement" }
    );

    Some(Story {
        kind: StoryKind::ForecastDirection,
        severity,
        headline: headline.to_string(),
        body,
        region: input.region.clone(),
        region_name: input.region_name.clone(),
        color_class: severity_to_color(severity),
        impact_score: abs_slope * 5.0,
    })
}

/// Story about savings timeline.
pub fn derive_savings_timeline(input: &StoryInput) -> Option<Story> {
    let latest = input.snapshots.last()?;
    let years = latest.years_to_save_investing?;

    if years.is_infinite() || years.is_nan() || years <= 0.0 {
        return Some(Story {
            kind: StoryKind::SavingsTimeline,
            severity: Severity::Critical,
            headline: "Saving for a flat may be impossible".to_string(),
            body: format!(
                "In {}, the average worker cannot save enough after expenses \
                 to ever accumulate a down payment through investing alone.",
                input.region_name
            ),
            region: input.region.clone(),
            region_name: input.region_name.clone(),
            color_class: severity_to_color(Severity::Critical),
            impact_score: 100.0,
        });
    }

    let severity = if years > 20.0 {
        Severity::Critical
    } else if years > 15.0 {
        Severity::Bad
    } else if years > 10.0 {
        Severity::Concerning
    } else if years > 6.0 {
        Severity::Moderate
    } else {
        Severity::Good
    };

    let headline = if years > 15.0 {
        format!("{:.0} years to save — a generational challenge", years)
    } else if years > 10.0 {
        format!("{:.0} years of saving needed", years)
    } else {
        format!("{:.0} years to ownership is achievable", years)
    };

    let body = format!(
        "Investing savings at 7% annual return, the average worker in {} \
         would need {:.1} years to accumulate a 20% down payment for a 60m\u{b2} flat.",
        input.region_name, years
    );

    Some(Story {
        kind: StoryKind::SavingsTimeline,
        severity,
        headline,
        body,
        region: input.region.clone(),
        region_name: input.region_name.clone(),
        color_class: severity_to_color(severity),
        impact_score: years.min(50.0),
    })
}

/// Run all 5 derivations, filter None, sort by impact_score DESC.
pub fn derive_stories(input: &StoryInput) -> Vec<Story> {
    let derivations: Vec<fn(&StoryInput) -> Option<Story>> = vec![
        derive_historical_change,
        derive_wages_vs_prices,
        derive_rent_mortgage_crossover,
        derive_forecast_direction,
        derive_savings_timeline,
    ];

    let mut stories: Vec<Story> = derivations.into_iter().filter_map(|f| f(input)).collect();
    stories.sort_by(|a, b| b.impact_score.partial_cmp(&a.impact_score).unwrap());
    stories
}

/// Select top N stories across all regions by impact_score.
pub fn top_stories(all: &[Story], n: usize) -> Vec<Story> {
    let mut sorted: Vec<Story> = all.to_vec();
    sorted.sort_by(|a, b| b.impact_score.partial_cmp(&a.impact_score).unwrap());
    sorted.truncate(n);
    sorted
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_input(region: &str, snapshots: Vec<SnapshotData>) -> StoryInput {
        StoryInput { region: region.to_string(), region_name: region.to_string(), snapshots }
    }

    #[allow(clippy::too_many_arguments)]
    fn snap(
        year: i32,
        months: f64,
        price: f64,
        wage: f64,
        rent: Option<f64>,
        payment: Option<f64>,
        ratio: Option<f64>,
        years_save: Option<f64>,
    ) -> SnapshotData {
        SnapshotData {
            year,
            months_to_buy: Some(months),
            avg_price_m2: Some(price),
            avg_monthly_wage_net: Some(wage),
            monthly_rent_60m2: rent,
            monthly_payment_30y: payment,
            rent_vs_mortgage_ratio: ratio,
            years_to_save_investing: years_save,
        }
    }

    // --- derive_historical_change ---

    #[test]
    fn historical_change_worsened() {
        let input = make_input(
            "praha",
            vec![
                snap(2020, 150.0, 80000.0, 30000.0, None, None, None, None),
                snap(2025, 200.0, 110000.0, 35000.0, None, None, None, None),
            ],
        );
        let story = derive_historical_change(&input).unwrap();
        assert_eq!(story.severity, Severity::Critical); // 33% increase
        assert!(story.headline.contains("worsened"));
    }

    #[test]
    fn historical_change_improved() {
        let input = make_input(
            "liberec",
            vec![
                snap(2020, 150.0, 80000.0, 30000.0, None, None, None, None),
                snap(2025, 120.0, 70000.0, 35000.0, None, None, None, None),
            ],
        );
        let story = derive_historical_change(&input).unwrap();
        assert_eq!(story.severity, Severity::Good); // -20%
        assert!(story.headline.contains("improved"));
    }

    #[test]
    fn historical_change_missing_year() {
        let input = make_input(
            "brno",
            vec![
                snap(2015, 100.0, 50000.0, 25000.0, None, None, None, None),
                snap(2025, 180.0, 100000.0, 35000.0, None, None, None, None),
            ],
        );
        assert!(derive_historical_change(&input).is_none());
    }

    // --- derive_wages_vs_prices ---

    #[test]
    fn wages_vs_prices_gap() {
        let input = make_input(
            "praha",
            vec![
                snap(2020, 150.0, 80000.0, 30000.0, None, None, None, None),
                snap(2025, 200.0, 120000.0, 33000.0, None, None, None, None),
            ],
        );
        let story = derive_wages_vs_prices(&input).unwrap();
        // price +50%, wage +10% => gap 40pp
        assert_eq!(story.severity, Severity::Critical);
        assert!(story.headline.contains("outpacing"));
    }

    #[test]
    fn wages_catching_up() {
        let input = make_input(
            "liberec",
            vec![
                snap(2020, 150.0, 80000.0, 25000.0, None, None, None, None),
                snap(2025, 120.0, 84000.0, 35000.0, None, None, None, None),
            ],
        );
        let story = derive_wages_vs_prices(&input).unwrap();
        // price +5%, wage +40% => gap -35pp
        assert_eq!(story.severity, Severity::Good);
        assert!(story.headline.contains("catching up"));
    }

    #[test]
    fn wages_vs_prices_no_data() {
        let input =
            make_input("x", vec![snap(2025, 100.0, 50000.0, 25000.0, None, None, None, None)]);
        assert!(derive_wages_vs_prices(&input).is_none());
    }

    // --- derive_rent_mortgage_crossover ---

    #[test]
    fn rent_exceeds_mortgage() {
        let input = make_input(
            "praha",
            vec![snap(
                2025,
                200.0,
                110000.0,
                35000.0,
                Some(22000.0),
                Some(15000.0),
                Some(1.3),
                None,
            )],
        );
        let story = derive_rent_mortgage_crossover(&input).unwrap();
        assert_eq!(story.severity, Severity::Bad);
        assert!(story.headline.contains("more than"));
    }

    #[test]
    fn rent_cheaper() {
        let input = make_input(
            "ostrava",
            vec![snap(2025, 80.0, 40000.0, 30000.0, Some(8000.0), Some(14000.0), Some(0.57), None)],
        );
        let story = derive_rent_mortgage_crossover(&input).unwrap();
        assert_eq!(story.severity, Severity::Good);
        assert!(story.headline.contains("cheaper"));
    }

    #[test]
    fn rent_mortgage_no_ratio() {
        let input =
            make_input("x", vec![snap(2025, 100.0, 50000.0, 25000.0, None, None, None, None)]);
        assert!(derive_rent_mortgage_crossover(&input).is_none());
    }

    // --- derive_forecast_direction ---

    #[test]
    fn forecast_worsening() {
        let input = make_input(
            "praha",
            vec![
                snap(2020, 150.0, 80000.0, 30000.0, None, None, None, None),
                snap(2025, 200.0, 110000.0, 35000.0, None, None, None, None),
            ],
        );
        let story = derive_forecast_direction(&input).unwrap();
        // slope = (200-150)/5 = 10 months/year
        assert_eq!(story.severity, Severity::Critical);
        assert!(story.headline.contains("rapidly"));
    }

    #[test]
    fn forecast_improving() {
        let input = make_input(
            "liberec",
            vec![
                snap(2020, 150.0, 80000.0, 30000.0, None, None, None, None),
                snap(2025, 140.0, 75000.0, 35000.0, None, None, None, None),
            ],
        );
        let story = derive_forecast_direction(&input).unwrap();
        // slope = -2/year
        assert_eq!(story.severity, Severity::Good);
        assert!(story.headline.contains("improving"));
    }

    #[test]
    fn forecast_insufficient_data() {
        let input =
            make_input("x", vec![snap(2025, 100.0, 50000.0, 25000.0, None, None, None, None)]);
        assert!(derive_forecast_direction(&input).is_none());
    }

    // --- derive_savings_timeline ---

    #[test]
    fn savings_impossible() {
        let input = make_input(
            "praha",
            vec![snap(2025, 200.0, 110000.0, 35000.0, None, None, None, Some(f64::INFINITY))],
        );
        let story = derive_savings_timeline(&input).unwrap();
        assert_eq!(story.severity, Severity::Critical);
        assert!(story.headline.contains("impossible"));
    }

    #[test]
    fn savings_long() {
        let input = make_input(
            "brno",
            vec![snap(2025, 160.0, 90000.0, 33000.0, None, None, None, Some(18.0))],
        );
        let story = derive_savings_timeline(&input).unwrap();
        assert_eq!(story.severity, Severity::Bad);
    }

    #[test]
    fn savings_achievable() {
        let input = make_input(
            "ostrava",
            vec![snap(2025, 80.0, 40000.0, 30000.0, None, None, None, Some(5.0))],
        );
        let story = derive_savings_timeline(&input).unwrap();
        assert_eq!(story.severity, Severity::Good);
        assert!(story.headline.contains("achievable"));
    }

    // --- derive_stories & top_stories ---

    #[test]
    fn derive_all_stories() {
        let input = make_input(
            "praha",
            vec![
                snap(
                    2020,
                    150.0,
                    80000.0,
                    30000.0,
                    Some(15000.0),
                    Some(12000.0),
                    Some(1.25),
                    Some(12.0),
                ),
                snap(
                    2025,
                    200.0,
                    110000.0,
                    35000.0,
                    Some(22000.0),
                    Some(18000.0),
                    Some(1.22),
                    Some(18.0),
                ),
            ],
        );
        let stories = derive_stories(&input);
        assert!(stories.len() >= 4); // all 5 should fire with this data
        // Should be sorted by impact_score DESC
        for w in stories.windows(2) {
            assert!(w[0].impact_score >= w[1].impact_score);
        }
    }

    #[test]
    fn top_stories_limits() {
        let input = make_input(
            "praha",
            vec![
                snap(
                    2020,
                    150.0,
                    80000.0,
                    30000.0,
                    Some(15000.0),
                    Some(12000.0),
                    Some(1.25),
                    Some(12.0),
                ),
                snap(
                    2025,
                    200.0,
                    110000.0,
                    35000.0,
                    Some(22000.0),
                    Some(18000.0),
                    Some(1.22),
                    Some(18.0),
                ),
            ],
        );
        let stories = derive_stories(&input);
        let top = top_stories(&stories, 2);
        assert_eq!(top.len(), 2);
        assert!(top[0].impact_score >= top[1].impact_score);
    }

    #[test]
    fn top_stories_empty() {
        let top = top_stories(&[], 5);
        assert!(top.is_empty());
    }
}
