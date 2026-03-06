use askama::Template;
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};

/// Wrapper to make any askama Template implement IntoResponse.
pub struct HtmlTemplate<T: Template>(pub T);

impl<T: Template> IntoResponse for HtmlTemplate<T> {
    fn into_response(self) -> Response {
        match self.0.render() {
            Ok(html) => Html(html).into_response(),
            Err(e) => {
                tracing::error!("Template render error: {e}");
                (StatusCode::INTERNAL_SERVER_ERROR, "Template error").into_response()
            }
        }
    }
}

/// Display-ready region card for the index page.
#[derive(Debug, Clone, serde::Serialize)]
pub struct RegionCard {
    pub slug: String,
    pub name: String,
    pub years: String,
    pub years_raw: f64,
    pub price_m2: String,
    pub wage: String,
    pub color_class: &'static str,
}

/// Landing page template.
#[derive(Template)]
#[template(path = "index.html")]
pub struct IndexTemplate {
    pub regions: Vec<RegionCard>,
    pub regions_json: String,
    pub refreshing: bool,
    pub last_refresh: String,
    pub explain_years: String,
    pub explain_forecast: String,
    pub explain_rent_burden: String,
    pub top_stories: Vec<StoryView>,
}

/// Display-ready example listing for templates.
#[derive(Debug, Clone)]
pub struct ListingView {
    pub name: String,
    pub price: String,
    pub area: String,
    pub price_per_m2: String,
    pub url: String,
}

/// Region detail page template.
#[derive(Template)]
#[template(path = "pages/region.html")]
pub struct RegionTemplate {
    pub slug: String,
    pub name: String,
    pub months_to_buy: String,
    pub months_to_buy_gross: String,
    pub price_m2: String,
    pub flat_price: String,
    pub wage: String,
    pub wage_net: String,
    pub mortgage_rate: String,
    pub monthly_payment: String,
    pub payment_to_wage: String,
    #[allow(dead_code)]
    pub rent_m2: String,
    pub monthly_rent: String,
    pub rent_vs_mortgage: String,
    pub living_expenses: String,
    pub living_expenses_raw: f64,
    pub monthly_savings: String,
    pub years_to_save: String,
    pub wage_net_raw: f64,
    pub flat_price_raw: f64,
    pub mortgage_rate_raw: f64,
    pub refreshing: bool,
    pub stories: Vec<StoryView>,
    pub sale_listings: Vec<ListingView>,
    pub rent_listings: Vec<ListingView>,
    pub sale_median_price_m2: String,
    pub rent_median_price_m2: String,
    pub explain_months: String,
    pub explain_price: String,
    pub explain_mortgage: String,
    pub explain_rent: String,
    pub explain_savings: String,
}

/// Compare page template.
#[derive(Template)]
#[template(path = "pages/compare.html")]
pub struct CompareTemplate {
    pub regions: Vec<RegionCard>,
    pub refreshing: bool,
    pub explain_comparison: String,
}

/// Display-ready story card for templates.
#[derive(Debug, Clone)]
pub struct StoryView {
    pub headline: String,
    pub body: String,
    pub region_name: String,
    pub region_slug: String,
    pub color_class: String,
    pub kind_label: String,
}

/// Convert a compute::stories::Story to a StoryView.
pub fn story_to_view(story: &crate::compute::stories::Story) -> StoryView {
    StoryView {
        headline: story.headline.clone(),
        body: story.body.clone(),
        region_name: story.region_name.clone(),
        region_slug: story.region.clone(),
        color_class: story.color_class.to_string(),
        kind_label: story.kind.label().to_string(),
    }
}

/// Stories page template.
#[derive(Template)]
#[template(path = "pages/stories.html")]
pub struct StoriesTemplate {
    pub national_stories: Vec<StoryView>,
    pub top_stories: Vec<StoryView>,
    pub region_groups: Vec<RegionStoryGroup>,
    pub refreshing: bool,
}

/// A group of stories for one region.
#[derive(Debug, Clone)]
pub struct RegionStoryGroup {
    pub slug: String,
    pub name: String,
    pub stories: Vec<StoryView>,
}

/// 404 page template.
#[derive(Template)]
#[template(path = "pages/not_found.html")]
pub struct NotFoundTemplate {
    pub slug: String,
}

/// Convert region slug to display name.
pub fn slug_to_display_name(slug: &str) -> String {
    match slug {
        "national" => "Czech Republic".to_string(),
        "praha" => "Praha".to_string(),
        "brno" => "Brno".to_string(),
        "ostrava" => "Ostrava".to_string(),
        "plzen" => "Plzen".to_string(),
        "liberec" => "Liberec".to_string(),
        "olomouc" => "Olomouc".to_string(),
        "hradec_kralove" => "Hradec Kralove".to_string(),
        "ceske_budejovice" => "Ceske Budejovice".to_string(),
        "usti_nad_labem" => "Usti nad Labem".to_string(),
        "pardubice" => "Pardubice".to_string(),
        "zlin" => "Zlin".to_string(),
        "karlovy_vary" => "Karlovy Vary".to_string(),
        "jihlava" => "Jihlava".to_string(),
        "stredocesky" => "Středočeský kraj".to_string(),
        _ => slug.replace('_', " "),
    }
}

/// Get CSS heat class based on months-to-buy severity.
pub fn severity_color(months: f64) -> &'static str {
    if months > 160.0 {
        "heat-5"
    } else if months > 130.0 {
        "heat-4"
    } else if months > 110.0 {
        "heat-3"
    } else if months > 100.0 {
        "heat-2"
    } else {
        "heat-1"
    }
}

/// Format an integer with Czech-style thousands separator (non-breaking space).
pub fn fmt_thousands(n: f64) -> String {
    let s = format!("{:.0}", n.abs());
    let bytes = s.as_bytes();
    let mut result = String::new();
    if n < 0.0 {
        result.push('-');
    }
    for (i, ch) in bytes.iter().enumerate() {
        if i > 0 && (bytes.len() - i) % 3 == 0 {
            result.push('\u{a0}'); // non-breaking space
        }
        result.push(*ch as char);
    }
    result
}

/// Format optional f64 with a suffix, or "N/A".
pub fn fmt_value(v: Option<f64>, suffix: &str) -> String {
    v.map_or("N/A".to_string(), |x| format!("{}{suffix}", fmt_thousands(x)))
}

/// Format optional f64 as percentage.
pub fn fmt_pct(v: Option<f64>) -> String {
    v.map_or("N/A".to_string(), |x| format!("{x:.2}%"))
}

/// Format optional f64 as ratio.
pub fn fmt_ratio(v: Option<f64>) -> String {
    v.map_or("N/A".to_string(), |x| format!("{x:.2}"))
}

/// Format optional f64 as years.
pub fn fmt_years(v: Option<f64>) -> String {
    v.map_or("N/A".to_string(), |x| {
        if x.is_infinite() { "N/A".to_string() } else { format!("{x:.1} years") }
    })
}
