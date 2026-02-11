use axum::{http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use logai_rag::{CausalChain, CausalLink, LogEvent};

/// JSON error response
#[derive(Serialize)]
pub struct ApiError {
    pub error: String,
    pub code: u16,
}

impl ApiError {
    pub fn new(status: StatusCode, message: impl Into<String>) -> (StatusCode, Json<Self>) {
        (status, Json(Self {
            error: message.into(),
            code: status.as_u16(),
        }))
    }
    
    pub fn not_found(message: impl Into<String>) -> (StatusCode, Json<Self>) {
        Self::new(StatusCode::NOT_FOUND, message)
    }
    
    pub fn internal(message: impl Into<String>) -> (StatusCode, Json<Self>) {
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, message)
    }
}

#[derive(Serialize)]
pub struct IngestResponse {
    pub id: String,
    pub status: String,
}

#[derive(Serialize)]
pub struct RawIngestResponse {
    pub total: usize,
    pub parsed: usize,
    pub failed: usize,
}

#[derive(Serialize)]
pub struct SearchResult {
    pub score: f32,
    pub log_id: String,
    pub service: String,
    pub level: String,
    pub message: String,
    pub timestamp: String,
}

#[derive(Serialize)]
pub struct AskResponse {
    pub answer: String,
    pub sources_count: usize,
    pub response_time_ms: u128,
    pub provider: String,
    pub query_analysis: QueryAnalysisResponse,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub causal_chain: Option<CausalChainResponse>,
}

/// Causal chain for "why" questions
#[derive(Serialize)]
pub struct CausalChainResponse {
    pub effect: LogEventResponse,
    pub chain: Vec<CausalLinkResponse>,
    pub root_cause: Option<LogEventResponse>,
    pub summary: String,
    pub recommendation: Option<String>,
}

#[derive(Serialize)]
pub struct CausalLinkResponse {
    pub effect: LogEventResponse,
    pub cause: LogEventResponse,
    pub confidence: f64,
    pub explanation: String,
}

#[derive(Serialize)]
pub struct LogEventResponse {
    pub timestamp: String,
    pub level: String,
    pub service: String,
    pub message: String,
}

impl From<CausalChain> for CausalChainResponse {
    fn from(c: CausalChain) -> Self {
        Self {
            effect: c.effect.into(),
            chain: c.chain.into_iter().map(|l| l.into()).collect(),
            root_cause: c.root_cause.map(|r| r.into()),
            summary: c.summary,
            recommendation: c.recommendation,
        }
    }
}

impl From<CausalLink> for CausalLinkResponse {
    fn from(l: CausalLink) -> Self {
        Self {
            effect: l.effect.into(),
            cause: l.cause.into(),
            confidence: l.confidence,
            explanation: l.explanation,
        }
    }
}

impl From<LogEvent> for LogEventResponse {
    fn from(e: LogEvent) -> Self {
        Self {
            timestamp: e.timestamp.to_rfc3339(),
            level: e.level,
            service: e.service,
            message: e.message,
        }
    }
}

#[derive(Serialize)]
pub struct QueryAnalysisResponse {
    pub search_query: String,
    pub time_filter: Option<String>,
    pub service_filter: Option<String>,
}

#[derive(Serialize)]
pub struct StatsResponse {
    pub total_logs: u64,
    pub logs_24h: u64,
    pub error_count: u64,
    pub services_count: u64,
    pub embeddings_count: u64,
    pub storage_mb: f64,
}

#[derive(Serialize, Deserialize, clickhouse::Row)]
pub struct RecentLogRow {
    pub log_id: String,
    pub service: String,
    pub level: String,
    pub message: String,
    pub timestamp: String,
}

#[derive(Serialize)]
pub struct AlertsResponse {
    pub alerts: Vec<AlertItem>,
}

#[derive(Serialize)]
pub struct AlertItem {
    pub id: String,
    pub service: String,
    pub severity: String,
    pub message: String,
    pub status: String,
    pub fired_at: String,
}

#[derive(Serialize)]
pub struct AnomaliesResponse {
    pub anomalies: Vec<AnomalyItem>,
    pub checked_at: String,
}

#[derive(Serialize)]
pub struct AnomalyItem {
    pub service: String,
    pub rule: String,
    pub severity: String,
    pub message: String,
    pub current_value: f64,
    pub expected_value: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Serialize)]
pub struct ChatApiResponse {
    pub answer: String,
    pub sources_count: usize,
    pub response_time_ms: u128,
    pub provider: String,
    pub context_logs: usize,
    pub conversation_turn: usize,
    pub source_logs: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub causal_chain: Option<CausalChainResponse>,
}

#[derive(Serialize)]
pub struct SessionInfo {
    pub session_id: String,
    pub turns: usize,
    pub last_logs_count: usize,
    pub age_seconds: u64,
}
