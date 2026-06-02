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
