pub mod api;
pub mod error;
pub mod htmx;
pub mod methodology;
pub mod routes;
pub mod scheduler;
pub mod templates;

use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use axum::Router;
use sqlx::SqlitePool;
use tokio::sync::RwLock;
use tower_http::cors::CorsLayer;
use tower_http::services::ServeDir;

use crate::config::Config;

/// Shared application state available to all route handlers.
#[derive(Clone)]
pub struct AppState {
    pub pool: SqlitePool,
    #[allow(dead_code)]
    pub config: Arc<Config>,
    pub refreshing: Arc<AtomicBool>,
    pub last_refresh: Arc<RwLock<Option<String>>>,
}

/// Build the axum Router with all routes and middleware.
pub fn build_router(state: AppState) -> Router {
    Router::new()
        .merge(routes::routes())
        .merge(api::routes())
        .nest_service("/static", ServeDir::new("static"))
        .layer(CorsLayer::permissive())
        .with_state(state)
}

/// Start the web server.
pub async fn serve(pool: SqlitePool, config: Config) -> anyhow::Result<()> {
    let port = config.server_port;
    let refreshing = Arc::new(AtomicBool::new(false));
    let last_refresh: Arc<RwLock<Option<String>>> = Arc::new(RwLock::new(None));
    let config = Arc::new(config);

    let state = AppState {
        pool: pool.clone(),
        config: config.clone(),
        refreshing: refreshing.clone(),
        last_refresh: last_refresh.clone(),
    };

    // Spawn initial data refresh in the background
    {
        let pool = pool.clone();
        let config = config.clone();
        let refreshing = refreshing.clone();
        let last_refresh = last_refresh.clone();
        tokio::spawn(async move {
            scheduler::run_refresh(&pool, &config, &refreshing, &last_refresh).await;
        });
    }

    // Start periodic refresh every 6 hours
    scheduler::start_periodic(pool, config, refreshing, last_refresh).await?;

    let app = build_router(state);
    let addr = format!("0.0.0.0:{port}");
    tracing::info!("Starting server on {addr}");

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
