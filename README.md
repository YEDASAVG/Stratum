# LogAI - AI-Powered Log Analysis Platform

Real-time log intelligence with semantic search, anomaly detection, and natural language queries powered by AI.

## Status: All Phases Complete ✅

## Features

- **Semantic Search** - Find logs by meaning, not just keywords
- **AI-Powered Analysis** - Ask questions about your logs in natural language  
- **Multi-Format Support** - Parse Apache, Nginx, Syslog, and JSON logs
- **Real-time Ingestion** - Stream logs via NATS messaging
- **Anomaly Detection** - Automatic detection of unusual patterns
- **Beautiful CLI** - Full-featured command-line interface
- **API Key Security** - Optional authentication for production

## Architecture

```
┌─────────────┐     ┌────────┐     ┌────────────┐     ┌─────────────┐
│  Log Files  │────▶│  API   │────▶│    NATS    │────▶│  ClickHouse │
│  or Streams │     │ Server │     │  (Ingest)  │     │  (Storage)  │
└─────────────┘     └────────┘     └────────────┘     └─────────────┘
                         │                                    │
                         ▼                                    │
                    ┌─────────┐                               │
                    │ Qdrant  │◀──────────────────────────────┘
                    │(Vectors)│       Vector Embeddings
                    └─────────┘
                         │
                         ▼
                    ┌─────────┐
                    │  Groq   │  Fast LLM for analysis
                    │   AI    │
                    └─────────┘
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
| LLM | Groq (llama3-70b) | ✅ |
| CLI | Clap + Colored | ✅ |

## Project Structure

```
log-intelligence/
├── Cargo.toml              # Workspace root
├── docker-compose.yml      # NATS, ClickHouse, Qdrant
├── start.sh                # One-click start script
├── stop.sh                 # Stop all services
└── crates/
    ├── logai-core/         # Shared types + log parsers
    ├── logai-api/          # HTTP API server
    ├── logai-ingest/       # NATS consumer → storage
    ├── logai-rag/          # RAG engine for AI queries
    └── logai-cli/          # CLI tool (logai)
```

## Quick Start

### Prerequisites
- Rust 1.75+
- Docker & Docker Compose
- Groq API key (free at [console.groq.com](https://console.groq.com))

### 1. Configure Environment

```bash
# Copy example env file
cp .env.example .env

# Edit and add your Groq API key
nano .env
```

### 2. One-Click Start

```bash
./start.sh
```

This will:
- Start infrastructure (NATS, ClickHouse, Qdrant)
- Build all components
- Launch API server and ingestion worker

### 3. Use the CLI

```bash
# Check system status
./target/release/logai status

# Search logs
./target/release/logai search "authentication failed"

# Ask AI about your logs
./target/release/logai ask "What are the most common errors?"

# Ingest logs from a file
./target/release/logai ingest /var/log/nginx/access.log --format nginx --service my-nginx

# View recent logs
./target/release/logai logs --limit 20
```

## CLI Reference

### Global Options

```
-a, --api-url <URL>    API server URL [default: http://localhost:3000]
-k, --api-key <KEY>    API key (or set LOGAI_API_KEY env var)
```

### Commands

| Command | Description |
|---------|-------------|
| `logai status` | Check health of all system components |
| `logai search <query>` | Semantic search for logs |
| `logai ask <question>` | Ask AI about your logs |
| `logai ingest <file>` | Import logs from a file |
| `logai logs` | Show recent logs |

### Examples

```bash
# Search with limit
logai search "database connection timeout" --limit 20

# Ask AI questions
logai ask "What caused the spike in errors at 3pm?"
logai ask "Summarize authentication patterns"
logai ask "Are there any security concerns?"

# Ingest different log formats
logai ingest logs.json --format json
logai ingest access.log --format apache --service apache-frontend
logai ingest nginx.log --format nginx --service nginx-gateway
logai ingest syslog.log --format syslog --service linux-server
```

## API Endpoints

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/health` | Health check (no auth required) |
| POST | `/api/logs` | Ingest JSON log entry |
| POST | `/api/logs/raw` | Ingest raw log lines |
| GET | `/api/search` | Semantic search logs |
| GET | `/api/ask` | Ask AI about logs |

### Authentication

If `LOGAI_API_KEY` is set, include header:
```
X-API-Key: your-api-key
```

### POST /api/logs - Ingest Log

```bash
curl -X POST http://localhost:3000/api/logs \
  -H "Content-Type: application/json" \
  -d '{
    "service": "my-app",
    "message": "User login successful",
    "level": "info",
    "metadata": {"user_id": "123"}
  }'
```

### POST /api/logs/raw - Ingest Raw Logs

```bash
curl -X POST http://localhost:3000/api/logs/raw \
  -H "Content-Type: application/json" \
  -d '{
    "format": "nginx",
    "service": "nginx-gateway",
    "lines": [
      "192.168.1.1 - - [24/May/2025:10:00:00 +0000] \"GET /api/users HTTP/1.1\" 200 1234"
    ]
  }'
```

### GET /api/search - Semantic Search

```bash
curl "http://localhost:3000/api/search?q=timeout%20error&limit=5"
```

### GET /api/ask - Ask AI

```bash
curl "http://localhost:3000/api/ask?q=What%20are%20the%20most%20common%20errors"
```

Response:
```json
{
  "question": "What are the most common errors",
  "answer": "Based on the logs, the most common errors are...",
  "sources": 5,
  "provider": "groq",
  "latency_ms": 856
}
```

## Log Formats Supported

### JSON
```json
{"service": "my-service", "message": "Operation completed", "level": "info"}
```

### Apache Combined
```
192.168.1.1 - frank [10/Oct/2000:13:55:36 -0700] "GET /page.html HTTP/1.0" 200 2326
```

### Nginx
```
192.168.1.1 - - [24/May/2025:10:00:00 +0000] "GET /api/users HTTP/1.1" 200 1234 "-" "curl"
```

### Syslog
```
May 24 10:30:00 server1 sshd[12345]: Accepted publickey for user from 10.0.0.5
```

## Environment Variables

| Variable | Description | Required |
|----------|-------------|----------|
| `GROQ_API_KEY` | Groq API key for LLM | Yes |
| `LOGAI_API_KEY` | API authentication key | No |

## Performance

- **Ingestion**: ~10,000 logs/second sustained
- **Search latency**: <50ms for vector search
- **AI response**: ~1 second (using Groq)
- **Storage**: Compressed columnar storage with ClickHouse

## License

MIT
