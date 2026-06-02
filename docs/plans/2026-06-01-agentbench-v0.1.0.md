# AgentBench v0.1.0 Implementation Plan

> **For Hermes:** Use subagent-driven-development skill to implement this plan task-by-task.

**Goal:** Build an open-source, self-hostable benchmark runner for AI coding agents. Point it at any harness (OpenShark, Hermes, Claude Code, Codex, generic OpenAI API), run SWE-bench / Terminal-bench / LiveCodeBench tasks, collect structured results, and render beautiful comparisons via TUI + web dashboard.

**Architecture:** Rust CLI binary with pluggable harness adapters, benchmark suite definitions in YAML, Docker-isolated task execution, SQLite results store, ratatui real-time TUI, and an embedded Axum web dashboard. Synthwave aesthetic throughout.

**Tech Stack:** Rust, tokio, clap, ratatui, rusqlite, axum, serde, reqwest, tokio-process, docker-api (or std::process::Command + docker CLI), crossterm, tui-rs-chart (or custom), handlebars (HTML reports).

---

## Project Structure

```
agentbench/
├── Cargo.toml
├── Cargo.lock
├── .gitignore
├── README.md
├── LICENSE
├── .github/
│   └── workflows/
│       ├── ci.yml
│       └── release.yml
├── src/
│   ├── main.rs              # CLI entry, tokio::main
│   ├── cli.rs               # clap derive definitions
│   ├── config.rs            # Config loading (YAML)
│   ├── error.rs             # Centralized error type
│   ├── harness/
│   │   ├── mod.rs           # Harness trait + registry
│   │   ├── generic.rs       # OpenAI-compatible API harness
│   │   ├── openshark.rs     # OpenShark adapter
│   │   ├── hermes.rs        # Hermes adapter
│   │   ├── claude_code.rs   # Claude Code adapter
│   │   └── codex.rs         # Codex adapter
│   ├── benchmark/
│   │   ├── mod.rs           # Benchmark trait + registry
│   │   ├── swe_bench.rs     # SWE-bench loader
│   │   ├── terminal_bench.rs # Terminal-bench loader
│   │   └── livecodebench.rs # LiveCodeBench loader
│   ├── runner.rs            # Task execution engine
│   ├── metrics.rs           # Result collection & scoring
│   ├── db.rs                # SQLite schema + queries
│   ├── report.rs            # Markdown/JSON/HTML export
│   ├── tui/
│   │   ├── mod.rs           # TUI entry point
│   │   ├── app.rs           # App state
│   │   ├── ui.rs            # Layout + widgets
│   │   └── theme.rs         # Synthwave color palette
│   └── web/
│       ├── mod.rs           # Axum server entry
│       ├── routes.rs        # HTTP handlers
│       └── state.rs         # Shared AppState
├── benches/
│   ├── swe-bench-lite.yml
│   └── terminal-bench.yml
├── themes/
│   └── synthwave.yml
└── tests/
    └── integration_tests.rs
```

---

## Phase 1: Foundation (Days 1-3)

### Task 1: Initialize Cargo project with dependencies

**Objective:** Create the project skeleton and add all required crates.

**Files:**
- Create: `/home/synth/projects/agentbench/Cargo.toml`
- Create: `/home/synth/projects/agentbench/.gitignore`
- Create: `/home/synth/projects/agentbench/src/main.rs`

**Step 1: Create Cargo.toml**

```toml
[package]
name = "agentbench"
version = "0.1.0"
edition = "2021"
authors = ["synth <synthalorian>"]
description = "Open-source benchmark runner for AI coding agents"
license = "MIT"
repository = "https://github.com/synthalorian/agentbench"

[dependencies]
tokio = { version = "1", features = ["full"] }
clap = { version = "4", features = ["derive"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
serde_yaml = "0.9"
reqwest = { version = "0.12", features = ["json"] }
rusqlite = { version = "0.32", features = ["bundled"] }
ratatui = "0.29"
crossterm = "0.28"
axum = "0.8"
tower-http = { version = "0.6", features = ["cors"] }
chrono = { version = "0.4", features = ["serde"] }
uuid = { version = "1", features = ["v4", "serde"] }
anyhow = "1"
thiserror = "1"
tracing = "0.1"
tracing-subscriber = "0.3"
indicatif = "0.17"
handlebars = "6"
walkdir = "2"
tempfile = "3"

[dev-dependencies]
tokio-test = "0.4"
```

**Step 2: Create .gitignore**

```
/target/
Cargo.lock
*.db
*.log
.env
.DS_Store
```

**Step 3: Create src/main.rs (stub)**

```rust
use clap::Parser;

mod cli;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let args = cli::Cli::parse();
    println!("AgentBench v{} — {:?}", env!("CARGO_PKG_VERSION"), args.command);
    Ok(())
}
```

**Step 4: Create src/cli.rs**

```rust
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "agentbench")]
#[command(about = "Benchmark runner for AI coding agents")]
#[command(version = env!("CARGO_PKG_VERSION"))]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Run a benchmark suite
    Run {
        /// Path to benchmark config YAML
        #[arg(short, long)]
        config: String,
        /// Harness to benchmark (e.g., generic, openshark, hermes)
        #[arg(short, long)]
        harness: String,
        /// Output format: table, json, markdown
        #[arg(short, long, default_value = "table")]
        output: String,
    },
    /// Start the TUI dashboard
    Tui,
    /// Start the web dashboard
    Web {
        #[arg(short, long, default_value = "8910")]
        port: u16,
    },
    /// List available harnesses and benchmarks
    List,
    /// Export results to a report
    Report {
        /// Run ID to report on
        #[arg(short, long)]
        run_id: String,
        /// Output format: markdown, json, html
        #[arg(short, long, default_value = "markdown")]
        format: String,
        /// Output file path
        #[arg(short, long)]
        output: Option<String>,
    },
}
```

**Step 5: Verify compilation**

Run: `cd /home/synth/projects/agentbench && cargo check`
Expected: Clean compile, no errors.

**Step 6: Commit**

```bash
git init
git add Cargo.toml .gitignore src/main.rs src/cli.rs
git commit -m "feat: initialize AgentBench project skeleton"
```

---

### Task 2: Centralized error handling

**Objective:** Define a custom Error type using thiserror for the entire crate.

**Files:**
- Create: `/home/synth/projects/agentbench/src/error.rs`
- Modify: `/home/synth/projects/agentbench/src/main.rs` (add mod error)

**Step 1: Create src/error.rs**

```rust
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
```

**Step 2: Add mod error to main.rs**

```rust
mod cli;
mod error;
```

**Step 3: Verify**

Run: `cargo check`
Expected: Clean compile.

**Step 4: Commit**

```bash
git add src/error.rs src/main.rs
git commit -m "feat: add centralized error type"
```

---

### Task 3: Config loading (YAML)

**Objective:** Load benchmark suite configurations from YAML files.

**Files:**
- Create: `/home/synth/projects/agentbench/src/config.rs`
- Create: `/home/synth/projects/agentbench/benches/swe-bench-lite.yml`

**Step 1: Create src/config.rs**

```rust
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;

use crate::error::BenchResult;

#[derive(Debug, Clone, Deserialize)]
pub struct BenchmarkConfig {
    pub name: String,
    pub description: String,
    pub benchmark_type: String, // "swe_bench", "terminal_bench", "livecodebench"
    pub dataset: DatasetConfig,
    pub harness: HarnessConfig,
    pub runner: RunnerConfig,
    pub scoring: ScoringConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DatasetConfig {
    pub source: String, // "huggingface", "local", "git"
    pub path: String,
    pub split: Option<String>,
    pub subset: Option<String>,
    pub filter: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct HarnessConfig {
    pub name: String,
    pub adapter: String, // "generic", "openshark", "hermes", "claude_code", "codex"
    pub endpoint: Option<String>,
    pub api_key: Option<String>,
    pub model: Option<String>,
    pub extra: Option<HashMap<String, serde_yaml::Value>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RunnerConfig {
    pub max_workers: usize,
    pub timeout_secs: u64,
    pub retries: u32,
    pub docker_image: Option<String>,
    pub env: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ScoringConfig {
    pub metric: String, // "pass_rate", "exact_match", "bleu"
    pub thresholds: Option<HashMap<String, f64>>,
}

impl BenchmarkConfig {
    pub fn from_file<P: AsRef<Path>>(path: P) -> BenchResult<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: BenchmarkConfig = serde_yaml::from_str(&content)?;
        Ok(config)
    }
}
```

**Step 2: Create benches/swe-bench-lite.yml**

