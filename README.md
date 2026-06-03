# AgentBench

Open-source, self-hostable benchmark runner for AI coding agents.

## Features

- **Pluggable Harnesses** — Benchmark any agent: OpenShark, Hermes, Claude Code, Codex, OpenCode, or generic OpenAI-compatible APIs
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

# List available harnesses and benchmarks
./target/release/agentbench list

# Run the sample benchmark (uses local sample data, no API needed)
./target/release/agentbench run \
  --config benches/sample-benchmark.yml \
  --harness generic

# Dry-run mode — test without any external API calls
./target/release/agentbench run \
  --config benches/sample-benchmark.yml \
  --harness generic \
  --dry-run

# Start TUI dashboard
./target/release/agentbench tui

# Start web dashboard
./target/release/agentbench web --port 8910

# Generate a report from a previous run
./target/release/agentbench report <run-id>
```

## Configuration

Benchmark suites are defined in YAML. See `benches/sample-benchmark.yml`:

```yaml
name: "AgentBench Sample"
description: "Sample benchmark for testing AgentBench with local data"
benchmark_type: "swe_bench"

dataset:
  source: "local"
  path: "./data/swe-bench-sample.json"

harness:
  name: "generic-openai"
  adapter: "generic"
  endpoint: "http://localhost:8080/v1"
  model: "local-model"
  extra:
    max_tokens: 4096
    temperature: 0.0

runner:
  max_workers: 2
  timeout_secs: 60
  retries: 0

scoring:
  metric: "pass_rate"
  thresholds:
    excellent: 0.80
    good: 0.50
    acceptable: 0.20
```

### Harness Adapters

| Adapter | Description | Config |
|---------|-------------|--------|
| `generic` | OpenAI-compatible API | `endpoint`, `api_key`, `model` |
| `mock` | Dry-run / testing | No config needed |
| `openshark` | OpenShark harness | `endpoint`, `api_key` |
| `hermes` | Hermes agent | `endpoint` or CLI path |
| `claude_code` | Claude Code CLI | workspace directory |
| `codex` | OpenAI Codex | `api_key` |
| `opencode` | OpenCode CLI/API | `endpoint`, `api_key` |

### Benchmark Types

| Type | Description | Validation |
|------|-------------|------------|
| `swe_bench` | Software engineering tasks | Docker + pytest |
| `terminal_bench` | Shell command tasks | stdout matching |
| `livecodebench` | Live coding challenges | Compile + test |

## Running Tests

```bash
# Run all tests
cargo test

# Run with output
cargo test -- --nocapture

# Run a specific test
cargo test test_swe_bench_load_tasks -- --nocapture
```

## Project Structure

```
agentbench/
├── benches/          # Benchmark config YAML files
├── data/             # Sample datasets for testing
├── src/
│   ├── benchmark/    # Benchmark suite implementations
│   ├── harness/      # Harness adapters
│   ├── tui/          # Ratatui dashboard
│   ├── web/          # Axum web server
│   ├── config.rs     # YAML config loading
│   ├── db.rs         # SQLite persistence
│   ├── runner.rs     # Concurrent task execution
│   ├── metrics.rs    # Cost tracking
│   └── report.rs     # Report generation
└── tests/            # Integration tests
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
