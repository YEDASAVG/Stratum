use axum::{
    Json, Router,
    body::Body,
    extract::{Query, State},
    http::{Request, StatusCode},
    middleware::{self, Next},
    response::Response,
    routing::{get, post},
};
use clickhouse::Client as ClickHouseClient;
use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
use logai_core::parser::{ApacheParser, NginxParser, ParserRegistry, SyslogParser};
use logai_core::{LogEntry, RawLogEntry};
use logai_rag::{RagEngine, RagConfig, Reranker};
use qdrant_client::Qdrant;
use qdrant_client::qdrant::{Condition, Filter, Range, SearchPointsBuilder};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};
use std::time::Instant;
use tracing::info;

const COLLECTION_NAME: &str = "log_embeddings";

/// Conversation session for chat
#[derive(Clone, Debug)]
struct ChatSession {
    history: Vec<ChatMessage>,
    last_logs: Vec<String>,
    created_at: std::time::Instant,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct ChatMessage {
    role: String, // "user" or "assistant"
    content: String,
}

// App state - Shared across handlers
struct AppState {
    nats: async_nats::Client,
    qdrant: Qdrant,
    clickhouse: ClickHouseClient,
    model: Mutex<TextEmbedding>,
    parser_registry: ParserRegistry,
    rag_engine: RagEngine,
    reranker: Reranker,
    sessions: RwLock<HashMap<String, ChatSession>>,
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
    format: String,     // "apache", "nginx", "syslog"
    service: String,    // Service name
    lines: Vec<String>, // Raw log lines
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
    from: Option<i64>,
    to: Option<i64>,
    service: Option<String>,
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

    // Build filter conditions
    let mut conditions = vec![];

    if let Some(from) = params.from {
        conditions.push(Condition::range(
            "timestamp_unix",
            Range {
                gte: Some(from as f64),
                ..Default::default()
            },
        ));
    }
    if let Some(to) = params.to {
        conditions.push(Condition::range(
            "timestamp_unix",
            Range {
                lte: Some(to as f64),
                ..Default::default()
            },
        ));
    }
    if let Some(ref service) = params.service {
        conditions.push(Condition::matches("service", service.clone()));
    }

    let filter = if conditions.is_empty() {
        None
    } else {
        Some(Filter::must(conditions))
    };

    // Search in Qdrant

    let mut search_builder =
        SearchPointsBuilder::new(COLLECTION_NAME, query_vector, params.limit).with_payload(true);

    if let Some(f) = filter {
        search_builder = search_builder.filter(f);
    }

