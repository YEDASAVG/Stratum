use axum::{
    extract::State,
    http::StatusCode,
    Json,
};
use logai_core::{LogEntry, RawLogEntry};
use std::sync::Arc;
use tracing::info;

use crate::models::{IngestResponse, RawIngestResponse, RawLogRequest};
use crate::state::AppState;

pub async fn ingest_log(
    State(state): State<Arc<AppState>>,
    Json(raw): Json<RawLogEntry>,
) -> Result<Json<IngestResponse>, (StatusCode, String)> {
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

pub async fn ingest_raw_log(
    State(state): State<Arc<AppState>>,
    Json(req): Json<RawLogRequest>,
) -> Result<Json<RawIngestResponse>, (StatusCode, String)> {
    let total = req.lines.len();
    let mut parsed = 0;
    let mut failed = 0;

    for line in req.lines {
        match state.parser_registry.parse(&req.format, &line) {
            Ok(mut raw) => {
                raw.service = Some(req.service.clone());
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
