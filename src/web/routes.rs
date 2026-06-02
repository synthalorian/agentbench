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
        table { width: 100%; border-collapse: collapse; margin-top: 1rem; }
        th { background: #240037; color: #F3E70F; padding: 0.5rem; text-align: left; border-bottom: 2px solid #8F00FF; }
        td { padding: 0.5rem; border-bottom: 1px solid #333; }
        tr:hover { background: #1a0029; }
        .pass { color: #00FF00; }
        .fail { color: #FF0000; }
        .chart { height: 200px; background: #1a0029; border: 1px solid #8F00FF; border-radius: 4px; padding: 1rem; margin: 1rem 0; }
    </style>
</head>
<body>
    <h1>🎹🦈 AgentBench Dashboard</h1>
    <div class="card">
        <p class="neon">Real-time benchmark results for AI coding agents</p>
        <div id="runs-container">Loading...</div>
    </div>
    <script>
        async function loadRuns() {
            const resp = await fetch('/api/runs');
            const data = await resp.json();
            const container = document.getElementById('runs-container');
            
            if (data.error) {
                container.innerHTML = '<p class="fail">Error: ' + data.error + '</p>';
                return;
            }
            
            let html = '<table><thead><tr>';
            html += '<th>Run ID</th><th>Harness</th><th>Benchmark</th><th>Status</th><th>Score</th>';
            html += '</tr></thead><tbody>';
            
            for (const run of data.runs) {
                const scoreClass = run.score && run.score > 0.5 ? 'pass' : 'fail';
                html += '<tr>';
                html += '<td>' + run.id.substring(0, 8) + '</td>';
                html += '<td>' + run.harness + '</td>';
                html += '<td>' + run.benchmark + '</td>';
                html += '<td>' + run.status + '</td>';
                html += '<td class="' + scoreClass + '">' + (run.score ? (run.score * 100).toFixed(1) + '%' : '—') + '</td>';
                html += '</tr>';
            }
            
            html += '</tbody></table>';
            container.innerHTML = html;
        }
        
        loadRuns();
        setInterval(loadRuns, 5000);
    </script>
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
                "tasks_completed": r.tasks_completed,
                "started_at": r.started_at,
            })).collect::<Vec<_>>()
        })),
        Err(e) => Json(serde_json::json!({"error": e.to_string()})),
    }
}

pub async fn get_run(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    match state.db.get_runs(100) {
        Ok(runs) => {
            if let Some(run) = runs.into_iter().find(|r| r.id == id) {
                Json(serde_json::json!({
                    "id": run.id,
                    "harness": run.harness_name,
                    "benchmark": run.benchmark_name,
                    "status": run.status,
                    "score": run.aggregate_score,
                    "tasks_completed": run.tasks_completed,
                    "started_at": run.started_at,
                    "finished_at": run.finished_at,
                }))
            } else {
                Json(serde_json::json!({"error": "Run not found"}))
            }
        }
        Err(e) => Json(serde_json::json!({"error": e.to_string()})),
    }
}
