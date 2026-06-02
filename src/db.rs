use rusqlite::{params, Connection};
use std::sync::Mutex;

use crate::benchmark::BenchmarkResult;
use crate::error::BenchResult;

pub struct Database {
    conn: Mutex<Connection>,
}

impl Database {
    pub fn new(path: &str) -> BenchResult<Self> {
        let conn = Connection::open(path)?;
        let db = Self {
            conn: Mutex::new(conn),
        };
        db.init_schema()?;
        Ok(db)
    }

    pub fn init_schema(&self) -> BenchResult<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS runs (
                id TEXT PRIMARY KEY,
                harness_name TEXT NOT NULL,
                benchmark_name TEXT NOT NULL,
                started_at TIMESTAMP NOT NULL,
                finished_at TIMESTAMP,
                aggregate_score REAL,
                tasks_completed INTEGER,
                status TEXT DEFAULT 'running'
            );

            CREATE TABLE IF NOT EXISTS results (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                run_id TEXT NOT NULL REFERENCES runs(id),
                task_id TEXT NOT NULL,
                passed BOOLEAN NOT NULL,
                score REAL NOT NULL,
                latency_ms INTEGER,
                tokens_input INTEGER,
                tokens_output INTEGER,
                output TEXT,
                patch TEXT,
                error TEXT,
                started_at TIMESTAMP,
                finished_at TIMESTAMP
            );

            CREATE INDEX IF NOT EXISTS idx_results_run_id ON results(run_id);
            CREATE INDEX IF NOT EXISTS idx_results_task_id ON results(task_id);
            CREATE INDEX IF NOT EXISTS idx_runs_harness ON runs(harness_name);
            CREATE INDEX IF NOT EXISTS idx_runs_benchmark ON runs(benchmark_name);
            "#,
        )?;
        Ok(())
    }

    pub fn create_run(
        &self,
        run_id: &str,
        harness: &str,
        benchmark: &str,
        started_at: chrono::DateTime<chrono::Utc>,
    ) -> BenchResult<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO runs (id, harness_name, benchmark_name, started_at, status) VALUES (?1, ?2, ?3, ?4, 'running')",
            params![run_id, harness, benchmark, started_at.to_rfc3339()],
        )?;
        Ok(())
    }

    pub fn save_result(&self, run_id: &str, result: &BenchmarkResult) -> BenchResult<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO results (run_id, task_id, passed, score, latency_ms, tokens_input, tokens_output, output, patch, error, started_at, finished_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                run_id,
                result.task_id,
                result.passed,
                result.score,
                result.response.latency_ms as i64,
                result.response.tokens_input as i64,
                result.response.tokens_output as i64,
                result.response.output,
                result.response.patch.as_ref(),
                result.error.as_ref(),
                result.started_at.to_rfc3339(),
                result.finished_at.to_rfc3339(),
            ],
        )?;
        Ok(())
    }

    pub fn finish_run(
        &self,
        run_id: &str,
        finished_at: chrono::DateTime<chrono::Utc>,
        aggregate_score: f64,
        tasks_completed: usize,
    ) -> BenchResult<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE runs SET finished_at = ?1, aggregate_score = ?2, tasks_completed = ?3, status = 'completed' WHERE id = ?4",
            params![finished_at.to_rfc3339(), aggregate_score, tasks_completed as i64, run_id],
        )?;
        Ok(())
    }

    pub fn get_runs(&self, limit: usize) -> BenchResult<Vec<RunSummary>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, harness_name, benchmark_name, started_at, finished_at, aggregate_score, tasks_completed, status FROM runs ORDER BY started_at DESC LIMIT ?1"
        )?;

        let rows = stmt.query_map(params![limit as i64], |row| {
            Ok(RunSummary {
                id: row.get(0)?,
                harness_name: row.get(1)?,
                benchmark_name: row.get(2)?,
                started_at: row.get(3)?,
                finished_at: row.get(4)?,
                aggregate_score: row.get(5)?,
                tasks_completed: row.get(6)?,
                status: row.get(7)?,
            })
        })?;

        let mut runs = vec![];
        for row in rows {
            runs.push(row?);
        }
        Ok(runs)
    }
}

#[derive(Debug, Clone)]
pub struct RunSummary {
    pub id: String,
    pub harness_name: String,
    pub benchmark_name: String,
    pub started_at: String,
    pub finished_at: Option<String>,
    pub aggregate_score: Option<f64>,
    pub tasks_completed: Option<i64>,
    pub status: String,
}
