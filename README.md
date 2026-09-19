<h1 align="center">Stratum</h1>

<p align="center">
  <strong>🧠 Ask your logs questions. Get answers in plain English.</strong>
</p>

<p align="center">
  <a href="#-one-command-setup">Setup</a> •
  <a href="#-what-can-it-do">Features</a> •
  <a href="#-how-it-works">How It Works</a> •
  <a href="#-faq">FAQ</a>
</p>

<p align="center">
  <img src="https://img.shields.io/badge/Rust-000000?style=for-the-badge&logo=rust&logoColor=white" alt="Rust">
  <img src="https://img.shields.io/badge/Next.js-black?style=for-the-badge&logo=next.js&logoColor=white" alt="Next.js">
  <img src="https://img.shields.io/badge/Docker-2496ED?style=for-the-badge&logo=docker&logoColor=white" alt="Docker">
  <img src="https://img.shields.io/badge/ClickHouse-FFCC01?style=for-the-badge&logo=clickhouse&logoColor=black" alt="ClickHouse">
</p>

---

## 🤔 What is this?

You know how you have thousands of log files and when something breaks, you spend hours searching through them?

**Stratum fixes that.**

Instead of this:
```bash
grep -r "error" /var/log/ | grep "payment" | grep "timeout" | head -50
# 😫 Still no idea what's wrong...
```

You just ask:
```
"Why did payments fail last night?"
```

And get:
```
The payment failures occurred due to a database connection timeout.
At 2:47 AM, the connection pool was exhausted because of a memory leak
in the order-service. Here's the root cause chain:

1. 02:30 - Memory usage exceeded 90%
2. 02:45 - Connection pool warnings started
3. 02:47 - First payment timeout
4. 02:52 - Service restarted automatically

Recommendation: Increase connection pool size and fix the memory leak
in OrderProcessor.java line 234.
```

---

## 🚀 One-Command Setup