```yaml
name: "SWE-bench Lite"
description: "Lightweight SWE-bench subset for rapid agent evaluation"
benchmark_type: "swe_bench"

dataset:
  source: "huggingface"
  path: "princeton-nlp/SWE-bench_Lite"
  split: "test"
  subset: null
  filter: null

harness:
  name: "generic-openai"
  adapter: "generic"
  endpoint: "http://localhost:8080/v1"
  api_key: null
  model: "local-model"
  extra:
    max_tokens: 4096
    temperature: 0.0

runner:
  max_workers: 4
  timeout_secs: 300
  retries: 1
  docker_image: "agentbench/swe-bench:latest"
  env:
    PYTHONPATH: "/testbed"

scoring:
  metric: "pass_rate"
  thresholds:
    excellent: 0.50
    good: 0.30
    acceptable: 0.15
```

**Step 3: Add mod config to main.rs**

```rust
mod cli;
mod config;
mod error;
```

**Step 4: Verify**

Run: `cargo check`
Expected: Clean compile.

**Step 5: Commit**

```bash
git add src/config.rs benches/swe-bench-lite.yml src/main.rs
git commit -m "feat: add YAML config loading with benchmark suite schema"
```

---

## Phase 2: Harness Adapters (Days 4-6)

### Task 4: Define the Harness trait

**Objective:** Create the core abstraction that all agent harnesses implement.

**Files:**
- Create: `/home/synth/projects/agentbench/src/harness/mod.rs`

**Step 1: Create src/harness/mod.rs**

```rust
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::error::BenchResult;

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
    
    pub fn get_mut(&mut self, name: &str) -> Option<&mut dyn HarnessAdapter> {
        self.adapters.get_mut(name).map(|a| a.as_mut())
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
```

**Step 2: Add mod harness to main.rs**

```rust
mod harness;
```

**Step 3: Verify**

Run: `cargo check`
Expected: Clean compile.

**Step 4: Commit**

```bash
git add src/harness/mod.rs src/main.rs
git commit -m "feat: define HarnessAdapter trait and registry"
```

---

### Task 5: Generic OpenAI-compatible harness adapter

**Objective:** Implement the most basic harness — any OpenAI-compatible API endpoint.

**Files:**
- Create: `/home/synth/projects/agentbench/src/harness/generic.rs`
- Modify: `/home/synth/projects/agentbench/src/harness/mod.rs` (add pub mod generic)

**Step 1: Create src/harness/generic.rs**

```rust
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;

use crate::error::{BenchError, BenchResult};
use super::{HarnessAdapter, HarnessAdapterConfig, Task, TaskResponse, ToolCall};

pub struct GenericOpenAIHarness {
    config: Option<HarnessAdapterConfig>,
    client: Client,
}

impl GenericOpenAIHarness {
    pub fn new() -> Self {
        Self {
            config: None,
            client: Client::new(),
        }
    }
}

#[derive(Debug, Serialize)]
struct ChatCompletionRequest {
    model: String,
    messages: Vec<Message>,
    max_tokens: u32,
    temperature: f32,
}

#[derive(Debug, Serialize)]
struct Message {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct ChatCompletionResponse {
    choices: Vec<Choice>,
    usage: Option<Usage>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: ResponseMessage,
}

#[derive(Debug, Deserialize)]
struct ResponseMessage {
    content: String,
}

#[derive(Debug, Deserialize)]
struct Usage {
    prompt_tokens: u64,
    completion_tokens: u64,
}

#[async_trait]
impl HarnessAdapter for GenericOpenAIHarness {
    fn name(&self) -> &str {
        "generic"
    }
    
    fn description(&self) -> &str {
        "Generic OpenAI-compatible API harness"
    }
    
    async fn init(&mut self, config: HarnessAdapterConfig) -> BenchResult<()> {
        self.config = Some(config);
        Ok(())
    }
    
    async fn execute_task(&self, task: &Task) -> BenchResult<TaskResponse> {
        let config = self.config.as_ref()
            .ok_or_else(|| BenchError::Harness("Harness not initialized".to_string()))?;
        
        let endpoint = config.endpoint.as_ref()
            .ok_or_else(|| BenchError::Harness("No endpoint configured".to_string()))?;
        
        let model = config.model.as_ref()
            .map(|s| s.as_str())
            .unwrap_or("local-model");
        
        let max_tokens = config.extra.get("max_tokens")
            .and_then(|v| v.as_u64())
            .unwrap_or(4096) as u32;
        
        let temperature = config.extra.get("temperature")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0) as f32;
        
        let request_body = ChatCompletionRequest {
            model: model.to_string(),
            messages: vec![Message {
                role: "user".to_string(),
                content: task.prompt.clone(),
            }],
            max_tokens,
            temperature,
        };
        
        let start = std::time::Instant::now();
        
        let mut req = self.client.post(format!("{}/chat/completions", endpoint))
            .json(&request_body);
        
        if let Some(key) = &config.api_key {
            req = req.header("Authorization", format!("Bearer {}", key));
        }
        
        let response = req.send().await?;
        let status = response.status();
        
        if !status.is_success() {
            let text = response.text().await.unwrap_or_default();
            return Err(BenchError::Harness(format!("API error {}: {}", status, text)));
        }
        
        let completion: ChatCompletionResponse = response.json().await?;
        let latency_ms = start.elapsed().as_millis() as u64;
        
        let output = completion.choices.first()
            .map(|c| c.message.content.clone())
            .unwrap_or_default();
        
        let (tokens_input, tokens_output) = completion.usage
            .map(|u| (u.prompt_tokens, u.completion_tokens))
            .unwrap_or((0, 0));
        
        Ok(TaskResponse {
            task_id: task.id.clone(),
            output,
            patch: None,
            tool_calls: vec![],
            metadata: HashMap::new(),
            latency_ms,
            tokens_input,
            tokens_output,
        })
    }
    
    async fn health_check(&self) -> BenchResult<bool> {
        let config = match &self.config {
            Some(c) => c,
            None => return Ok(false),
        };
        
        let endpoint = match &config.endpoint {
            Some(e) => e,
            None => return Ok(false),
        };
        
        let resp = self.client.get(format!("{}/models", endpoint))
            .send().await?;
        
        Ok(resp.status().is_success())
    }
    
    async fn shutdown(&self) -> BenchResult<()> {
        Ok(())
    }
}
```

**Step 2: Update src/harness/mod.rs**

Add at the top:
```rust
pub mod generic;
```

**Step 3: Verify**

Run: `cargo check`
Expected: Clean compile.

**Step 4: Commit**

```bash
git add src/harness/generic.rs src/harness/mod.rs
git commit -m "feat: add generic OpenAI-compatible harness adapter"
```

---

### Task 6: Stub remaining harness adapters

**Objective:** Create placeholder adapters for OpenShark, Hermes, Claude Code, and Codex so the registry is complete.

**Files:**
- Create: `/home/synth/projects/agentbench/src/harness/openshark.rs`
- Create: `/home/synth/projects/agentbench/src/harness/hermes.rs`
- Create: `/home/synth/projects/agentbench/src/harness/claude_code.rs`
- Create: `/home/synth/projects/agentbench/src/harness/codex.rs`
- Modify: `/home/synth/projects/agentbench/src/harness/mod.rs`

**Step 1: Create src/harness/openshark.rs**

```rust
use async_trait::async_trait;
use crate::error::BenchResult;
use super::{HarnessAdapter, HarnessAdapterConfig, Task, TaskResponse};

pub struct OpenSharkHarness;

#[async_trait]
impl HarnessAdapter for OpenSharkHarness {
    fn name(&self) -> &str { "openshark" }
    fn description(&self) -> &str { "OpenShark harness (TODO: implement)" }
    async fn init(&mut self, _config: HarnessAdapterConfig) -> BenchResult<()> { Ok(()) }
    async fn execute_task(&self, _task: &Task) -> BenchResult<TaskResponse> {
        todo!("OpenShark adapter not yet implemented")
    }
    async fn health_check(&self) -> BenchResult<bool> { Ok(false) }
    async fn shutdown(&self) -> BenchResult<()> { Ok(()) }
}
```

**Step 2: Create src/harness/hermes.rs**

```rust
use async_trait::async_trait;
use crate::error::BenchResult;
use super::{HarnessAdapter, HarnessAdapterConfig, Task, TaskResponse};

pub struct HermesHarness;

#[async_trait]
impl HarnessAdapter for HermesHarness {
    fn name(&self) -> &str { "hermes" }
    fn description(&self) -> &str { "Hermes Agent harness (TODO: implement)" }
    async fn init(&mut self, _config: HarnessAdapterConfig) -> BenchResult<()> { Ok(()) }
    async fn execute_task(&self, _task: &Task) -> BenchResult<TaskResponse> {
        todo!("Hermes adapter not yet implemented")
    }
    async fn health_check(&self) -> BenchResult<bool> { Ok(false) }
    async fn shutdown(&self) -> BenchResult<()> { Ok(()) }
}
```

