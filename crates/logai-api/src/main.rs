mod handlers;
mod middleware;
mod models;
mod state;

use axum::{middleware as axum_mw, routing::{get, post}, Router};
use clickhouse::Client as ClickHouseClient;
use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
use logai_core::parser::{ApacheParser, NginxParser, ParserRegistry, SyslogParser};
use logai_rag::{RagConfig, RagEngine, Reranker};
use qdrant_client::Qdrant;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};
use tower_http::cors::{Any, CorsLayer};
use tracing::info;

use handlers::*;
use middleware::require_api_key;
use state::AppState;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load .env file
    dotenvy::dotenv().ok();

    //logging setup
    tracing_subscriber::fmt::init();

    // Read infrastructure URLs from environment
    let nats_url = std::env::var("NATS_URL").unwrap_or_else(|_| "localhost:4222".to_string());
    let qdrant_url = std::env::var("QDRANT_URL").unwrap_or_else(|_| "http://localhost:6334".to_string());
    let clickhouse_url = std::env::var("CLICKHOUSE_URL").unwrap_or_else(|_| "http://localhost:8123".to_string());

    // connect to NATS
    info!("Connecting to NATS at {}...", nats_url);
    let nats = async_nats::connect(&nats_url).await?;
    info!("Connected to NATS!");

    // Connect to Qdrant
    info!("Connecting to Qdrant at {}...", qdrant_url);
    let qdrant = Qdrant::from_url(&qdrant_url).build()?;
    info!("Connected to Qdrant!");

    // Connect to ClickHouse
    info!("Connecting to ClickHouse at {}...", clickhouse_url);
    let clickhouse = ClickHouseClient::default()
        .with_url(&clickhouse_url)
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

    // Setup RAG engine (configurable via LOGAI_GROQ_MODEL env var)
    let rag_config = RagConfig::from_env();
    info!(
        model = %rag_config.groq_model,
        "Setting up RAG engine with Groq..."
    );
    let rag_engine = RagEngine::new(rag_config);
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
        .route("/api/logs/recent", get(get_recent_logs))
        .route("/api/search", get(search_logs))
        .route("/api/ask", get(ask_logs))
        .route("/api/chat", post(chat_logs))
        .route("/api/session", get(get_session))
        .route("/api/stats", get(get_stats))
        .route("/api/alerts", get(get_alerts))
        .route("/api/anomalies", get(get_anomalies))
        .route("/api/services", get(get_services))
        .layer(axum_mw::from_fn(require_api_key));
    
    // Health endpoint without auth
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);
    
    let app = Router::new()
        .route("/health", get(|| async { "ok" }))
        .merge(protected_routes)
        .layer(cors)
        .with_state(state);
    
    // Log if API key is enabled
    if std::env::var("LOGAI_API_KEY").ok().filter(|k| !k.is_empty()).is_some() {
        info!("API key authentication ENABLED");
    } else {
        info!("API key authentication DISABLED (set LOGAI_API_KEY to enable)");
    }

    // Server start
    let port = std::env::var("PORT").unwrap_or_else(|_| "3000".to_string());
    let addr = format!("0.0.0.0:{}", port);
    info!("Starting server on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await?;

    Ok(())
}
