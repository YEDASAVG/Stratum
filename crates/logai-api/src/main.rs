use axum::{routing::post, Router, Json, http::StatusCode, extract::State};
use logai_core::{RawLogEntry, LogEntry};
use tracing::{info};
use std::sync::Arc;

// APp state - Shared across handlers
#[derive(Clone)]
struct AppState{
    nats: async_nats::Client,
}

// Handler: Post /api/logs
async fn ingest_log(State(state): State<Arc<AppState>>, Json(raw): Json<RawLogEntry>,) -> Result<Json<IngestResponse>, (StatusCode, String)> {
    // Rawlogentry -> Log entry (parse + enrich)
    let entry = LogEntry::from_raw(raw);

    let payload = serde_json::to_vec(&entry)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    state.nats
    .publish("logs.ingest", payload.into())
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    info!(
        id = %entry.id,
        level = ?entry.level,
        service = %entry.service,
        "Log published to NATS"
    );

    Ok(Json(IngestResponse { id: entry.id.to_string(), status: "accepted".to_string(), }))
}


// Response type
#[derive(serde::Serialize)]
struct IngestResponse {
    id: String,
    status: String,
}


#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    //logiing setup
    tracing_subscriber::fmt::init();

    // connect to NATS
    info!("Connecting to NATS...");
    let nats = async_nats::connect("localhost:4222").await?;
    info!("Connected to NATS!");

    let state = Arc::new(AppState {nats});

    //routes
    let app = Router::new()
    .route("/api/logs", post(ingest_log))
    .with_state(state);

    // Server start
    let addr = "0.0.0.0:3000";
    info!("Starting server on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await?;

    Ok(())
}