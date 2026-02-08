use axum::{
    Json, Router,
    extract::{Query, State},
    http::StatusCode,
    routing::{get, post},
};
use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
use logai_core::{LogEntry, RawLogEntry};
use logai_core::parser::{ApacheParser, NginxParser, SyslogParser, ParserRegistry};
use qdrant_client::Qdrant;
use qdrant_client::qdrant::SearchPointsBuilder;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use tracing::info;

const COLLECTION_NAME: &str = "log_embeddings";

// App state - Shared across handlers
struct AppState {
    nats: async_nats::Client,
    qdrant: Qdrant,
    model: Mutex<TextEmbedding>,
    parser_registry: ParserRegistry,
}

/// Ingest Endpoint
// Handler: Post /api/logs
async fn ingest_log(
    State(state): State<Arc<AppState>>,
    Json(raw): Json<RawLogEntry>,
) -> Result<Json<IngestResponse>, (StatusCode, String)> {
    // Rawlogentry -> Log entry (parse + enrich)
    let entry = LogEntry::from_raw(raw);

    let payload = serde_json::to_vec(&entry)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    state
        .nats
        .publish("logs.ingest", payload.into())
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    info!(
        id = %entry.id,
        level = ?entry.level,
        service = %entry.service,
        "Log published to NATS"
    );

    Ok(Json(IngestResponse {
        id: entry.id.to_string(),
        status: "accepted".to_string(),
    }))
}

// Response type
#[derive(serde::Serialize)]
struct IngestResponse {
    id: String,
    status: String,
}

// Raw log request (for parsing)
#[derive(Deserialize)]
struct RawLogRequest {
    format: String,      // "apache", "nginx", "syslog"
    service: String,     // Service name
    lines: Vec<String>,  // Raw log lines
}

#[derive(Serialize)]
struct RawIngestResponse {
    total: usize,
    parsed: usize,
    failed: usize,
}

/// Raw Log Ingest Endpoint - Parses logs before ingestion
async fn ingest_raw_log(
    State(state): State<Arc<AppState>>,
    Json(req): Json<RawLogRequest>,
) -> Result<Json<RawIngestResponse>, (StatusCode, String)> {
    let total = req.lines.len();
    let mut parsed = 0;
    let mut failed = 0;

    for line in req.lines {
        // Parse using registry
        match state.parser_registry.parse(&req.format, &line) {
            Ok(mut raw) => {
                // Override service from request
                raw.service = Some(req.service.clone());

                // Convert to LogEntry and publish
                let entry = LogEntry::from_raw(raw);
                let payload = serde_json::to_vec(&entry)
                    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

                state
                    .nats
                    .publish("logs.ingest", payload.into())
                    .await
                    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

                parsed += 1;
            }
            Err(_) => {
                failed += 1;
            }
        }
    }

    info!(total, parsed, failed, format = %req.format, "Raw logs ingested");

    Ok(Json(RawIngestResponse {
        total,
        parsed,
        failed,
    }))
}

/// Search ENdpoint

#[derive(Deserialize)]
struct SearchQuery {
    q: String,
    #[serde(default = "default_limit")]
    limit: u64,
}

fn default_limit() -> u64 {
    5
}

#[derive(Serialize)]
struct SearchResult {
    score: f32,
    log_id: String,
    service: String,
    level: String,
    message: String,
    timestamp: String,
}

async fn search_logs(
    State(state): State<Arc<AppState>>,
    Query(params): Query<SearchQuery>,
) -> Result<Json<Vec<SearchResult>>, (StatusCode, String)> {
    info!(query = %params.q, limit = params.limit, "Search request");

    // Embed the search query
    let query_vector = {
        let mut model = state.model.lock().unwrap();
        let embeddings = model
            .embed(vec![params.q.clone()], None)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        embeddings.into_iter().next().ok_or((
            StatusCode::INTERNAL_SERVER_ERROR,
            "No embedding".to_string(),
        ))?
    };

    // Search in Qdrant
    let results = state
        .qdrant
        .search_points(
            SearchPointsBuilder::new(COLLECTION_NAME, query_vector, params.limit)
                .with_payload(true),
        )
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Convert to reponse
    let search_results: Vec<SearchResult> = results
        .result
        .into_iter()
        .map(|point| {
            let payload = point.payload;
            SearchResult {
                score: point.score,
                log_id: get_string(&payload, "log_id"),
                service: get_string(&payload, "service"),
                level: get_string(&payload, "level"),
                message: get_string(&payload, "message"),
                timestamp: get_string(&payload, "timestamp"),
            }
        })
        .collect();
    info!(results = search_results.len(), "Search Complete");
    Ok(Json(search_results))
}

// Helper to extract string from payload
fn get_string(
    payload: &std::collections::HashMap<String, qdrant_client::qdrant::Value>,
    key: &str,
) -> String {
    payload
        .get(key)
        .and_then(|v| v.as_str().map(|s| s.to_string()))
        .unwrap_or_default()
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    //logging setup
    tracing_subscriber::fmt::init();

    // connect to NATS
    info!("Connecting to NATS...");
    let nats = async_nats::connect("localhost:4222").await?;
    info!("Connected to NATS!");

    // Connect to Qdrant
    info!("Connecting to Qdrant...");
    let qdrant = Qdrant::from_url("http://localhost:6334").build()?;
    info!("Connected to Qdrant!");

    // Load embedding model
    info!("Loading embedding model...");
    let model = TextEmbedding::try_new(InitOptions::new(EmbeddingModel::AllMiniLML6V2))?;
    info!("Model loaded!");

    // Setup parser registry
    info!("Setting up parser registry...");
    let mut parser_registry = ParserRegistry::new();
    parser_registry.register(Box::new(ApacheParser::new()));
    parser_registry.register(Box::new(NginxParser::new()));
    parser_registry.register(Box::new(SyslogParser::new()));
    info!("Parsers registered: apache, nginx, syslog");

    let state = Arc::new(AppState {
        nats,
        qdrant,
        model: Mutex::new(model),
        parser_registry,
    });

    //routes
    let app = Router::new()
        .route("/api/logs", post(ingest_log))
        .route("/api/logs/raw", post(ingest_raw_log))
        .route("/api/search", get(search_logs))
        .with_state(state);

    // Server start
    let addr = "0.0.0.0:3000";
    info!("Starting server on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await?;

    Ok(())
}
