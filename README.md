# Log Intelligence System

AI-powered log analysis with semantic search, anomaly detection, and root cause analysis.

## Status: Phase 1 Complete ✅ | Phase 2 Next 🚀

## Architecture

```
┌─────────────┐     ┌──────────┐     ┌─────────────┐
│  logai-api  │────▶│   NATS   │────▶│ logai-worker│
│  (HTTP)     │     │  (Queue) │     │ (Processing)│
│  :3000      │     │  :4222   │     │             │
└─────────────┘     └──────────┘     └──────┬──────┘
                                           │
                    ┌──────────────────────┼──────────────────────┐
                    │                      │                      │
                    ▼                      ▼                      ▼
              ┌──────────┐          ┌──────────┐          ┌──────────┐
              │ClickHouse│          │  Qdrant  │          │  Ollama  │
              │  :8123   │          │  :6333   │          │  :11434  │
              │ (Logs) ✅ │          │(Vectors) │          │  (LLM)   │
              └──────────┘          └──────────┘          └──────────┘
```

## Tech Stack

| Component | Technology | Status |
|-----------|------------|--------|
| Language | Rust | ✅ |
| HTTP Server | Axum 0.8 | ✅ |
| Message Queue | NATS 2.10 | ✅ |
| Log Storage | ClickHouse 24.1 | ✅ |
| Vector DB | Qdrant 1.7 | 🔜 Phase 2 |
| Embeddings | FastEmbed (384D) | 🔜 Phase 2 |
| LLM | Ollama (local) | 🔜 Phase 4 |

## Project Structure

```
log-intelligence/
├── Cargo.toml              # Workspace root
├── docker-compose.yml      # NATS, ClickHouse, Qdrant
└── crates/
    ├── logai-core/         # Shared types (LogEntry, LogLevel, etc.)
    ├── logai-api/          # HTTP API (POST /api/logs)
    ├── logai-worker/       # NATS consumer → ClickHouse
    └── logai-cli/          # CLI tool (coming soon)
```

## Phase Progress

### ✅ Phase 1: Foundation & Ingestion (Complete)
- [x] Project setup with Rust workspace
- [x] Log data models (LogEntry, RawLogEntry, LogLevel, ErrorCategory, LogChunk)
- [x] HTTP Ingestion API (POST /api/logs)
- [x] NATS integration (publish/subscribe)
- [x] ClickHouse storage (11 columns)
- [ ] Basic CLI (optional)

### 🔜 Phase 2: Vector Search & Embeddings
- [ ] Qdrant collection setup
- [ ] Chunking strategy (time-window grouping)
- [ ] FastEmbed integration (384D vectors)
- [ ] Hybrid search API (semantic + filters)

### 📋 Phase 3: Anomaly Detection & Alerting
- [ ] Statistical anomaly detection
- [ ] Slack integration
- [ ] Alert management API

### 📋 Phase 4: RAG Query Engine
- [ ] Ollama/LLM integration
- [ ] Natural language queries
- [ ] Answer generation with sources

### 📋 Phase 5: React Dashboard
- [ ] Log explorer with filters
- [ ] Real-time streaming
- [ ] AI chat interface

### 📋 Phase 6: Production Polish
- [ ] Authentication
- [ ] Performance optimization
- [ ] Docker packaging

## Quick Start

### Prerequisites
- Rust 1.75+
- Docker & Docker Compose

### 1. Start Infrastructure
```bash
docker-compose up -d
```

### 2. Verify Services
```bash
curl http://localhost:8222/healthz   # NATS
curl http://localhost:8123/ping      # ClickHouse
curl http://localhost:6333/collections  # Qdrant
```

### 3. Build & Run

**Terminal 1 - Worker:**
```bash
cargo run --bin logai-worker
```

**Terminal 2 - API:**
```bash
cargo run --bin logai-api
```

### 4. Send Test Log
```bash
curl -X POST http://localhost:3000/api/logs \
  -H "Content-Type: application/json" \
  -d '{"message": "Payment failed for user 123", "level": "error", "service": "payment-service"}'
```

### 5. Verify in ClickHouse
```bash
curl "http://localhost:8123" -d "SELECT * FROM logai.logs FORMAT Pretty"
```

## API Endpoints

| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/api/logs` | Ingest single log entry |

**Request:**
```json
{
  "message": "Error message here",
  "level": "error",
  "service": "my-service",
  "trace_id": "optional-trace-id",
  "fields": {"key": "value"}
}
```

**Response:**
```json
{
  "id": "uuid-here",
  "status": "accepted"
}
```

## Data Flow

```
1. Client POST /api/logs
          │
          ▼
2. logai-api receives JSON
   • Parse → RawLogEntry
   • Enrich → LogEntry (add id, timestamps)
   • Publish to NATS "logs.ingest"
   • Return {id, status: "accepted"}
          │
          ▼
3. NATS queue holds message
          │
          ▼
4. logai-worker subscribes
   • Receive from NATS
   • Parse → LogEntry
   • INSERT into ClickHouse
          │
          ▼
5. ClickHouse stores log
   • 11 columns (id, timestamp, level, service, message, ...)
   • Partitioned by month
   • Sorted by (service, timestamp)
```

## Performance

- **Ingestion latency:** ~4ms (API to ClickHouse)
- **Throughput:** Tested up to 1000 logs/sec (single worker)

## License

MIT
