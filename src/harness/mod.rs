use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::error::BenchResult;

pub mod claude_code;
pub mod codex;
pub mod generic;
pub mod hermes;
pub mod opencode;
pub mod openshark;

/// A single task given to a harness
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub task_type: String,
    pub prompt: String,
    pub context: HashMap<String, String>,
    pub files: Vec<String>, // paths to relevant files
    pub expected_output: Option<String>,
}

/// The harness's response to a task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResponse {
    pub task_id: String,
    pub output: String,
    pub patch: Option<String>, // unified diff format
    pub tool_calls: Vec<ToolCall>,
    pub metadata: HashMap<String, String>,
    pub latency_ms: u64,
    pub tokens_input: u64,
    pub tokens_output: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub tool: String,
    pub args: serde_json::Value,
    pub result: Option<String>,
}

/// Configuration for a harness adapter
#[derive(Debug, Clone)]
pub struct HarnessAdapterConfig {
    pub name: String,
    pub endpoint: Option<String>,
    pub api_key: Option<String>,
    pub model: Option<String>,
    pub extra: HashMap<String, serde_yaml::Value>,
}

/// Core trait for all harness adapters
#[async_trait]
pub trait HarnessAdapter: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;

    /// Initialize the harness with config
    async fn init(&mut self, config: HarnessAdapterConfig) -> BenchResult<()>;

    /// Execute a single task
    async fn execute_task(&self, task: &Task) -> BenchResult<TaskResponse>;

    /// Health check
    async fn health_check(&self) -> BenchResult<bool>;

    /// Shutdown / cleanup
    async fn shutdown(&self) -> BenchResult<()>;
}

/// Registry of available harness adapters
pub struct HarnessRegistry {
    adapters: HashMap<String, Box<dyn HarnessAdapter>>,
}

impl HarnessRegistry {
    pub fn new() -> Self {
        Self {
            adapters: HashMap::new(),
        }
    }

    pub fn register(&mut self, name: String, adapter: Box<dyn HarnessAdapter>) {
        self.adapters.insert(name, adapter);
    }

    pub fn get(&self, name: &str) -> Option<&dyn HarnessAdapter> {
        self.adapters.get(name).map(|a| a.as_ref())
    }

    pub fn list(&self) -> Vec<&str> {
        self.adapters.keys().map(|s| s.as_str()).collect()
    }
}

impl Default for HarnessRegistry {
    fn default() -> Self {
        Self::new()
    }
}
