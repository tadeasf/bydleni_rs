use axum::extract::State;
use axum::response::{IntoResponse, Json};

use crate::server::AppState;

/// Data freshness / status endpoint.
pub(super) async fn status(State(state): State<AppState>) -> impl IntoResponse {
    let refreshing = state.refreshing.load(std::sync::atomic::Ordering::Relaxed);
    let last_refresh = state.last_refresh.read().await.clone();

    Json(serde_json::json!({
        "refreshing": refreshing,
        "last_refresh": last_refresh,
    }))
}