    let results = state
        .qdrant
        .search_points(search_builder)
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

/// ASK Endpoint - Natural language RAG query
#[derive(Deserialize)]
struct AskQuery {
    q: String,
}

#[derive(Serialize)]
struct AskResponse {
    answer: String,
    sources_count: usize,
    response_time_ms: u128,
    provider: String,
    query_analysis: QueryAnalysisResponse,
}

#[derive(Serialize)]
struct QueryAnalysisResponse {
    search_query: String,
    time_filter: Option<String>,
    service_filter: Option<String>,
}

async fn ask_logs(
    State(state): State<Arc<AppState>>,
    Query(params): Query<AskQuery>,
) -> Result<Json<AskResponse>, (StatusCode, String)> {
    let start = Instant::now();
    info!(query = %params.q, "ASK request");

    // Step 1: Analyze query to extract filters
    let analyzed = state.rag_engine.analyze_query(&params.q);

    // Step 2: Embed search query
    let query_vector = {
        let mut model = state.model.lock().unwrap();
        let embeddings = model
            .embed(vec![analyzed.search_query.clone()], None)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        embeddings.into_iter().next().ok_or((
            StatusCode::INTERNAL_SERVER_ERROR,
            "No embedding".to_string(),
        ))?
    };

    // Step 3: Build filters from analyzed query
    let mut conditions = vec![];
    if let Some(from) = analyzed.from {
        conditions.push(Condition::range(
            "timestamp_unix",
            Range {
                gte: Some(from.timestamp() as f64),
                ..Default::default()
            },
        ));
    }
    if let Some(ref service) = analyzed.service {
        // Use text match for substring matching (nginx matches stress-test-nginx)
        conditions.push(Condition::matches_text("service", service.clone()));
    }

    let filter = if conditions.is_empty() {
        None
    } else {
        Some(Filter::must(conditions))
    };

    // Step 4: Search Qdrant (get more for reranking)
    let mut search_builder =
        SearchPointsBuilder::new(COLLECTION_NAME, query_vector, 30).with_payload(true);
    if let Some(f) = filter {
        search_builder = search_builder.filter(f);
    }

    let results = state
        .qdrant
        .search_points(search_builder)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Step 5: Rerank logs
    let logs_with_scores: Vec<(String, f32)> = results
        .result
        .iter()
        .map(|point| (get_string(&point.payload, "message"), point.score))
        .collect();

    info!(logs_found = logs_with_scores.len(), "Logs retrieved from Qdrant");

    if logs_with_scores.is_empty() {
        return Err((StatusCode::NOT_FOUND, "No relevant logs found".to_string()));
    }

    // Rerank and take top 10
    let reranked = state.reranker.rerank(&params.q, logs_with_scores, 10);
    let logs: Vec<String> = reranked.into_iter().map(|r| r.message).collect();

    info!(reranked_count = logs.len(), "Logs reranked");
    info!("Calling Ollama LLM...");

    // Step 6: Call RAG engine
    let rag_response = state
        .rag_engine
        .query(&params.q, logs)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let elapsed = start.elapsed().as_millis();
    info!(sources = rag_response.sources_count, provider = %rag_response.provider, time_ms = elapsed, "ASK complete");

    Ok(Json(AskResponse {
        answer: rag_response.answer,
        sources_count: rag_response.sources_count,
        response_time_ms: elapsed,
        provider: rag_response.provider,
        query_analysis: QueryAnalysisResponse {
            search_query: rag_response.query_analysis.search_query,
            time_filter: rag_response.query_analysis.time_filter,
            service_filter: rag_response.query_analysis.service_filter,
        },
    }))
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

/// Stats Endpoint - System statistics
#[derive(Serialize)]
struct StatsResponse {
    total_logs: u64,
    logs_24h: u64,
    error_count: u64,
    services_count: u64,
    embeddings_count: u64,
    storage_mb: f64,
}

async fn get_stats(
    State(state): State<Arc<AppState>>,
) -> Result<Json<StatsResponse>, (StatusCode, String)> {
    info!("Stats request");

    // Query ClickHouse for stats
    let total_logs: u64 = state.clickhouse
        .query("SELECT count(*) FROM logs")
        .fetch_one()
        .await
        .unwrap_or(0);

    let logs_24h: u64 = state.clickhouse
        .query("SELECT count(*) FROM logs WHERE timestamp > now() - INTERVAL 1 DAY")
        .fetch_one()
        .await
        .unwrap_or(0);

    let error_count: u64 = state.clickhouse
        .query("SELECT count(*) FROM logs WHERE level = 'Error'")
        .fetch_one()
        .await
        .unwrap_or(0);

    let services_count: u64 = state.clickhouse
        .query("SELECT count(DISTINCT service) FROM logs")
        .fetch_one()
        .await
        .unwrap_or(0);

    // Get embeddings count from Qdrant
    let embeddings_count = match state.qdrant.collection_info(COLLECTION_NAME).await {
        Ok(info) => info.result.map(|r| r.points_count.unwrap_or(0)).unwrap_or(0),
        Err(_) => 0,
    };

    // Estimate storage (rough approximation)
    let storage_mb = (total_logs as f64 * 0.5) / 1024.0; // ~0.5KB per log average

    Ok(Json(StatsResponse {
        total_logs,
        logs_24h,
        error_count,
        services_count,
        embeddings_count,
        storage_mb,
    }))
}

/// Alerts Endpoint - List active alerts
#[derive(Deserialize)]
struct AlertsQuery {
    status: Option<String>,
}

#[derive(Serialize)]
struct AlertsResponse {
    alerts: Vec<AlertItem>,
}

#[derive(Serialize)]
struct AlertItem {
    id: String,
    service: String,
    severity: String,
    message: String,
    status: String,
    fired_at: String,
}

async fn get_alerts(
    State(state): State<Arc<AppState>>,
    Query(params): Query<AlertsQuery>,
) -> Result<Json<AlertsResponse>, (StatusCode, String)> {
    info!(status = ?params.status, "Alerts request");

    // Query ClickHouse for recent anomalies (simulating alerts)
    // In production, you'd have a separate alerts table
    let query = match &params.status {
        Some(status) if status == "firing" => {
            "SELECT service, level, message, timestamp 
             FROM logs 
             WHERE level = 'Error' 
             AND timestamp > now() - INTERVAL 1 HOUR
             ORDER BY timestamp DESC
             LIMIT 20"
        }
        _ => {
            "SELECT service, level, message, timestamp 
             FROM logs 
             WHERE level = 'Error' 
             AND timestamp > now() - INTERVAL 24 HOUR
             ORDER BY timestamp DESC
             LIMIT 50"
        }
    };

    let rows: Vec<(String, String, String, i64)> = state.clickhouse
        .query(query)
        .fetch_all()
        .await
        .unwrap_or_default();

    let alerts: Vec<AlertItem> = rows
        .into_iter()
        .enumerate()
        .map(|(i, (service, level, message, ts))| {
            let severity = if message.to_lowercase().contains("critical") || message.to_lowercase().contains("fatal") {
                "critical"
            } else if level == "Error" {
                "warning"
            } else {
                "info"
            };
            
            AlertItem {
                id: format!("alert-{}", i),
                service,
                severity: severity.to_string(),
                message: if message.len() > 100 { format!("{}...", &message[..97]) } else { message },
                status: "firing".to_string(),
                fired_at: chrono::DateTime::from_timestamp(ts / 1000, 0)
                    .map(|dt| dt.format("%H:%M:%S").to_string())
                    .unwrap_or_default(),
            }
        })
        .collect();

    info!(count = alerts.len(), "Alerts returned");

    Ok(Json(AlertsResponse { alerts }))
}

/// Anomalies Endpoint - Check for anomalies
#[derive(Deserialize)]
struct AnomaliesQuery {
    service: Option<String>,
}

#[derive(Serialize)]
struct AnomaliesResponse {
    anomalies: Vec<AnomalyItem>,
    checked_at: String,
}

#[derive(Serialize)]
struct AnomalyItem {
    service: String,
    rule: String,
    severity: String,
    message: String,
    current_value: f64,
    expected_value: f64,
}

async fn get_anomalies(
    State(state): State<Arc<AppState>>,
    Query(params): Query<AnomaliesQuery>,
) -> Result<Json<AnomaliesResponse>, (StatusCode, String)> {
    info!(service = ?params.service, "Anomalies request");

    let mut anomalies = Vec::new();
    let now = chrono::Utc::now();

    // Get list of services
    let services_query = match &params.service {
        Some(s) => format!("SELECT DISTINCT service FROM logs WHERE service = '{}' LIMIT 20", s),
        None => "SELECT DISTINCT service FROM logs LIMIT 20".to_string(),
    };

    let services: Vec<String> = state.clickhouse
        .query(&services_query)
        .fetch_all()
        .await
        .unwrap_or_default();

    // Check each service for anomalies
    for service in services {
        // Get error count in last 5 minutes
        let current_errors: u64 = state.clickhouse
            .query(&format!(
                "SELECT count(*) FROM logs WHERE service = '{}' AND level = 'Error' AND timestamp > now() - INTERVAL 5 MINUTE",
                service
            ))
            .fetch_one()
            .await
            .unwrap_or(0);

        // Get average error count per 5-minute window in last hour (baseline)
        let baseline_errors: f64 = state.clickhouse
            .query(&format!(
                "SELECT avg(error_count) FROM (
                    SELECT count(*) as error_count 
                    FROM logs 
                    WHERE service = '{}' AND level = 'Error' 
                    AND timestamp > now() - INTERVAL 1 HOUR
                    GROUP BY toStartOfFiveMinutes(timestamp)
                )",
                service
            ))
            .fetch_one()
            .await
            .unwrap_or(0.0);

        // Detect spike: current > 2x baseline (and baseline > 0)
        if baseline_errors > 0.0 && (current_errors as f64) > baseline_errors * 2.0 {
            let severity = if (current_errors as f64) > baseline_errors * 5.0 {
                "critical"
            } else {
                "warning"
            };

            anomalies.push(AnomalyItem {
                service: service.clone(),
                rule: "Error Spike".to_string(),
                severity: severity.to_string(),
                message: format!(
                    "Error count spike: {} errors in last 5 min (baseline: {:.1})",
                    current_errors, baseline_errors
                ),
                current_value: current_errors as f64,
                expected_value: baseline_errors,
            });
        }

        // Get log volume in last 5 minutes
        let current_volume: u64 = state.clickhouse
            .query(&format!(
                "SELECT count(*) FROM logs WHERE service = '{}' AND timestamp > now() - INTERVAL 5 MINUTE",
                service
            ))
            .fetch_one()
            .await
            .unwrap_or(0);

        // Get average volume
        let baseline_volume: f64 = state.clickhouse
            .query(&format!(
                "SELECT avg(log_count) FROM (
                    SELECT count(*) as log_count 
                    FROM logs 
                    WHERE service = '{}' 
                    AND timestamp > now() - INTERVAL 1 HOUR
                    GROUP BY toStartOfFiveMinutes(timestamp)
                )",
                service
            ))
            .fetch_one()
            .await
            .unwrap_or(0.0);

        // Detect volume drop: current < 10% of baseline
        if baseline_volume > 10.0 && (current_volume as f64) < baseline_volume * 0.1 {
            anomalies.push(AnomalyItem {
                service: service.clone(),
                rule: "Volume Drop".to_string(),
                severity: "warning".to_string(),
                message: format!(
                    "Log volume dropped: {} logs in last 5 min (baseline: {:.1})",
                    current_volume, baseline_volume
                ),
                current_value: current_volume as f64,
                expected_value: baseline_volume,
            });
        }
    }

