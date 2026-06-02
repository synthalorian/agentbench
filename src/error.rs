use thiserror::Error;

#[derive(Error, Debug)]
pub enum BenchError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("YAML error: {0}")]
    Yaml(#[from] serde_yaml::Error),
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("Harness error: {0}")]
    Harness(String),
    #[error("Benchmark error: {0}")]
    Benchmark(String),
    #[error("Config error: {0}")]
    Config(String),
    #[error("Task execution error: {0}")]
    TaskExecution(String),
    #[error("Not found: {0}")]
    NotFound(String),
}

pub type BenchResult<T> = Result<T, BenchError>;