**Step 3: Create src/harness/claude_code.rs**

```rust
use async_trait::async_trait;
use crate::error::BenchResult;
use super::{HarnessAdapter, HarnessAdapterConfig, Task, TaskResponse};

pub struct ClaudeCodeHarness;

#[async_trait]
impl HarnessAdapter for ClaudeCodeHarness {
    fn name(&self) -> &str { "claude_code" }
    fn description(&self) -> &str { "Claude Code harness (TODO: implement)" }
    async fn init(&mut self, _config: HarnessAdapterConfig) -> BenchResult<()> { Ok(()) }
    async fn execute_task(&self, _task: &Task) -> BenchResult<TaskResponse> {
        todo!("Claude Code adapter not yet implemented")
    }
    async fn health_check(&self) -> BenchResult<bool> { Ok(false) }
    async fn shutdown(&self) -> BenchResult<()> { Ok(()) }
}
```

**Step 4: Create src/harness/codex.rs**

```rust
use async_trait::async_trait;
use crate::error::BenchResult;
use super::{HarnessAdapter, HarnessAdapterConfig, Task, TaskResponse};

pub struct CodexHarness;

#[async_trait]
impl HarnessAdapter for CodexHarness {
    fn name(&self) -> &str { "codex" }
    fn description(&self) -> &str { "OpenAI Codex harness (TODO: implement)" }
    async fn init(&mut self, _config: HarnessAdapterConfig) -> BenchResult<()> { Ok(()) }
    async fn execute_task(&self, _task: &Task) -> BenchResult<TaskResponse> {
        todo!("Codex adapter not yet implemented")
    }
    async fn health_check(&self) -> BenchResult<bool> { Ok(false) }
    async fn shutdown(&self) -> BenchResult<()> { Ok(()) }
}
```

**Step 5: Update src/harness/mod.rs**

```rust
pub mod generic;
pub mod openshark;
pub mod hermes;
pub mod claude_code;
pub mod codex;
```

**Step 6: Verify**

Run: `cargo check`
Expected: Clean compile (with todo!() warnings).

**Step 7: Commit**

```bash
git add src/harness/*.rs
git commit -m "feat: add harness adapter stubs for openshark, hermes, claude_code, codex"
```

---

## Phase 3: Benchmark Definitions (Days 7-9)

### Task 7: Define the Benchmark trait

**Objective:** Create the abstraction for benchmark suites (SWE-bench, Terminal-bench, etc.).

**Files:**
- Create: `/home/synth/projects/agentbench/src/benchmark/mod.rs`

**Step 1: Create src/benchmark/mod.rs**

```rust
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::error::BenchResult;
use crate::harness::TaskResponse;

/// A single benchmark task (loaded from dataset)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkTask {
    pub id: String,
    pub task_type: String,
    pub repo: Option<String>,
    pub base_commit: Option<String>,
    pub problem_statement: String,
    pub hints: Vec<String>,
    pub test_patch: Option<String>,
    pub expected_files: Vec<String>,
    pub metadata: HashMap<String, String>,
}

/// Result of running a single benchmark task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkResult {
    pub task_id: String,
    pub harness_name: String,
    pub benchmark_name: String,
    pub passed: bool,
    pub score: f64,
    pub response: TaskResponse,
    pub validation_output: Option<String>,
    pub error: Option<String>,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub finished_at: chrono::DateTime<chrono::Utc>,
}

/// Configuration for a benchmark run
#[derive(Debug, Clone)]
pub struct BenchmarkRunConfig {
    pub harness_name: String,
    pub max_tasks: Option<usize>,
    pub shuffle: bool,
    pub seed: Option<u64>,
}

/// Core trait for all benchmark suites
#[async_trait]
pub trait BenchmarkSuite: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn task_count(&self) -> usize;
    
    /// Load tasks from the dataset source
    async fn load_tasks(&mut self, config: &crate::config::DatasetConfig) -> BenchResult<()>;
    
    /// Get all loaded tasks
    fn tasks(&self) -> &[BenchmarkTask];
    
    /// Validate a harness response against the expected solution
    async fn validate(&self, task: &BenchmarkTask, response: &TaskResponse) -> BenchResult<BenchmarkResult>;
    
    /// Compute aggregate score from individual results
    fn aggregate_score(&self, results: &[BenchmarkResult]) -> f64;
}

/// Registry of available benchmark suites
pub struct BenchmarkRegistry {
    suites: HashMap<String, Box<dyn BenchmarkSuite>>,
}

impl BenchmarkRegistry {
    pub fn new() -> Self {
        Self {
            suites: HashMap::new(),
        }
    }
    
    pub fn register(&mut self, name: String, suite: Box<dyn BenchmarkSuite>) {
        self.suites.insert(name, suite);
    }
    
    pub fn get(&self, name: &str) -> Option<&dyn BenchmarkSuite> {
        self.suites.get(name).map(|s| s.as_ref())
    }
    
    pub fn get_mut(&mut self, name: &str) -> Option<&mut dyn BenchmarkSuite> {
        self.suites.get_mut(name).map(|s| s.as_mut())
    }
    
    pub fn list(&self) -> Vec<&str> {
        self.suites.keys().map(|s| s.as_str()).collect()
    }
}

impl Default for BenchmarkRegistry {
    fn default() -> Self {
        Self::new()
    }
}
```

**Step 2: Add mod benchmark to main.rs**

```rust
mod benchmark;
```

**Step 3: Verify**

Run: `cargo check`
Expected: Clean compile.

**Step 4: Commit**

```bash
git add src/benchmark/mod.rs src/main.rs
git commit -m "feat: define BenchmarkSuite trait and registry"
```

---

### Task 8: SWE-bench loader (stub)

**Objective:** Create a benchmark suite that can load SWE-bench tasks from HuggingFace or local JSON.

**Files:**
- Create: `/home/synth/projects/agentbench/src/benchmark/swe_bench.rs`
- Modify: `/home/synth/projects/agentbench/src/benchmark/mod.rs`

**Step 1: Create src/benchmark/swe_bench.rs**

```rust
use async_trait::async_trait;
use std::collections::HashMap;

use crate::error::BenchResult;
use crate::harness::TaskResponse;
use super::{BenchmarkResult, BenchmarkSuite, BenchmarkTask, BenchmarkRunConfig};

pub struct SWEBenchSuite {
    tasks: Vec<BenchmarkTask>,
}

impl SWEBenchSuite {
    pub fn new() -> Self {
        Self { tasks: vec![] }
    }
}

#[async_trait]
impl BenchmarkSuite for SWEBenchSuite {
    fn name(&self) -> &str { "swe_bench" }
    fn description(&self) -> &str { "SWE-bench: Software engineering tasks from GitHub issues" }
    fn task_count(&self) -> usize { self.tasks.len() }
    
    async fn load_tasks(&mut self, config: &crate::config::DatasetConfig) -> BenchResult<()> {
        match config.source.as_str() {
            "local" => {
                let path = std::path::Path::new(&config.path);
                if path.extension().and_then(|s| s.to_str()) == Some("json") {
                    let content = tokio::fs::read_to_string(path).await?;
                    let raw: Vec<serde_json::Value> = serde_json::from_str(&content)?;
                    self.tasks = raw.into_iter().enumerate().map(|(i, v)| BenchmarkTask {
                        id: v.get("instance_id").and_then(|s| s.as_str()).unwrap_or(&format!("task-{}", i)).to_string(),
                        task_type: "swe_bench".to_string(),
                        repo: v.get("repo").and_then(|s| s.as_str()).map(|s| s.to_string()),
                        base_commit: v.get("base_commit").and_then(|s| s.as_str()).map(|s| s.to_string()),
                        problem_statement: v.get("problem_statement").and_then(|s| s.as_str()).unwrap_or("").to_string(),
                        hints: vec![],
                        test_patch: v.get("test_patch").and_then(|s| s.as_str()).map(|s| s.to_string()),
                        expected_files: vec![],
                        metadata: HashMap::new(),
                    }).collect();
                }
            }
            "huggingface" => {
                // TODO: Implement HuggingFace datasets loading
                // For now, require local path
                return Err(crate::error::BenchError::Benchmark(
                    "HuggingFace loading not yet implemented. Download dataset locally and use source: local".to_string()
                ));
            }
            _ => {
                return Err(crate::error::BenchError::Benchmark(
                    format!("Unknown dataset source: {}", config.source)
                ));
            }
        }
        Ok(())
    }
    
    fn tasks(&self) -> &[BenchmarkTask] { &self.tasks }
    
    async fn validate(&self, task: &BenchmarkTask, response: &TaskResponse) -> BenchResult<BenchmarkResult> {
        let now = chrono::Utc::now();
        // TODO: Run tests in Docker container to validate patch
        let passed = response.patch.is_some();
        let score = if passed { 1.0 } else { 0.0 };
        
        Ok(BenchmarkResult {
            task_id: task.id.clone(),
            harness_name: "generic".to_string(), // populated by runner
            benchmark_name: self.name().to_string(),
            passed,
            score,
            response: response.clone(),
            validation_output: None,
            error: None,
            started_at: now,
            finished_at: now,
        })
    }
    
    fn aggregate_score(&self, results: &[BenchmarkResult]) -> f64 {
        if results.is_empty() { return 0.0; }
        let passed = results.iter().filter(|r| r.passed).count();
        passed as f64 / results.len() as f64
    }
}
```

