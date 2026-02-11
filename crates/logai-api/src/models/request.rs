use serde::Deserialize;
use super::response::ChatMessage;

#[derive(Deserialize)]
pub struct RawLogRequest {
    pub format: String,
    pub service: String,
    pub lines: Vec<String>,
}

#[derive(Deserialize)]
pub struct SearchQuery {
    pub q: String,
    #[serde(default = "default_limit")]
    pub limit: u64,
    pub from: Option<i64>,
    pub to: Option<i64>,
    pub service: Option<String>,
}

fn default_limit() -> u64 {
    5
}

#[derive(Deserialize)]
pub struct AskQuery {
    pub q: String,
}

#[derive(Deserialize)]
pub struct RecentLogsQuery {
    pub limit: Option<u32>,
    pub service: Option<String>,
    pub level: Option<String>,
}

#[derive(Deserialize)]
pub struct AlertsQuery {
    pub status: Option<String>,
}

#[derive(Deserialize)]
pub struct AnomaliesQuery {
    pub service: Option<String>,
}

#[derive(Deserialize)]
pub struct ChatRequest {
    pub session_id: String,
    pub message: String,
    #[serde(default)]
    pub history: Vec<ChatMessage>,
}

#[derive(Deserialize)]
pub struct SessionQuery {
    pub session_id: String,
}
