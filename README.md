# Log Intelligence System

AI-powered log analysis with semantic search, anomaly detection, and root cause analysis.

## Status: Phase 2 Complete ✅ | Phase 3 Next 🚀

## Architecture

```
┌─────────────┐     ┌──────────┐     ┌─────────────┐
│  logai-api  │────▶│   NATS   │────▶│ logai-worker│
│  (HTTP)     │     │  (Queue) │     │ (Processing)│
│  :3000      │     │  :4222   │     │             │
└──────┬──────┘     └──────────┘     └──────┬──────┘
       │                                    │
       │ Search                    Store + Embed
       │                                    │
       ▼                    ┌───────────────┼───────────────┐
┌──────────┐                │               │               │
│  Qdrant  │◀───────────────┤               ▼               ▼
│  :6334   │           ┌──────────┐   ┌──────────┐   ┌──────────┐
│(Vectors) │           │ClickHouse│   │  Qdrant  │   │  Ollama  │
└──────────┘           │  :8123   │   │  :6334   │   │  :11434  │
                       │ (Logs) ✅ │   │(Vectors)✅│   │  (LLM)   │
                       └──────────┘   └──────────┘   └──────────┘
```

## Tech Stack

| Component | Technology | Status |
|-----------|------------|--------|
| Language | Rust | ✅ |
| HTTP Server | Axum 0.8 | ✅ |
| Message Queue | NATS 2.10 | ✅ |
| Log Storage | ClickHouse 24.1 | ✅ |
| Vector DB | Qdrant 1.15 | ✅ |
| Embeddings | FastEmbed (384D) | ✅ |
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

### ✅ Phase 2: Vector Search & Embeddings (Complete)
- [x] Qdrant collection setup (384D vectors, Cosine distance)
- [x] FastEmbed integration (AllMiniLML6V2 model)
- [x] Embedding generation in worker
- [x] Vector storage with payload metadata
- [x] Semantic search API (GET /api/search)

### 🔜 Phase 3: Anomaly Detection & Alerting
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

### 4. Send Test Logs
```bash
# Error log
curl -X POST http://localhost:3000/api/logs \
  -H "Content-Type: application/json" \
  -d '{"message": "Database connection timeout after 30 seconds", "service": "payment-api", "level": "error"}'

# Info log
curl -X POST http://localhost:3000/api/logs \
  -H "Content-Type: application/json" \
  -d '{"message": "User login successful", "service": "auth-service", "level": "info"}'
```

### 5. Semantic Search
```bash
# Search for timeout-related logs
curl "http://localhost:3000/api/search?q=timeout%20error" | jq
```

### 6. Verify in ClickHouse
```bash
curl "http://localhost:8123" -d "SELECT * FROM logai.logs FORMAT Pretty"
```

## API Endpoints

| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/api/logs` | Ingest single log entry |
| GET | `/api/search?q=query&limit=5` | Semantic search logs |

### POST /api/logs - Ingest Log

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

### GET /api/search - Semantic Search

**Request:**
```bash
curl "http://localhost:3000/api/search?q=timeout%20error&limit=5"
```

**Response:**
```json
[
  {
    "score": 0.496,
    "log_id": "cc19dfea-78b0-49c6-a0f1-f88f8926485b",
    "service": "order-service",
    "level": "Error",
    "message": "Request timeout while connecting to database",
    "timestamp": "2026-02-06T14:17:02.764066+00:00"
  },
  {
    "score": 0.465,
    "log_id": "219ff38f-0fac-43a0-9119-77b915bb2c29",
    "service": "payment-api",
    "level": "Error",
    "message": "Database connection timeout after 30 seconds",
    "timestamp": "2026-02-06T09:33:20.350477+00:00"
  }
]
```

## Data Flow

### Ingestion Flow
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
   • Generate embedding (384D vector)
   • Store in Qdrant with metadata
          │
          ▼
5. Data stored in:
   • ClickHouse: Full log data (11 columns)
   • Qdrant: Vector + payload (for search)
```

### Search Flow
```
1. Client GET /api/search?q=timeout error
          │
          ▼
2. logai-api processes query
   • Embed query → 384D vector
   • Search Qdrant (cosine similarity)
   • Return ranked results with scores
          │
          ▼
3. Results ordered by similarity
   • Higher score = more relevant
   • Includes metadata (service, level, message, timestamp)
```

## Performance

- **Ingestion latency:** ~4ms (API to ClickHouse + Qdrant)
- **Embedding generation:** ~10ms per log (AllMiniLML6V2)
- **Search latency:** <50ms (vector similarity search)
- **Throughput:** Tested up to 1000 logs/sec (single worker)

## License

MIT
