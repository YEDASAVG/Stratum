# Log Intelligence System

AI-powered log analysis with semantic search, anomaly detection, and root cause analysis.

## Status: 🚧 Work in Progress

## Architecture

```
┌─────────────┐     ┌──────────┐     ┌─────────────┐
│  logai-api  │────▶│   NATS   │────▶│ logai-worker│
│  (HTTP)     │     │  (Queue) │     │ (Processing)│
└─────────────┘     └──────────┘     └──────┬──────┘
                                           │
                    ┌──────────────────────┼──────────────────────┐
                    │                      │                      │
                    ▼                      ▼                      ▼
              ┌──────────┐          ┌──────────┐          ┌──────────┐
              │ClickHouse│          │  Qdrant  │          │  Ollama  │
              │ (Logs)   │          │ (Vectors)│          │  (LLM)   │
              └──────────┘          └──────────┘          └──────────┘
```

## Tech Stack

- **Language:** Rust
- **HTTP:** Axum
- **Queue:** NATS
- **Storage:** ClickHouse (logs), Qdrant (vectors)
- **Embeddings:** FastEmbed (384D)
- **LLM:** Ollama (local)

## Project Structure

```
crates/
├── logai-core/    # Shared types
├── logai-api/     # HTTP ingestion API
├── logai-worker/  # Background processing
└── logai-cli/     # CLI tool
```

## Quick Start

```bash
# Build
cargo build

# Run API server
cargo run --bin logai-api

# Test
curl -X POST http://localhost:3000/api/logs \
  -H "Content-Type: application/json" \
  -d '{"message": "Test log", "level": "info", "service": "test"}'
```

## License

MIT
