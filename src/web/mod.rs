pub mod routes;
pub mod state;

use axum::{routing::get, Router};
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};

use state::AppState;

pub async fn serve(db: Arc<crate::db::Database>, port: u16) -> anyhow::Result<()> {
    let state = AppState { db };

    let cors = CorsLayer::new()
        .allow_methods([axum::http::Method::GET, axum::http::Method::POST])
        .allow_origin(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/", get(routes::index))
        .route("/api/runs", get(routes::list_runs))
        .route("/api/runs/{id}", get(routes::get_run))
        .layer(cors)
        .with_state(Arc::new(state));

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
    println!(
        "AgentBench web dashboard running on http://0.0.0.0:{}",
        port
    );

    axum::serve(listener, app).await?;
    Ok(())
}
