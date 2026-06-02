use axum::{
    extract::{Path, State},
    response::Html,
    Json,
};
use std::sync::Arc;

use super::state::AppState;

pub async fn index() -> Html<&'static str> {
    Html(
        r#"
<!DOCTYPE html>
<html>
<head>
    <title>AgentBench Dashboard</title>
    <style>
        body { background: #0D001A; color: #E0E0E0; font-family: monospace; margin: 0; padding: 2rem; }
        h1 { color: #F3E70F; }
        .card { background: #240037; border: 1px solid #8F00FF; border-radius: 8px; padding: 1rem; margin: 1rem 0; }
        .neon { color: #FF7EDB; }
    </style>
</head>
<body>
    <h1>🎹🦈 AgentBench Dashboard</h1>
    <div class="card">
        <p>Web dashboard is a stub. Full implementation coming in v0.2.0.</p>
        <p class="neon">API endpoints: /api/runs, /api/runs/{id}</p>
    </div>
</body>
</html>
    "#,
    )
}

pub async fn list_runs(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    match state.db.get_runs(100) {
        Ok(runs) => Json(serde_json::json!({
            "runs": runs.iter().map(|r| serde_json::json!({
                "id": r.id,
                "harness": r.harness_name,
                "benchmark": r.benchmark_name,
                "status": r.status,
                "score": r.aggregate_score,
            })).collect::<Vec<_>>()
        })),
        Err(e) => Json(serde_json::json!({"error": e.to_string()})),
    }
}

pub async fn get_run(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    match state.db.get_runs(1) {
        Ok(runs) => {
            if let Some(run) = runs.into_iter().find(|r| r.id == id) {
                Json(serde_json::json!({
                    "id": run.id,
                    "harness": run.harness_name,
                    "benchmark": run.benchmark_name,
                    "status": run.status,
                    "score": run.aggregate_score,
                }))
            } else {
                Json(serde_json::json!({"error": "Run not found"}))
            }
        }
        Err(e) => Json(serde_json::json!({"error": e.to_string()})),
    }
}