**Step 2: Update src/benchmark/mod.rs**

```rust
pub mod swe_bench;
```

**Step 3: Verify**

Run: `cargo check`
Expected: Clean compile.

**Step 4: Commit**

```bash
git add src/benchmark/swe_bench.rs src/benchmark/mod.rs
git commit -m "feat: add SWE-bench suite loader (local JSON, stub validation)"
```

---

### Task 9: Terminal-bench and LiveCodeBench stubs

**Objective:** Create placeholder benchmark suites for Terminal-bench and LiveCodeBench.

**Files:**
- Create: `/home/synth/projects/agentbench/src/benchmark/terminal_bench.rs`
- Create: `/home/synth/projects/agentbench/src/benchmark/livecodebench.rs`
- Modify: `/home/synth/projects/agentbench/src/benchmark/mod.rs`

**Step 1: Create src/benchmark/terminal_bench.rs**

```rust
use async_trait::async_trait;
use std::collections::HashMap;

use crate::error::BenchResult;
use crate::harness::TaskResponse;
use super::{BenchmarkResult, BenchmarkSuite, BenchmarkTask};

pub struct TerminalBenchSuite {
    tasks: Vec<BenchmarkTask>,
}

impl TerminalBenchSuite {
    pub fn new() -> Self { Self { tasks: vec![] } }
}

#[async_trait]
impl BenchmarkSuite for TerminalBenchSuite {
    fn name(&self) -> &str { "terminal_bench" }
    fn description(&self) -> &str { "Terminal-bench: Command-line task execution" }
    fn task_count(&self) -> usize { self.tasks.len() }
    
    async fn load_tasks(&mut self, _config: &crate::config::DatasetConfig) -> BenchResult<()> {
        todo!("Terminal-bench loader not yet implemented")
    }
    
    fn tasks(&self) -> &[BenchmarkTask] { &self.tasks }
    
    async fn validate(&self, _task: &BenchmarkTask, _response: &TaskResponse) -> BenchResult<BenchmarkResult> {
        todo!("Terminal-bench validation not yet implemented")
    }
    
    fn aggregate_score(&self, results: &[BenchmarkResult]) -> f64 {
        if results.is_empty() { return 0.0; }
        results.iter().map(|r| r.score).sum::<f64>() / results.len() as f64
    }
}
```

**Step 2: Create src/benchmark/livecodebench.rs**

```rust
use async_trait::async_trait;
use std::collections::HashMap;

use crate::error::BenchResult;
use crate::harness::TaskResponse;
use super::{BenchmarkResult, BenchmarkSuite, BenchmarkTask};

pub struct LiveCodeBenchSuite {
    tasks: Vec<BenchmarkTask>,
}

impl LiveCodeBenchSuite {
    pub fn new() -> Self { Self { tasks: vec![] } }
}

#[async_trait]
impl BenchmarkSuite for LiveCodeBenchSuite {
    fn name(&self) -> &str { "livecodebench" }
    fn description(&self) -> &str { "LiveCodeBench: Live coding competition problems" }
    fn task_count(&self) -> usize { self.tasks.len() }
    
    async fn load_tasks(&mut self, _config: &crate::config::DatasetConfig) -> BenchResult<()> {
        todo!("LiveCodeBench loader not yet implemented")
    }
    
    fn tasks(&self) -> &[BenchmarkTask] { &self.tasks }
    
    async fn validate(&self, _task: &BenchmarkTask, _response: &TaskResponse) -> BenchResult<BenchmarkResult> {
        todo!("LiveCodeBench validation not yet implemented")
    }
    
    fn aggregate_score(&self, results: &[BenchmarkResult]) -> f64 {
        if results.is_empty() { return 0.0; }
        results.iter().map(|r| r.score).sum::<f64>() / results.len() as f64
    }
}
```

**Step 3: Update src/benchmark/mod.rs**

```rust
pub mod swe_bench;
pub mod terminal_bench;
pub mod livecodebench;
```

**Step 4: Verify**

Run: `cargo check`
Expected: Clean compile.

**Step 5: Commit**

```bash
git add src/benchmark/terminal_bench.rs src/benchmark/livecodebench.rs src/benchmark/mod.rs
git commit -m "feat: add terminal-bench and livecodebench suite stubs"
```

---

## Phase 4: Task Runner & Metrics (Days 10-12)

### Task 10: Task execution engine

**Objective:** Build the runner that orchestrates harness execution over benchmark tasks with concurrency, timeouts, and retries.

**Files:**
- Create: `/home/synth/projects/agentbench/src/runner.rs`

**Step 1: Create src/runner.rs**