### Prerequisites
- [Docker](https://docs.docker.com/get-docker/) installed
- **Either:** Free [Groq API key](https://console.groq.com) (takes 30 seconds)
- **Or:** [Ollama](https://ollama.ai) installed locally (fully offline, no API key)

### Let's Go!

```bash
# 1. Clone the repo
git clone https://github.com/YEDASAVG/Stratum.git
cd Stratum

# 2. Run setup
./setup.sh  # For Groq (will ask for API key)

# Or for local-only with Ollama:
# Set LLM_PROVIDER=ollama in .env (see Configuration section)

# 3. Open your browser
# Dashboard: http://localhost:3001
```

**That's it. You're done.** 🎉

---

## 🎯 What Can It Do?

### 💬 Ask Questions in Plain English

| You Ask | Stratum Answers |
|---------|---------------|
| "Why is the API slow?" | Finds latency issues, shows timeline, suggests fixes |
| "Show errors from nginx" | Filters + ranks relevant logs automatically |
| "What happened at 3am?" | Summarizes all events in that time window |
| "Why did users get 502 errors?" | Traces the root cause across services |

### 🧠 Understands What You Meant

"Walk me through request 7f3a92" is a trace lookup. "Give me the gist of last
night" is a summary. Stratum works this out from meaning rather than matching
keywords, and says how confident it is - so an ambiguous question is answered
carefully instead of confidently wrong. See
[TypeSafe Jev](#option-3-smarter-query-understanding-typesafe-jev---optional).

### 🔍 Smart Search (Not Just Keywords)

Search for `"database connection issues"` and it finds:
- `Connection refused to postgres:5432`
- `MySQL timeout after 30s`
- `Redis reconnection failed`

Even though none of them contain "database connection issues"!

### 🚨 Automatic Anomaly Detection

Stratum watches your logs 24/7 and alerts you when:
- Error rate spikes (5x normal)
- New error patterns appear
- Service goes quiet (volume drop)

Get alerts in Slack before users complain.

### 📊 Beautiful Dashboard

- Real-time log explorer
- AI chat interface
- Anomaly timeline
- Service health overview

---

## 🏗️ How It Works

```
┌─────────────────────────────────────────────────────────────────────┐
│                         YOUR LOGS                                    │
│  (nginx, apache, apps, anything)                                    │
└─────────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────────┐
│                         LOG AI                                       │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐                 │
│  │   Parser    │  │  Embeddings │  │   Search    │                 │
│  │ nginx,json  │  │   (AI)      │  │   (Qdrant)  │                 │
│  └─────────────┘  └─────────────┘  └─────────────┘                 │
│         │                │                │                         │
│         ▼                ▼                ▼                         │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐                 │
│  │  ClickHouse │  │    Groq     │  │  Dashboard  │                 │
│  │  (Storage)  │  │   (LLM)     │  │  (Next.js)  │                 │
│  └─────────────┘  └─────────────┘  └─────────────┘                 │
│                                                                     │
│  ┌───────────────────────────────────────────────┐                 │
│  │  Jev (optional): typed query classification   │                 │
│  │  intent / service / severity + confidence     │                 │
│  └───────────────────────────────────────────────┘                 │
└─────────────────────────────────────────────────────────────────────┘
                                │
                                ▼
                    ┌─────────────────────┐
                    │  "The error was     │
                    │   caused by..."     │
                    └─────────────────────┘
```

**In Simple Terms:**
1. Your logs go in
2. AI understands what they mean
3. You ask questions
4. You get answers

---

## 📁 Project Structure

```
log-intelligence/
├── 🚀 setup.sh              # One-command setup
├── 🐳 docker-compose.yml    # All services defined here
├── 📄 Dockerfile            # Rust backend container
│
├── crates/                  # Rust code (the backend)
│   ├── logai-api/           # HTTP API server
│   ├── logai-core/          # Log parsers
│   ├── logai-rag/           # AI/search engine
│   ├── logai-worker/        # Background processor
│   ├── logai-anomaly/       # Anomaly detection
│   └── logai-cli/           # Terminal commands
│
└── dashboard/               # Next.js frontend
    ├── 🐳 Dockerfile
    └── src/
        └── app/             # React pages
```

---

## 🛠️ Commands

### Docker (Recommended)

```bash
# Start everything
docker compose up -d

# Stop everything
docker compose down

# View logs
docker compose logs -f

# Start with demo data (simulated logs)
docker compose --profile demo up -d
```

### Development Mode

If you want to modify the code:

```bash
# Start only infrastructure
docker compose -f docker-compose.dev.yml up -d

# Run Rust backend locally
./dev.sh

# Run frontend locally
cd dashboard && pnpm dev
```

### CLI Commands

```bash
# Check if everything is running
logai status

# Search logs
logai search "timeout error"

# Ask AI a question  
logai ask "What caused the crash at 3am?"

# Interactive chat mode (keeps context)
logai chat

# Import your log files
logai ingest /var/log/nginx/access.log --format nginx --service my-nginx

# View recent logs
logai logs --limit 50

# System statistics
logai stats
```

> **Tip:** The CLI binary is at `./target/release/logai` after building

---

## 🔌 Supported Log Formats

| Format | Example |
|--------|---------|
| **JSON** | `{"level":"error","message":"Connection failed"}` |
| **Nginx** | `192.168.1.1 - - [10/Feb/2026:14:00:00 +0000] "GET /api" 500` |
| **Apache** | `[Tue Feb 10 14:00:00 2026] [error] Connection refused` |
| **Syslog** | `Feb 10 14:00:00 server sshd[1234]: Failed password` |
| **Proxmox** | `Feb 23 14:00:00 pve1 pveproxy[1234]: starting worker` |

Don't see your format? The AI figures it out automatically for most logs!

---

## 🔗 Connect Your Logs

### Option 1: From Your App (HTTP API)

Send logs directly from your application code:

**Python**
```python
import requests
import datetime

def send_log(message, level="info", service="my-app"):
    requests.post("http://localhost:3000/api/logs", json={
        "message": message,
        "level": level,
        "service": service,
        "timestamp": datetime.datetime.utcnow().isoformat() + "Z"
    })

# Usage
send_log("User logged in successfully", "info")
send_log("Database connection failed", "error")
```

**Node.js**
```javascript
async function sendLog(message, level = "info", service = "my-app") {
  await fetch("http://localhost:3000/api/logs", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({
      message,
      level,
      service,
      timestamp: new Date().toISOString()
    })
  });
}

// Usage
sendLog("Order processed", "info");
sendLog("Payment timeout after 30s", "error");
```

**cURL**
```bash
curl -X POST http://localhost:3000/api/logs \
  -H "Content-Type: application/json" \
  -d '{
    "message": "User signup completed",
    "level": "info",
    "service": "auth-service",
    "fields": {"user_id": "12345", "plan": "pro"}
  }'
```

### Option 2: From Existing Log Files

Already have log files? Import them with the CLI:

```bash
# Nginx access logs
logai ingest /var/log/nginx/access.log --format nginx --service nginx

# Apache logs  
logai ingest /var/log/apache2/error.log --format apache --service apache

# Syslog
logai ingest /var/log/syslog --format syslog --service linux

# Proxmox VE logs
logai ingest /var/log/pveproxy/access.log --format proxmox --service proxmox

# JSON logs (common with Docker)
logai ingest /var/log/myapp/app.log --format json --service my-app
```

> **Note:** The CLI binary is called `logai`. After building, find it at `./target/release/logai`

### Option 3: From Docker Containers

**Using Docker logging driver:**
```yaml
# docker-compose.yml for YOUR app
services:
  my-app:
    image: your-app:latest
    logging:
      driver: "fluentd"
      options:
        fluentd-address: "localhost:24224"
        tag: "my-app"
```

**Or just pipe Docker logs:**
```bash
# One-liner to send all container logs
docker logs -f my-container 2>&1 | while read line; do
  curl -s -X POST http://localhost:3000/api/logs \
    -H "Content-Type: application/json" \
    -d "{\"message\": \"$line\", \"service\": \"my-container\"}"
done
```

### Option 4: Using Log Forwarders

**Fluent Bit** (lightweight, recommended)
```ini
# fluent-bit.conf
[OUTPUT]
    Name        http
    Match       *
    Host        localhost
    Port        3000
    URI         /api/logs
    Format      json
```

**Vector** (by Datadog)
```toml
# vector.toml
[sinks.stratum]
type = "http"
inputs = ["your_source"]
uri = "http://localhost:3000/api/logs"
encoding.codec = "json"
```

**Filebeat**
```yaml
# filebeat.yml
output.http:
  hosts: ["http://localhost:3000/api/logs"]
  codec.json:
    pretty: false
```

### Option 5: Try Demo Mode

Want to see it in action first? Start with simulated logs:

```bash
# Start with demo data (generates realistic logs automatically)
docker compose --profile demo up -d
```

This runs a simulator that generates logs from 5 fake services including payment failures, auth attacks, and database slowdowns - so you can test the AI without connecting your real apps.

---

## ⚙️ Configuration

Create a `.env` file (or run `./setup.sh` which does this for you).

### Remote Deployment (Server/VPS)

If you're deploying Stratum on a remote server (not localhost), set your server's IP:

```bash
# In your .env file
STRATUM_HOST=192.168.1.100  # or your-domain.com
```

This ensures the dashboard can connect to the API from your browser.

### Option 1: Cloud LLM (Groq - Free Tier)

```bash
# Get free key at https://console.groq.com
GROQ_API_KEY=gsk_your_key_here

# Optional. Defaults to openai/gpt-oss-20b
GROQ_MODEL=openai/gpt-oss-20b
```

Groq retires models regularly, and a retired name returns
`500 model_not_found` on every request. List what your account can actually
use before picking one:

```bash
curl https://api.groq.com/openai/v1/models \
  -H "Authorization: Bearer $GROQ_API_KEY"
```

Verified on a free account, with latency for a short log-analysis prompt:

| Model | Latency | Notes |
|---|---|---|
| `openai/gpt-oss-20b` | ~650ms | default |
| `qwen/qwen3.8-27b` | ~170ms | fastest, most concise answers |
| `openai/gpt-oss-120b` | ~920ms | more detail |
| `groq/compound-mini` | ~1800ms | verbose |

### Option 2: Local-Only (Ollama - No Internet)

```bash
# No API key needed - fully offline
LLM_PROVIDER=ollama
OLLAMA_URL=http://host.docker.internal:11434  # Use this for Docker
OLLAMA_MODEL=llama3.2  # or any model you have pulled
```

> **Note:** Use `host.docker.internal` when running in Docker. Use `localhost` only for local development outside Docker.

Make sure Ollama is running locally with a model:
```bash
ollama pull llama3.2
ollama serve
```

### Option 3: Smarter Query Understanding (TypeSafe Jev - Optional)

Stratum works out of the box using keyword rules to work out what a question
means. Those rules only recognise the words they were written for: intent
detection keys off `starts_with("why")`, and service detection matches a
fixed list of 17 names, so a query about your `payments-api` or
`ledger-worker` silently matches nothing and searches unfiltered.

Set a TypeSafe key and a System One model classifies instead. It returns a
typed answer plus a calibrated confidence, so low-confidence filters are
dropped rather than guessed:

```bash
# Optional. Get a key at https://console.typesafe.ai
TYPESAFE_API_KEY=ts_your_key_here

# Optional. Defaults to jev-latest
# JEV_MODEL=jev-1.13.0
```

Leave it blank and everything falls back to the keyword rules, so the
fully-offline Ollama setup is unaffected.

Compare the two paths on your own queries:

```bash
cargo run -p logai-rag --example jev_smoke
```

```
QUERY                                    RULES              JEV
why did payments-api throw 500s at 3am   Causal / api       Causal / payments-api
anything weird with the ledger worker    Search / -         Search / ledger-worker
walk me through request 7f3a92           Search / -         Trace  / -
give me the gist of last night           Search / -         Summary / -
```

Note row one: the regex matches the substring `api` inside `payments-api`
and filters to a service that does not exist, which returns nothing.

### Optional Settings

```bash
# Protect your API
LOGAI_API_KEY=your-secret-key

# Logs sent to the LLM per answer (default 25)
LOGAI_MAX_CONTEXT_LOGS=25

# Slack alerts
SLACK_WEBHOOK_URL=https://hooks.slack.com/...
```

---

## ❓ FAQ

### "Do I need to pay for anything?"

**Nope!** 
- **Option 1:** Groq API has a generous free tier (enough for personal use)
- **Option 2:** Use Ollama for 100% free, fully local AI (no API key needed)
- All infrastructure runs locally in Docker

### "How many logs can it handle?"

- **Ingestion**: 50,000+ logs/second
- **Storage**: Millions of logs (ClickHouse is crazy efficient)
- **Search**: <100ms response time

### "Can I use my own LLM?"

Yes! Set `LLM_PROVIDER=ollama` in your `.env` and point `OLLAMA_URL` to your local Ollama instance. No Groq API key required.

### "Is my data sent anywhere?"

Only the AI query + relevant log snippets go to Groq for analysis. 
Your raw logs stay 100% local in Docker volumes.

### "It's not working!"

```bash
# Check if all services are running
docker compose ps

# Check logs for errors
docker compose logs api

# Common fixes:
docker compose down
docker compose up -d --build
```

---

## 🆚 Why Stratum vs Others?

| Feature | Stratum | Datadog | Splunk | ELK |
|---------|-------|---------|--------|-----|
| **Price** | Free | $$$$ | $$$$ | Free |
| **Setup Time** | 1 min | Hours | Hours | Hours |
| **AI Chat** | ✅ | ✅ | ❌ | ❌ |
| **Self-Hosted** | ✅ | ❌ | ❌ | ✅ |
| **Semantic Search** | ✅ | ❌ | ❌ | ❌ |
| **Root Cause Analysis** | ✅ Auto | Manual | Manual | Manual |

---

## 🧪 Running Benchmarks

```bash
# Run parsing benchmarks
cargo bench -p logai-core --bench parsing

# Run RAG benchmarks
cargo bench -p logai-rag --bench rag

# Stress test (API must be running)
cargo run --release --bin logai-stress -- --rate 10000 --total 100000
```

---

## 🤝 Contributing

1. Fork the repo
2. Create a branch (`git checkout -b feature/awesome`)
3. Make your changes
4. Run tests (`cargo test`)
5. Push and create a PR

---

## 📜 License

Licensed under the [Apache License 2.0](LICENSE) - use it freely, with patent protection included!

---

## ⭐ Star This Repo!

If Stratum saved you time debugging, give it a star! It helps others find it.

<p align="center">
  <a href="https://github.com/YEDASAVG/Stratum">
    <img src="https://img.shields.io/github/stars/YEDASAVG/Stratum?style=social" alt="GitHub stars">
  </a>
</p>

---

<p align="center">
  Built with ❤️ and mass amounts of ☕
</p>