    info!(count = anomalies.len(), "Anomalies detected");

    Ok(Json(AnomaliesResponse {
        anomalies,
        checked_at: now.format("%Y-%m-%dT%H:%M:%SZ").to_string(),
    }))
}

/// Chat Endpoint - Interactive debugging with conversation memory
#[derive(Deserialize)]
struct ChatRequest {
    session_id: String,
    message: String,
    #[serde(default)]
    history: Vec<ChatMessage>,
}

#[derive(Serialize)]
struct ChatApiResponse {
    answer: String,
    sources_count: usize,
    response_time_ms: u128,
    provider: String,
    context_logs: usize,
    conversation_turn: usize,
}

async fn chat_logs(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ChatRequest>,
) -> Result<Json<ChatApiResponse>, (StatusCode, String)> {
    let start = Instant::now();
    info!(session = %req.session_id, message = %req.message, "CHAT request");

    // Get or create session
    let (history, turn) = {
        let mut sessions = state.sessions.write().unwrap();
        let session = sessions.entry(req.session_id.clone()).or_insert_with(|| {
            ChatSession {
                history: Vec::new(),
                last_logs: Vec::new(),
                created_at: std::time::Instant::now(),
            }
        });
        
        // Merge incoming history with stored history
        if !req.history.is_empty() && session.history.is_empty() {
            session.history = req.history.clone();
        }
        
        (session.history.clone(), session.history.len() / 2 + 1)
    };

    // Step 1: Analyze query to extract filters
    let analyzed = state.rag_engine.analyze_query(&req.message);

    // Step 2: Embed search query
    let query_vector = {
        let mut model = state.model.lock().unwrap();
        let embeddings = model
            .embed(vec![analyzed.search_query.clone()], None)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        embeddings.into_iter().next().ok_or((
            StatusCode::INTERNAL_SERVER_ERROR,
            "No embedding".to_string(),
        ))?
    };

    // Step 3: Build filters from analyzed query
    let mut conditions = vec![];
    if let Some(from) = analyzed.from {
        conditions.push(Condition::range(
            "timestamp_unix",
            Range {
                gte: Some(from.timestamp() as f64),
                ..Default::default()
            },
        ));
    }
    if let Some(ref service) = analyzed.service {
        conditions.push(Condition::matches_text("service", service.clone()));
    }

    let filter = if conditions.is_empty() {
        None
    } else {
        Some(Filter::must(conditions))
    };

    // Step 4: Search Qdrant (get more for reranking)
    let mut search_builder =
        SearchPointsBuilder::new(COLLECTION_NAME, query_vector, 30).with_payload(true);
    if let Some(f) = filter {
        search_builder = search_builder.filter(f);
    }

    let results = state
        .qdrant
        .search_points(search_builder)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Step 5: Rerank logs
    let logs_with_scores: Vec<(String, f32)> = results
        .result
        .iter()
        .map(|point| (get_string(&point.payload, "message"), point.score))
        .collect();

    info!(logs_found = logs_with_scores.len(), "Logs retrieved for chat");

    if logs_with_scores.is_empty() {
        return Err((StatusCode::NOT_FOUND, "No relevant logs found".to_string()));
    }

    // Rerank and take top 10
    let reranked = state.reranker.rerank(&req.message, logs_with_scores, 10);
    let logs: Vec<String> = reranked.into_iter().map(|r| r.message).collect();
    let context_logs = logs.len();

    info!(reranked_count = logs.len(), "Logs reranked for chat");

    // Step 6: Build conversation-aware prompt
    let conversation_context = build_conversation_context(&history);
    
    // Query with conversation context
    let full_query = if conversation_context.is_empty() {
        req.message.clone()
    } else {
        format!(
            "Previous conversation:\n{}\n\nCurrent question: {}",
            conversation_context, req.message
        )
    };

    // Call RAG engine with conversation context
    let rag_response = state
        .rag_engine
        .query(&full_query, logs.clone())
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Update session with new exchange
    {
        let mut sessions = state.sessions.write().unwrap();
        if let Some(session) = sessions.get_mut(&req.session_id) {
            session.history.push(ChatMessage {
                role: "user".to_string(),
                content: req.message.clone(),
            });
            session.history.push(ChatMessage {
                role: "assistant".to_string(),
                content: rag_response.answer.clone(),
            });
            session.last_logs = logs;
            
            // Keep history manageable (last 20 messages = 10 turns)
            if session.history.len() > 20 {
                session.history.drain(0..2);
            }
        }
    }

    let elapsed = start.elapsed().as_millis();
    info!(
        turn = turn,
        sources = rag_response.sources_count,
        provider = %rag_response.provider,
        time_ms = elapsed,
        "CHAT complete"
    );

    Ok(Json(ChatApiResponse {
        answer: rag_response.answer,
        sources_count: rag_response.sources_count,
        response_time_ms: elapsed,
        provider: rag_response.provider,
        context_logs,
        conversation_turn: turn,
    }))
}

/// Build conversation context from history
fn build_conversation_context(history: &[ChatMessage]) -> String {
    if history.is_empty() {
        return String::new();
    }
    
    // Take last 6 messages (3 turns) for context
    let recent: Vec<&ChatMessage> = history.iter().rev().take(6).collect::<Vec<_>>().into_iter().rev().collect();
    
    recent
        .iter()
        .map(|msg| {
            let role = if msg.role == "user" { "User" } else { "AI" };
            format!("{}: {}", role, msg.content)
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Get session info (for debugging)
#[derive(Deserialize)]
struct SessionQuery {
    session_id: String,
}

#[derive(Serialize)]
struct SessionInfo {
    session_id: String,
    turns: usize,
    last_logs_count: usize,
    age_seconds: u64,
}

async fn get_session(
    State(state): State<Arc<AppState>>,
    Query(params): Query<SessionQuery>,
) -> Result<Json<SessionInfo>, (StatusCode, String)> {
    let sessions = state.sessions.read().unwrap();
    
    match sessions.get(&params.session_id) {
        Some(session) => Ok(Json(SessionInfo {
            session_id: params.session_id,
            turns: session.history.len() / 2,
            last_logs_count: session.last_logs.len(),
            age_seconds: session.created_at.elapsed().as_secs(),
        })),
        None => Err((StatusCode::NOT_FOUND, "Session not found".to_string())),
    }
}

// API Key Authentication Middleware
async fn require_api_key(
    request: Request<Body>,
    next: Next,
) -> Result<Response, (StatusCode, &'static str)> {
    // Get API key from environment (if not set, authentication is disabled)
    let expected_key = std::env::var("LOGAI_API_KEY").ok();
    
    // If no API key configured, skip authentication
    let Some(expected) = expected_key else {
        return Ok(next.run(request).await);
    };
    
    // If API key is empty, skip authentication
    if expected.is_empty() {
        return Ok(next.run(request).await);
    }
    
    // Check for API key in header
    let provided = request
        .headers()
        .get("X-API-Key")
        .and_then(|v| v.to_str().ok());
    
    match provided {
        Some(key) if key == expected => Ok(next.run(request).await),
        Some(_) => Err((StatusCode::UNAUTHORIZED, "Invalid API key")),
        None => Err((StatusCode::UNAUTHORIZED, "Missing X-API-Key header")),
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load .env file
    dotenvy::dotenv().ok();

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

    // Connect to ClickHouse
    info!("Connecting to ClickHouse...");
    let clickhouse = ClickHouseClient::default()
        .with_url("http://localhost:8123")
        .with_database("logai");
    info!("Connected to ClickHouse!");

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

    // Setup RAG engine (using Groq for fast inference)
    info!("Setting up RAG engine with Groq...");
    let rag_engine = RagEngine::new(RagConfig::with_groq());
    let reranker = Reranker::new();
    info!("RAG engine ready!");

    let state = Arc::new(AppState {
        nats,
        qdrant,
        clickhouse,
        model: Mutex::new(model),
        parser_registry,
        rag_engine,
        reranker,
        sessions: RwLock::new(HashMap::new()),
    });

    //routes - protected routes with API key
    let protected_routes = Router::new()
        .route("/api/logs", post(ingest_log))
        .route("/api/logs/raw", post(ingest_raw_log))
        .route("/api/search", get(search_logs))
        .route("/api/ask", get(ask_logs))
        .route("/api/chat", post(chat_logs))
        .route("/api/session", get(get_session))
        .route("/api/stats", get(get_stats))
        .route("/api/alerts", get(get_alerts))
        .route("/api/anomalies", get(get_anomalies))
        .layer(middleware::from_fn(require_api_key));
    
    // Health endpoint without auth
    let app = Router::new()
        .route("/health", get(|| async { "ok" }))
        .merge(protected_routes)
        .with_state(state);
    
    // Log if API key is enabled
    if std::env::var("LOGAI_API_KEY").ok().filter(|k| !k.is_empty()).is_some() {
        info!("API key authentication ENABLED");
    } else {
        info!("API key authentication DISABLED (set LOGAI_API_KEY to enable)");
    }

    // Server start
    let addr = "0.0.0.0:3000";
    info!("Starting server on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await?;

    Ok(())
}