```rust
use std::sync::Arc;
use tokio::sync::{Semaphore, mpsc};
use tokio::time::{timeout, Duration};
use indicatif::{ProgressBar, ProgressStyle};

use crate::error::BenchResult;
use crate::harness::{HarnessAdapter, Task as HarnessTask};
use crate::benchmark::{BenchmarkResult, BenchmarkRunConfig, BenchmarkSuite, BenchmarkTask};
use crate::db::Database;

pub struct Runner {
    db: Arc<Database>,
}

impl Runner {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }
    
    pub async fn run(
        &self,
        harness: &dyn HarnessAdapter,
        suite: &mut dyn BenchmarkSuite,
        config: &BenchmarkRunConfig,
        run_config: &crate::config::BenchmarkConfig,
    ) -> BenchResult<Vec<BenchmarkResult>> {
        let run_id = uuid::Uuid::new_v4().to_string();
        let started_at = chrono::Utc::now();
        
        // Initialize run record in DB
        self.db.create_run(&run_id, &config.harness_name, suite.name(), started_at)?;
        
        let tasks = suite.tasks().to_vec();
        let task_count = config.max_tasks.unwrap_or(tasks.len()).min(tasks.len());
        let tasks_to_run = &tasks[..task_count];
        
        let pb = ProgressBar::new(task_count as u64);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.cyan} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} {msg}")
                .unwrap()
                .progress_chars("=> "),
        );
        
        let max_workers = run_config.runner.max_workers.max(1);
        let semaphore = Arc::new(Semaphore::new(max_workers));
        let timeout_secs = run_config.runner.timeout_secs;
        let retries = run_config.runner.retries;
        
        let (tx, mut rx) = mpsc::channel::<BenchmarkResult>(max_workers);
        
        let mut handles = vec![];
        
        for task in tasks_to_run {
            let permit = semaphore.clone().acquire_owned().await.unwrap();
            let tx = tx.clone();
            let task = task.clone();
            let harness_task = HarnessTask {
                id: task.id.clone(),
                task_type: task.task_type.clone(),
                prompt: task.problem_statement.clone(),
                context: task.metadata.clone(),
                files: task.expected_files.clone(),
                expected_output: None,
            };
            let harness_ref: *const dyn HarnessAdapter = harness;
            let benchmark_name = suite.name().to_string();
            let harness_name = config.harness_name.clone();
            let pb = pb.clone();
            
            let handle = tokio::spawn(async move {
                let _permit = permit;
                pb.set_message(format!("task {}", task.id));
                
                let result = Self::execute_with_retry(
                    harness_ref,
                    &harness_task,
                    &task,
                    &benchmark_name,
                    &harness_name,
                    timeout_secs,
                    retries,
                ).await;
                
                match result {
                    Ok(r) => {
                        let _ = tx.send(r).await;
                    }
                    Err(e) => {
                        let _ = tx.send(BenchmarkResult {
                            task_id: task.id.clone(),
                            harness_name: harness_name.clone(),
                            benchmark_name: benchmark_name.clone(),
                            passed: false,
                            score: 0.0,
                            response: crate::harness::TaskResponse {
                                task_id: task.id.clone(),
                                output: String::new(),
                                patch: None,
                                tool_calls: vec![],
                                metadata: Default::default(),
                                latency_ms: 0,
                                tokens_input: 0,
                                tokens_output: 0,
                            },
                            validation_output: None,
                            error: Some(e.to_string()),
                            started_at: chrono::Utc::now(),
                            finished_at: chrono::Utc::now(),
                        }).await;
                    }
                }
                
                pb.inc(1);
            });
            
            handles.push(handle);
        }
        
        drop(tx);
        
        let mut results = vec![];
        while let Some(result) = rx.recv().await {
            self.db.save_result(&run_id, &result)?;
            results.push(result);
        }
        
        for h in handles {
            let _ = h.await;
        }
        
        pb.finish_with_message("done");
        
        let finished_at = chrono::Utc::now();
        let aggregate = suite.aggregate_score(&results);
        self.db.finish_run(&run_id, finished_at, aggregate, results.len())?;
        
        Ok(results)
    }
    
    async fn execute_with_retry(
        harness: *const dyn HarnessAdapter,
        harness_task: &HarnessTask,
        benchmark_task: &BenchmarkTask,
        benchmark_name: &str,
        harness_name: &str,
        timeout_secs: u64,
        retries: u32,
    ) -> BenchResult<BenchmarkResult> {
        let harness = unsafe { &*harness };
        let mut last_error = None;
        
        for attempt in 0..=retries {
            let started_at = chrono::Utc::now();
            
            let result = timeout(
                Duration::from_secs(timeout_secs),
                harness.execute_task(harness_task)
            ).await;
            
            match result {
                Ok(Ok(response)) => {
                    // TODO: Call suite.validate() here — need to pass suite as param
                    // For now, simple check
                    let passed = !response.output.is_empty();
                    let finished_at = chrono::Utc::now();
                    
                    return Ok(BenchmarkResult {
                        task_id: benchmark_task.id.clone(),
                        harness_name: harness_name.to_string(),
                        benchmark_name: benchmark_name.to_string(),
                        passed,
                        score: if passed { 1.0 } else { 0.0 },
                        response,
                        validation_output: None,
                        error: None,
                        started_at,
                        finished_at,
                    });
                }
                Ok(Err(e)) => {
                    last_error = Some(e);
                }
                Err(_) => {
                    last_error = Some(crate::error::BenchError::TaskExecution(
                        format!("Timeout after {}s (attempt {})", timeout_secs, attempt + 1)
                    ));
                }
            }
        }
        
        Err(last_error.unwrap_or_else(|| crate::error::BenchError::TaskExecution(
            "All retries exhausted".to_string()
        )))
    }
}
```

**Step 2: Add mod runner to main.rs**

```rust
mod runner;
```

**Step 3: Verify**

Run: `cargo check`
Expected: Clean compile.

**Step 4: Commit**

```bash
git add src/runner.rs src/main.rs
git commit -m "feat: add concurrent task runner with semaphore, timeout, retry"
```

---

### Task 11: SQLite database schema

**Objective:** Create the persistent store for benchmark runs and results.

**Files:**
- Create: `/home/synth/projects/agentbench/src/db.rs`

**Step 1: Create src/db.rs**

```rust
use rusqlite::{Connection, params};
use std::sync::Mutex;

use crate::error::BenchResult;
use crate::benchmark::BenchmarkResult;

pub struct Database {
    conn: Mutex<Connection>,
}

impl Database {
    pub fn new(path: &str) -> BenchResult<Self> {
        let conn = Connection::open(path)?;
        let db = Self { conn: Mutex::new(conn) };
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
            "#
        )?;
        Ok(())
    }
    
    pub fn create_run(&self, run_id: &str, harness: &str, benchmark: &str, started_at: chrono::DateTime<chrono::Utc>) -> BenchResult<()> {
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
    
    pub fn finish_run(&self, run_id: &str, finished_at: chrono::DateTime<chrono::Utc>, aggregate_score: f64, tasks_completed: usize) -> BenchResult<()> {
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
    
    pub fn get_results_for_run(&self, run_id: &str) -> BenchResult<Vec<BenchmarkResult>> {
        // TODO: Implement deserialization from DB rows
        Ok(vec![])
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
```

**Step 2: Add mod db to main.rs**

```rust
mod db;
```

**Step 3: Verify**

Run: `cargo check`
Expected: Clean compile.

**Step 4: Commit**

```bash
git add src/db.rs src/main.rs
git commit -m "feat: add SQLite database schema for runs and results"
```

---

### Task 12: Metrics collection

**Objective:** Add structured metrics tracking (tokens, cost, latency) per task and per run.

**Files:**
- Create: `/home/synth/projects/agentbench/src/metrics.rs`

**Step 1: Create src/metrics.rs**

```rust
use serde::{Deserialize, Serialize};

/// Per-task metrics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TaskMetrics {
    pub latency_ms: u64,
    pub tokens_input: u64,
    pub tokens_output: u64,
    pub cost_usd: f64,
}

/// Per-run aggregate metrics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RunMetrics {
    pub total_tasks: usize,
    pub passed_tasks: usize,
    pub failed_tasks: usize,
    pub total_latency_ms: u64,
    pub total_tokens_input: u64,
    pub total_tokens_output: u64,
    pub total_cost_usd: f64,
    pub pass_rate: f64,
    pub avg_latency_ms: f64,
}

impl RunMetrics {
    pub fn from_results(results: &[crate::benchmark::BenchmarkResult]) -> Self {
        let total = results.len();
        let passed = results.iter().filter(|r| r.passed).count();
        
        let total_latency: u64 = results.iter().map(|r| r.response.latency_ms).sum();
        let total_input: u64 = results.iter().map(|r| r.response.tokens_input).sum();
        let total_output: u64 = results.iter().map(|r| r.response.tokens_output).sum();
        
        Self {
            total_tasks: total,
            passed_tasks: passed,
            failed_tasks: total - passed,
            total_latency_ms: total_latency,
            total_tokens_input: total_input,
            total_tokens_output: total_output,
            total_cost_usd: 0.0, // TODO: cost model
            pass_rate: if total > 0 { passed as f64 / total as f64 } else { 0.0 },
            avg_latency_ms: if total > 0 { total_latency as f64 / total as f64 } else { 0.0 },
        }
    }
}

/// Cost model for different providers/models
#[derive(Debug, Clone)]
pub struct CostModel {
    pub cost_per_1k_input: f64,
    pub cost_per_1k_output: f64,
}

impl CostModel {
    pub fn estimate(&self, tokens_input: u64, tokens_output: u64) -> f64 {
        let input_cost = (tokens_input as f64 / 1000.0) * self.cost_per_1k_input;
        let output_cost = (tokens_output as f64 / 1000.0) * self.cost_per_1k_output;
        input_cost + output_cost
    }
}

pub fn get_cost_model(model: &str) -> Option<CostModel> {
    match model {
        "gpt-4" | "gpt-4-turbo" => Some(CostModel { cost_per_1k_input: 0.03, cost_per_1k_output: 0.06 }),
        "gpt-3.5-turbo" => Some(CostModel { cost_per_1k_input: 0.0005, cost_per_1k_output: 0.0015 }),
        "claude-3-opus" => Some(CostModel { cost_per_1k_input: 0.015, cost_per_1k_output: 0.075 }),
        "claude-3-sonnet" => Some(CostModel { cost_per_1k_input: 0.003, cost_per_1k_output: 0.015 }),
        _ => None,
    }
}
```

**Step 2: Add mod metrics to main.rs**

```rust
mod metrics;
```

**Step 3: Verify**

Run: `cargo check`
Expected: Clean compile.

**Step 4: Commit**

```bash
git add src/metrics.rs src/main.rs
git commit -m "feat: add metrics collection (latency, tokens, cost model)"
```

---

## Phase 5: CLI Commands (Days 13-14)

### Task 13: Wire up the `run` command

**Objective:** Make `agentbench run --config ... --harness ...` actually execute a benchmark.

**Files:**
- Modify: `/home/synth/projects/agentbench/src/main.rs`
- Modify: `/home/synth/projects/agentbench/src/cli.rs` (if needed)

**Step 1: Update src/main.rs**

