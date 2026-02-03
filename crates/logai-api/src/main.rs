use axum::{routing::post, Router, Json, http::StatusCode,};
use logai_core::{RawLogEntry, LogEntry};
use tracing::{info};

// Handler: Post /api/logs

async fn ingest_log(Json(raw): Json<RawLogEntry>,) -> Result<Json<LogEntry>, (StatusCode, String)> {
    // Rawlogentry -> Log entry (parse + enrich)
    let entry = LogEntry::from_raw(raw);

    info!(
        level = ?entry.level,
        service = %entry.service,
        "Log recieved"
    );
    // for now we will Return
    //Later we will publish it on NATS
    Ok(Json(entry))
}

#[tokio::main]
async fn main() {
    //logiing setup
    tracing_subscriber::fmt::init();

    //routes
    let app = Router::new()
    .route("/api/logs", post(ingest_log));

    // Server start
    let addr = "0.0.0.0:3000";
    info!("Starting server on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}