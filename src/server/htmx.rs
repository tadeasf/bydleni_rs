use axum::http::HeaderMap;

/// Check if the request is an HTMX request.
#[allow(dead_code)]
pub fn is_htmx_request(headers: &HeaderMap) -> bool {
    headers.contains_key("hx-request")
}

/// Get the HTMX trigger element ID, if present.
#[allow(dead_code)]
pub fn htmx_trigger(headers: &HeaderMap) -> Option<&str> {
    headers.get("hx-trigger")?.to_str().ok()
}