```rust
use clap::Parser;
use std::sync::Arc;

mod benchmark;
mod cli;
mod config;
mod db;
mod error;
mod harness;
mod metrics;
mod runner;

use crate::cli::{Cli, Commands};
use crate::config::BenchmarkConfig;
use crate::db::Database;
use crate::harness::{HarnessRegistry, generic::GenericOpenAIHarness};
use crate::benchmark::{BenchmarkRegistry, swe_bench::SWEBenchSuite};
use crate::runner::Runner;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let args = Cli::parse();
    
    match args.command {
        Commands::Run { config, harness, output } => {
            let bench_config = BenchmarkConfig::from_file(&config)?;
            let db = Arc::new(Database::new("agentbench.db")?);
            
            let mut harness_registry = HarnessRegistry::new();
            harness_registry.register("generic".to_string(), Box::new(GenericOpenAIHarness::new()));
            
            let mut benchmark_registry = BenchmarkRegistry::new();
            benchmark_registry.register("swe_bench".to_string(), Box::new(SWEBenchSuite::new()));
            
            let harness_adapter = harness_registry.get(&harness)
                .ok_or_else(|| anyhow::anyhow!("Harness '{}' not found", harness))?;
            
            let mut harness_mut = harness_registry.get_mut(&harness).unwrap();
            harness_mut.init(crate::harness::HarnessAdapterConfig {
                name: harness.clone(),
                endpoint: bench_config.harness.endpoint.clone(),
                api_key: bench_config.harness.api_key.clone(),
                model: bench_config.harness.model.clone(),
                extra: bench_config.harness.extra.clone().unwrap_or_default(),
            }).await?;
            
            let suite = benchmark_registry.get_mut(&bench_config.benchmark_type)
                .ok_or_else(|| anyhow::anyhow!("Benchmark '{}' not found", bench_config.benchmark_type))?;
            
            suite.load_tasks(&bench_config.dataset).await?;
            
            let run_config = crate::benchmark::BenchmarkRunConfig {
                harness_name: harness.clone(),
                max_tasks: None,
                shuffle: false,
                seed: None,
            };
            
            let runner = Runner::new(db.clone());
            let results = runner.run(harness_adapter, suite, &run_config, &bench_config).await?;
            
            // Output results
            match output.as_str() {
                "json" => {
                    println!("{}", serde_json::to_string_pretty(&results)?);
                }
                "markdown" => {
                    println!("# AgentBench Results\n");
                    println!("| Task | Passed | Score | Latency | Tokens |");
                    println!("|------|--------|-------|---------|--------|");
                    for r in &results {
                        println!("| {} | {} | {:.2} | {}ms | {} |",
                            r.task_id, r.passed, r.score, r.response.latency_ms,
                            r.response.tokens_input + r.response.tokens_output);
                    }
                }
                _ => {
                    println!("AgentBench Results — {} tasks", results.len());
                    let passed = results.iter().filter(|r| r.passed).count();
                    println!("Passed: {}/{} ({:.1}%)", passed, results.len(),
                        if !results.is_empty() { passed as f64 / results.len() as f64 * 100.0 } else { 0.0 });
                }
            }
        }
        Commands::List => {
            println!("Available harnesses: generic");
            println!("Available benchmarks: swe_bench");
        }
        Commands::Tui => {
            println!("TUI not yet implemented. Use 'run' command instead.");
        }
        Commands::Web { port } => {
            println!("Web dashboard not yet implemented. Port: {}", port);
        }
        Commands::Report { run_id, format, output } => {
            println!("Report generation not yet implemented. Run ID: {}", run_id);
        }
    }
    
    Ok(())
}
```

**Step 2: Verify**

Run: `cargo check`
Expected: Clean compile.

**Step 3: Test the CLI**

Run: `cargo run -- list`
Expected:
```
Available harnesses: generic
Available benchmarks: swe_bench
```

**Step 4: Commit**

```bash
git add src/main.rs
git commit -m "feat: wire up run, list, tui, web, report CLI commands"
```

---

### Task 14: Report generation (Markdown, JSON, HTML stubs)

**Objective:** Implement basic report export for completed runs.

**Files:**
- Create: `/home/synth/projects/agentbench/src/report.rs`
- Modify: `/home/synth/projects/agentbench/src/main.rs`

**Step 1: Create src/report.rs**

```rust
use serde::Serialize;

use crate::benchmark::BenchmarkResult;
use crate::metrics::RunMetrics;

#[derive(Debug, Serialize)]
pub struct Report {
    pub run_id: String,
    pub harness_name: String,
    pub benchmark_name: String,
    pub metrics: RunMetrics,
    pub results: Vec<BenchmarkResult>,
}

impl Report {
    pub fn to_markdown(&self) -> String {
        let mut md = format!(
            "# AgentBench Report\n\n**Run ID:** {}\n**Harness:** {}\n**Benchmark:** {}\n\n",
            self.run_id, self.harness_name, self.benchmark_name
        );
        
        md.push_str("## Summary\n\n");
        md.push_str(&format!("- **Total Tasks:** {}\n", self.metrics.total_tasks));
        md.push_str(&format!("- **Passed:** {}\n", self.metrics.passed_tasks));
        md.push_str(&format!("- **Failed:** {}\n", self.metrics.failed_tasks));
        md.push_str(&format!("- **Pass Rate:** {:.1}%\n", self.metrics.pass_rate * 100.0));
        md.push_str(&format!("- **Avg Latency:** {:.0}ms\n", self.metrics.avg_latency_ms));
        md.push_str(&format!("- **Total Tokens:** {}\n", self.metrics.total_tokens_input + self.metrics.total_tokens_output));
        md.push_str("\n");
        
        md.push_str("## Results\n\n");
        md.push_str("| Task | Status | Score | Latency | Input | Output |\n");
        md.push_str("|------|--------|-------|---------|-------|--------|\n");
        
        for r in &self.results {
            let status = if r.passed { "✅ PASS" } else { "❌ FAIL" };
            md.push_str(&format!(
                "| {} | {} | {:.2} | {}ms | {} | {} |\n",
                r.task_id, status, r.score,
                r.response.latency_ms,
                r.response.tokens_input,
                r.response.tokens_output
            ));
        }
        
        md
    }
    
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }
    
    pub fn to_html(&self) -> String {
        format!(
            r#"<!DOCTYPE html>
<html>
<head><title>AgentBench Report</title></head>
<body>
<h1>AgentBench Report</h1>
<p>Run ID: {}</p>
<p>Harness: {}</p>
<p>Benchmark: {}</p>
<h2>Summary</h2>
<ul>
<li>Total Tasks: {}</li>
<li>Passed: {}</li>
<li>Pass Rate: {:.1}%</li>
</ul>
<p><em>HTML report generation is a stub. Full dashboard coming in v0.2.0.</em></p>
</body>
</html>"#,
            self.run_id, self.harness_name, self.benchmark_name,
            self.metrics.total_tasks, self.metrics.passed_tasks,
            self.metrics.pass_rate * 100.0
        )
    }
}
```

**Step 2: Add mod report to main.rs**

```rust
mod report;
```

**Step 3: Verify**

Run: `cargo check`
Expected: Clean compile.

**Step 4: Commit**

```bash
git add src/report.rs src/main.rs
git commit -m "feat: add report generation (markdown, json, html stubs)"
```

---

## Phase 6: TUI Dashboard (Days 15-17)

### Task 15: ratatui app skeleton

**Objective:** Create the TUI application structure with ratatui.

**Files:**
- Create: `/home/synth/projects/agentbench/src/tui/mod.rs`
- Create: `/home/synth/projects/agentbench/src/tui/app.rs`
- Create: `/home/synth/projects/agentbench/src/tui/theme.rs`

**Step 1: Create src/tui/mod.rs**

```rust
pub mod app;
pub mod theme;
pub mod ui;

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    Terminal,
};
use std::io;

use app::App;
use ui::draw;

pub async fn run_tui() -> anyhow::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    
    let app = App::new();
    let res = run_app(&mut terminal, app).await;
    
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    
    if let Err(err) = res {
        println!("Error: {:?}", err);
    }
    
    Ok(())
}

async fn run_app<B: Backend>(terminal: &mut Terminal<B>, mut app: App) -> io::Result<()> {
    let mut last_tick = std::time::Instant::now();
    let tick_rate = std::time::Duration::from_millis(250);
    
    loop {
        terminal.draw(|f| draw(f, &app))?;
        
        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| std::time::Duration::from_secs(0));
        
        if crossterm::event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => return Ok(()),
                    KeyCode::Char('r') => app.refresh(),
                    KeyCode::Up => app.previous(),
                    KeyCode::Down => app.next(),
                    _ => {}
                }
            }
        }
        
        if last_tick.elapsed() >= tick_rate {
            app.on_tick();
            last_tick = std::time::Instant::now();
        }
    }
}
```

