# LogAI Benchmarks

Performance benchmarks for the LogAI log intelligence system.

## Benchmark Types

### 1. Criterion Microbenchmarks

CPU-bound operations measured with [Criterion.rs](https://github.com/bheisler/criterion.rs):

| Benchmark | Description |
|-----------|-------------|
| `parsing` | Log parser performance (Apache, Nginx, Syslog) |
| `rag` | Query analysis, intent detection, reranking |

### 2. Stress Test (CLI)

High-throughput ingestion testing using the built-in `logai-stress` tool.

### 3. k6 Load Test

API endpoint load testing with mixed workloads using [k6](https://k6.io/).

---

## Quick Start

```bash
# Make script executable
chmod +x benches/run-benchmarks.sh

# Run all benchmarks
./benches/run-benchmarks.sh --all

# Run only Criterion microbenchmarks
./benches/run-benchmarks.sh --criterion

# Run only stress test
./benches/run-benchmarks.sh --stress

# Run only k6 load test
./benches/run-benchmarks.sh --k6
```

---

## Criterion Benchmarks

### Running

```bash
# Run all criterion benchmarks
cargo bench -p logai-core --bench parsing
cargo bench -p logai-rag --bench rag

# Run specific benchmark
cargo bench -p logai-core --bench parsing -- apache_parser
cargo bench -p logai-rag --bench rag -- reranking

# Run with baseline comparison
cargo bench -p logai-core --bench parsing -- --save-baseline main
cargo bench -p logai-core --bench parsing -- --baseline main
```

### Benchmarks Included

#### Parsing Benchmarks (`parsing.rs`)

| Benchmark | Description |
|-----------|-------------|
| `apache_parser_single` | Parse single Apache log line |
| `nginx_parser_single` | Parse single Nginx log line |
| `syslog_parser_single` | Parse single Syslog line |
| `parser_registry/*` | Parser lookup + parse |
| `raw_to_log_entry` | RawLogEntry → LogEntry conversion |
| `batch_parsing/nginx/*` | Batch parsing (10, 100, 1000, 10000 logs) |
| `serialization/*` | JSON serialize/deserialize |

#### RAG Benchmarks (`rag.rs`)

| Benchmark | Description |
|-----------|-------------|
| `query_analysis/*` | Time/service extraction (regex) |
| `reranking/rerank_logs/*` | Rerank 10-1000 logs |
| `intent_detection` | Classify query intent |
| `query_cleaning` | Remove filler words from queries |

### Output

Reports are saved to `target/criterion/report/index.html`.

---

## Stress Test

Built-in high-performance ingestion tester.

### Running

```bash
# Build release binary
cargo build --release --bin logai-stress

# Run with defaults (100K logs, 50K/sec target)
cargo run --release --bin logai-stress

# Custom parameters
cargo run --release --bin logai-stress -- \
  --rate 100000 \
  --total 1000000 \
  --batch 1000 \
  --workers 100 \
  --endpoint http://localhost:3000 \
  --format structured
```

### Options

| Option | Default | Description |
|--------|---------|-------------|
| `--rate` | 100000 | Target logs/second (0 = unlimited) |
| `--total` | 1000000 | Total logs to send |
| `--batch` | 1000 | Logs per HTTP request |
| `--workers` | 100 | Concurrent connections |
| `--endpoint` | localhost:3000 | API URL |
| `--format` | structured | Log format: `structured`, `apache`, `nginx`, `syslog` |

### Output

```
╔═══════════════════════════════════════════════════════════╗
║                    STRESS TEST RESULTS                     ║
╠═══════════════════════════════════════════════════════════╣
║ Total Sent:        1000000 logs                          ║
║ Successful:        1000000 logs                          ║
║ Failed:                  0 logs                          ║
║ Duration:            10.23 seconds                       ║
║ Throughput:          97752 logs/sec                      ║
║ Avg Latency:         12.34 ms                            ║
║ Success Rate:       100.00%                              ║
╚═══════════════════════════════════════════════════════════╝
```

---

## k6 Load Test

API endpoint testing with realistic mixed workloads.

### Prerequisites

```bash
# macOS
brew install k6

# Linux
sudo apt install k6
```

### Running

```bash
# Default (60s, mixed workload)
k6 run benches/loadtest.js

# Custom
k6 run --vus 100 --duration 5m benches/loadtest.js

# With custom API URL
API_URL=http://localhost:8080 k6 run benches/loadtest.js
```

### Scenarios

| Scenario | VUs | Duration | Description |
|----------|-----|----------|-------------|
| ingestion | 20 | 60s | Batch log ingestion (100 logs/request) |
| search | 10 | 60s | Semantic search queries |
| ask | 5 rps | 60s | AI question answering |

### Thresholds

| Metric | Threshold |
|--------|-----------|
| Ingestion p95 | < 500ms |
| Search p95 | < 200ms |
| Error rate | < 1% |

---

## Performance Targets

Based on architecture design goals:

| Metric | Target | Notes |
|--------|--------|-------|
| **Ingestion throughput** | 10K logs/sec/instance | HTTP API intake |
| **NATS → ClickHouse** | 50K logs/sec | Batch inserts |
| **Semantic search p99** | < 100ms | Qdrant HNSW |
| **ClickHouse filter** | < 50ms | Columnar storage |
| **AI question answering** | < 5s | With Groq API |
| **Parser throughput** | > 500K logs/sec | Per parser |
| **Memory usage** | < 200MB | API server |

---

## Profiling

### Flamegraph

```bash
# Install
cargo install flamegraph

# Profile
cargo flamegraph --bench parsing -- --bench
```

### DHAT (Heap Profiling)

```bash
# Add to Cargo.toml
[profile.bench]
debug = true

# Run with DHAT
cargo bench --bench parsing 2>&1 | head -100
```

---

## CI Integration

Add to `.github/workflows/bench.yml`:

```yaml
name: Benchmarks
on:
  push:
    branches: [main]
  pull_request:

jobs:
  benchmark:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      
      - name: Run benchmarks
        run: cargo bench --bench parsing --bench rag -- --noplot
        
      - name: Upload results
        uses: actions/upload-artifact@v4
        with:
          name: criterion-report
          path: target/criterion
```

---

## Comparing Results

```bash
# Save baseline
cargo bench -- --save-baseline before

# Make changes...

# Compare
cargo bench -- --baseline before

# Generate comparison HTML
cargo bench -- --baseline before --output-format html
```

---

## Results (Sample Run)

> MacBook Pro M2, 16GB RAM, Feb 2026

### Parsing

| Benchmark | Time | Throughput |
|-----------|------|------------|
| apache_parser_single | 1.6 µs | 625K/s |
| nginx_parser_single | 0.9 µs | 1.1M/s |
| syslog_parser_single | 1.6 µs | 625K/s |
| raw_to_log_entry | 1.2 µs | 833K/s |
| batch_parsing/nginx/10 | 8.9 µs | 1.1M elem/s |
| batch_parsing/nginx/100 | 88 µs | 1.1M elem/s |
| batch_parsing/nginx/1000 | 920 µs | 1.1M elem/s |
| batch_parsing/nginx/10000 | 9.2 ms | 1.1M elem/s |
| log_entry_to_json | 780 ns | 1.3M/s |
| json_to_log_entry | 1.1 µs | 910K/s |

### RAG

| Benchmark | Time | Throughput |
|-----------|------|------------|
| intent_detection (10 queries) | 1.8 µs | 5.5M queries/s |
| query_cleaning (10 queries) | 5.0 µs | 2M queries/s |
| reranking/10 logs | 2.0 µs | 5.0M elem/s |
| reranking/50 logs | 10 µs | 5.0M elem/s |
| reranking/100 logs | 19 µs | 5.3M elem/s |
| reranking/500 logs | 92 µs | 5.4M elem/s |
| reranking/1000 logs | 199 µs | 5.0M elem/s |
| service_extraction | 20 ns | 50M/s |
| time_extraction | 110 ns | 9M/s |

### Stress Test (Target)

| Metric | Value |
|--------|-------|
| Throughput | 85,000 logs/sec |
| Latency p50 | 8 ms |
| Latency p99 | 45 ms |
| Success rate | 99.99% |