**Step 2: Create src/tui/app.rs**

```rust
use crate::db::RunSummary;

pub struct App {
    pub runs: Vec<RunSummary>,
    pub selected: usize,
    pub should_quit: bool,
}

impl App {
    pub fn new() -> Self {
        Self {
            runs: vec![],
            selected: 0,
            should_quit: false,
        }
    }
    
    pub fn refresh(&mut self) {
        // TODO: Load from database
    }
    
    pub fn previous(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }
    
    pub fn next(&mut self) {
        if self.selected < self.runs.len().saturating_sub(1) {
            self.selected += 1;
        }
    }
    
    pub fn on_tick(&mut self) {
        // TODO: Poll for updates
    }
}
```

**Step 3: Create src/tui/theme.rs**

```rust
use ratatui::style::Color;

pub struct SynthwaveTheme;

impl SynthwaveTheme {
    pub const DEEP_PURPLE: Color = Color::Rgb(0x24, 0x00, 0x37);
    pub const ELECTRIC_PURPLE: Color = Color::Rgb(0x8F, 0x00, 0xFF);
    pub const HOT_PINK: Color = Color::Rgb(0xFF, 0x7E, 0xDB);
    pub const MAGENTA: Color = Color::Rgb(0xFF, 0x00, 0xFF);
    pub const NEON_YELLOW: Color = Color::Rgb(0xF3, 0xE7, 0x0F);
    pub const CYAN: Color = Color::Rgb(0x00, 0xFF, 0xFF);
    pub const DARK_BG: Color = Color::Rgb(0x0D, 0x00, 0x1A);
    pub const TEXT: Color = Color::Rgb(0xE0, 0xE0, 0xE0);
    pub const MUTED: Color = Color::Rgb(0x80, 0x80, 0x80);
}
```

**Step 4: Add mod tui to main.rs**

```rust
mod tui;
```

**Step 5: Verify**

Run: `cargo check`
Expected: Clean compile.

**Step 6: Commit**

```bash
git add src/tui/mod.rs src/tui/app.rs src/tui/theme.rs src/main.rs
git commit -m "feat: add ratatui TUI skeleton with synthwave theme"
```

---

### Task 16: TUI layout and widgets

**Objective:** Build the actual TUI layout — header, run list, detail pane, status bar.

**Files:**
- Create: `/home/synth/projects/agentbench/src/tui/ui.rs`

**Step 1: Create src/tui/ui.rs**

```rust
use ratatui::{
    backend::Backend,
    layout::{Alignment, Constraint, Direction, Layout, Margin},
    style::{Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Cell, Clear, Paragraph, Row, Table, Wrap},
    Frame,
};

use super::app::App;
use super::theme::SynthwaveTheme;

pub fn draw<B: Backend>(f: &mut Frame<B>, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(10),
            Constraint::Length(3),
        ])
        .split(f.size());
    
    draw_header(f, chunks[0]);
    draw_main(f, app, chunks[1]);
    draw_footer(f, chunks[2]);
}

fn draw_header<B: Backend>(f: &mut Frame<B>, area: ratatui::layout::Rect) {
    let header = Paragraph::new(Text::from(vec![
        Line::from(vec![
            Span::styled("AgentBench", Style::default().fg(SynthwaveTheme::NEON_YELLOW).add_modifier(Modifier::BOLD)),
            Span::raw(" — "),
            Span::styled("Agent Benchmark Runner", Style::default().fg(SynthwaveTheme::HOT_PINK)),
        ]),
        Line::from(vec![
            Span::styled("v0.1.0", Style::default().fg(SynthwaveTheme::MUTED)),
        ]),
    ]))
    .alignment(Alignment::Center)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(SynthwaveTheme::ELECTRIC_PURPLE))
            .title(Span::styled(" 🎹🦈 ", Style::default().fg(SynthwaveTheme::CYAN)))
    );
    
    f.render_widget(header, area);
}

fn draw_main<B: Backend>(f: &mut Frame<B>, app: &App, area: ratatui::layout::Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);
    
    draw_run_list(f, app, chunks[0]);
    draw_detail_pane(f, app, chunks[1]);
}

fn draw_run_list<B: Backend>(f: &mut Frame<B>, app: &App, area: ratatui::layout::Rect) {
    let header_cells = ["Run ID", "Harness", "Benchmark", "Score", "Status"]
        .iter()
        .map(|h| Cell::from(*h).style(Style::default().fg(SynthwaveTheme::NEON_YELLOW).add_modifier(Modifier::BOLD)));
    let header = Row::new(header_cells)
        .style(Style::default().bg(SynthwaveTheme::DEEP_PURPLE))
        .height(1);
    
    let rows: Vec<Row> = app.runs.iter().enumerate().map(|(i, run)| {
        let style = if i == app.selected {
            Style::default().bg(SynthwaveTheme::ELECTRIC_PURPLE).fg(SynthwaveTheme::TEXT)
        } else {
            Style::default().fg(SynthwaveTheme::TEXT)
        };
        
        let score = run.aggregate_score.map(|s| format!("{:.1}%", s * 100.0)).unwrap_or_else(|| "—".to_string());
        
        Row::new(vec![
            Cell::from(run.id.chars().take(8).collect::<String>()),
            Cell::from(run.harness_name.clone()),
            Cell::from(run.benchmark_name.clone()),
            Cell::from(score),
            Cell::from(run.status.clone()),
        ]).style(style)
    }).collect();
    
    let table = Table::new(rows)
        .header(header)
        .block(
            Block::default()
                .title(" Benchmark Runs ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(SynthwaveTheme::HOT_PINK))
        )
        .widths(&[
            Constraint::Length(10),
            Constraint::Length(15),
            Constraint::Length(15),
            Constraint::Length(10),
            Constraint::Length(12),
        ]);
    
    f.render_widget(table, area);
}

fn draw_detail_pane<B: Backend>(f: &mut Frame<B>, _app: &App, area: ratatui::layout::Rect) {
    let text = Text::from(vec![
        Line::from(Span::styled("Select a run to view details", Style::default().fg(SynthwaveTheme::MUTED))),
        Line::from(""),
        Line::from(vec![
            Span::raw("Press "),
            Span::styled("r", Style::default().fg(SynthwaveTheme::NEON_YELLOW).add_modifier(Modifier::BOLD)),
            Span::raw(" to refresh, "),
            Span::styled("q", Style::default().fg(SynthwaveTheme::NEON_YELLOW).add_modifier(Modifier::BOLD)),
            Span::raw(" to quit"),
        ]),
    ]);
    
    let paragraph = Paragraph::new(text)
        .block(
            Block::default()
                .title(" Details ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(SynthwaveTheme::CYAN))
        )
        .wrap(Wrap { trim: true });
    
    f.render_widget(paragraph, area);
}

fn draw_footer<B: Backend>(f: &mut Frame<B>, area: ratatui::layout::Rect) {
    let footer = Paragraph::new(Text::from(vec![
        Line::from(vec![
            Span::styled("[q]uit", Style::default().fg(SynthwaveTheme::MUTED)),
            Span::raw(" | "),
            Span::styled("[r]efresh", Style::default().fg(SynthwaveTheme::MUTED)),
            Span::raw(" | "),
            Span::styled("↑↓", Style::default().fg(SynthwaveTheme::MUTED)),
            Span::raw(" navigate"),
        ]),
    ]))
    .alignment(Alignment::Center)
    .block(
        Block::default()
            .borders(Borders::TOP)
            .border_style(Style::default().fg(SynthwaveTheme::ELECTRIC_PURPLE))
    );
    
    f.render_widget(footer, area);
}
```

**Step 2: Verify**

Run: `cargo check`
Expected: Clean compile.

**Step 3: Commit**

```bash
git add src/tui/ui.rs
git commit -m "feat: add TUI layout with header, run list, detail pane, footer"
```

---

## Phase 7: Web Dashboard (Days 18-19)

### Task 17: Axum web server stub

**Objective:** Create the embedded Axum server for the web dashboard.

**Files:**
- Create: `/home/synth/projects/agentbench/src/web/mod.rs`
- Create: `/home/synth/projects/agentbench/src/web/routes.rs`
- Create: `/home/synth/projects/agentbench/src/web/state.rs`

**Step 1: Create src/web/mod.rs**

```rust
pub mod routes;
pub mod state;

use axum::{
    routing::get,
    Router,
};
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
    println!("AgentBench web dashboard running on http://0.0.0.0:{}", port);
    
    axum::serve(listener, app).await?;
    Ok(())
}
```

**Step 2: Create src/web/state.rs**

```rust
use std::sync::Arc;
use crate::db::Database;

#[derive(Clone)]
pub struct AppState {
    pub db: Arc<Database>,
}
```

**Step 3: Create src/web/routes.rs**

```rust
use axum::{
    extract::{Path, State},
    response::Html,
    Json,
};
use std::sync::Arc;

use super::state::AppState;

pub async fn index() -> Html<&'static str> {
    Html(r#"
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
    "#)
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

pub async fn get_run(State(state): State<Arc<AppState>>, Path(id): Path<String>) -> Json<serde_json::Value> {
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
```

**Step 4: Add mod web to main.rs**

```rust
mod web;
```

**Step 5: Verify**

Run: `cargo check`
Expected: Clean compile.

**Step 6: Commit**

```bash
git add src/web/mod.rs src/web/state.rs src/web/routes.rs src/main.rs
git commit -m "feat: add Axum web dashboard stub with API routes"
```

---

## Phase 8: CI/CD & Polish (Days 20-21)

### Task 18: GitHub Actions CI

**Objective:** Add CI workflow for check, fmt, clippy, test.

**Files:**
- Create: `/home/synth/projects/agentbench/.github/workflows/ci.yml`

**Step 1: Create .github/workflows/ci.yml**

```yaml
name: CI

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

env:
  CARGO_TERM_COLOR: always

jobs:
  check:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      - run: cargo check --all-targets

  fmt:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: rustfmt
      - run: cargo fmt --all -- --check

  clippy:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: clippy
      - uses: Swatinem/rust-cache@v2
      - run: cargo clippy --all-targets -- -D warnings

  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      - run: cargo test
```

**Step 2: Commit**

```bash
git add .github/workflows/ci.yml
git commit -m "ci: add GitHub Actions workflow for check, fmt, clippy, test"
```

---

### Task 19: GitHub Actions release workflow

**Objective:** Add release workflow triggered by version tags.

**Files:**
- Create: `/home/synth/projects/agentbench/.github/workflows/release.yml`

**Step 1: Create .github/workflows/release.yml**

```yaml
name: Release

on:
  push:
    tags:
      - 'v*'

permissions:
  contents: write

jobs:
  build:
    strategy:
      matrix:
        target:
          - x86_64-unknown-linux-gnu
          - x86_64-apple-darwin
          - aarch64-apple-darwin
          - x86_64-pc-windows-msvc
    runs-on: ${{ matrix.target == 'x86_64-apple-darwin' && 'macos-latest' || matrix.target == 'aarch64-apple-darwin' && 'macos-latest' || matrix.target == 'x86_64-pc-windows-msvc' && 'windows-latest' || 'ubuntu-latest' }}
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: ${{ matrix.target }}
      - uses: Swatinem/rust-cache@v2
      - run: cargo build --release --target ${{ matrix.target }}
      - name: Package
        shell: bash
        run: |
          cd target/${{ matrix.target }}/release
          if [[ "${{ matrix.target }}" == *windows* ]]; then
            7z a ../../../agentbench-${{ matrix.target }}.zip agentbench.exe
          else
            tar czvf ../../../agentbench-${{ matrix.target }}.tar.gz agentbench
          fi
          cd -
      - name: Upload artifact
        uses: actions/upload-artifact@v4
        with:
          name: agentbench-${{ matrix.target }}
          path: agentbench-${{ matrix.target }}.*

  release:
    needs: build
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/download-artifact@v4
        with:
          path: artifacts
      - name: Create Release
        uses: softprops/action-gh-release@v2
        with:
          files: artifacts/**/*
          generate_release_notes: true
```

**Step 2: Commit**

```bash
git add .github/workflows/release.yml
git commit -m "ci: add release workflow with multi-target builds"
```

---

### Task 20: README and LICENSE

**Objective:** Add project README and MIT license.

**Files:**
- Create: `/home/synth/projects/agentbench/README.md`
- Create: `/home/synth/projects/agentbench/LICENSE`

**Step 1: Create README.md**

```markdown
# 🎹🦈 AgentBench

Open-source, self-hostable benchmark runner for AI coding agents.

## Features

- **Pluggable Harnesses** — Benchmark any agent: OpenShark, Hermes, Claude Code, Codex, or generic OpenAI-compatible APIs
- **Benchmark Suites** — SWE-bench, Terminal-bench, LiveCodeBench (extensible)
- **Concurrent Execution** — Semaphore-based parallelism with timeouts and retries
- **Persistent Results** — SQLite store with queryable history
- **Beautiful TUI** — Real-time ratatui dashboard with synthwave aesthetic
- **Web Dashboard** — Embedded Axum server for result visualization
- **Report Export** — Markdown, JSON, HTML

## Quick Start

```bash
# Clone
git clone https://github.com/synthalorian/agentbench.git
cd agentbench

# Build
cargo build --release

# Run a benchmark
cargo run -- run --config benches/swe-bench-lite.yml --harness generic

# List available harnesses and benchmarks
cargo run -- list

# Start TUI dashboard
cargo run -- tui

# Start web dashboard
cargo run -- web --port 8910
```

## Configuration

Benchmark suites are defined in YAML:

```yaml
name: "SWE-bench Lite"
benchmark_type: "swe_bench"
dataset:
  source: "local"
  path: "./data/swe-bench-lite.json"
harness:
  adapter: "generic"
  endpoint: "http://localhost:8080/v1"
  model: "local-model"
runner:
  max_workers: 4
  timeout_secs: 300
```

## Architecture

```
┌─────────┐    ┌─────────────┐    ┌──────────────┐
│   CLI   │───▶│    Runner   │───▶│  Harness     │
│  / TUI  │    │  (tokio)    │    │  Adapters    │
└─────────┘    └─────────────┘    └──────────────┘
                      │
                      ▼
               ┌──────────────┐
               │  Benchmark   │
               │   Suites     │
               └──────────────┘
                      │
                      ▼
               ┌──────────────┐
               │   SQLite     │
               │   Results    │
               └──────────────┘
```

## License

MIT — see [LICENSE](LICENSE)

Made by synth with synthshark
```

**Step 2: Create LICENSE**

```
MIT License

Copyright (c) 2026 synth (synthalorian)

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```

**Step 3: Commit**

```bash
git add README.md LICENSE
git commit -m "docs: add README and MIT license"
```

---

## Phase 9: Final Verification (Day 22)

### Task 21: End-to-end verification

**Objective:** Run the full toolchain — cargo check, fmt, clippy, test, build — and verify the binary works.

**Step 1: Run quality checks**

```bash
cd /home/synth/projects/agentbench
cargo fmt --all
cargo clippy --all-targets -- -D warnings
cargo test
cargo build --release
```

**Step 2: Test the binary**

```bash
./target/release/agentbench list
```

Expected:
```
Available harnesses: generic
Available benchmarks: swe_bench
```

**Step 3: Commit final state**

```bash
git add -A
git commit -m "chore: v0.1.0 release candidate"
```

---

## v0.2.0 Roadmap (Post-MVP)

1. **Real SWE-bench validation** — Docker container execution, test patch application, pytest runner
2. **Terminal-bench implementation** — Shell command validation, output comparison
3. **LiveCodeBench implementation** — Code compilation + test execution
4. **Full harness adapters** — OpenShark, Hermes, Claude Code, Codex native integration
5. **TUI data binding** — Live database polling, real run display
6. **Web dashboard charts** — Trend lines, comparison tables, per-task detail
7. **Cost tracking** — Per-model cost estimation, budget alerts
8. **HuggingFace datasets integration** — Direct loading without manual download
9. **Custom benchmark definitions** — User-defined YAML benchmark suites
10. **Plugin system** — Third-party harness and benchmark adapters

---

## Development Notes

### Adding a new harness adapter

1. Create `src/harness/your_harness.rs`
2. Implement `HarnessAdapter` trait
3. Register in `src/harness/mod.rs`
4. Add to registry in `src/main.rs`

### Adding a new benchmark suite

1. Create `src/benchmark/your_suite.rs`
2. Implement `BenchmarkSuite` trait
3. Register in `src/benchmark/mod.rs`
4. Add to registry in `src/main.rs`

### Running with a local model

```bash
# Start llama.cpp server
./server -m model.gguf --port 8080

# Run benchmark
agentbench run --config benches/swe-bench-lite.yml --harness generic
```

---

*Plan written: 2026-06-01*
*Target: v0.1.0 MVP in ~3 weeks (22 days of focused work)*
*Made by synth with synthshark* 🎹🦈
