use clickhouse::Client as ClickHouseClient;
use fastembed::TextEmbedding;
use logai_core::parser::ParserRegistry;
use logai_rag::{JevClient, RagEngine, Reranker};
use qdrant_client::Qdrant;
use std::collections::HashMap;
use std::sync::{Mutex, RwLock};
use std::time::{Duration, Instant};

use crate::models::ChatMessage;

pub const COLLECTION_NAME: &str = "log_embeddings";

#[derive(Clone, Debug)]
pub struct ChatSession {
    pub history: Vec<ChatMessage>,
    pub last_logs: Vec<String>,
    pub last_query: String,
    pub created_at: std::time::Instant,
}

#[derive(Debug, PartialEq)]
pub enum QueryIntent {
    NewSearch,
    FollowUp,
}

pub struct AppState {
    pub nats: async_nats::Client,
    pub qdrant: Qdrant,
    pub clickhouse: ClickHouseClient,
    pub model: Mutex<TextEmbedding>,
    pub parser_registry: ParserRegistry,
    pub rag_engine: RagEngine,
    pub reranker: Reranker,
    /// `None` when `TYPESAFE_API_KEY` is unset; callers fall back to rules.
    pub jev: Option<JevClient>,
    pub sessions: RwLock<HashMap<String, ChatSession>>,
    /// Option set for Jev's service question, so it matches services that
    /// actually exist rather than a hardcoded regex.
    pub service_cache: RwLock<ServiceCache>,
}

#[derive(Debug, Default)]
pub struct ServiceCache {
    pub services: Vec<String>,
    pub refreshed_at: Option<Instant>,
}

impl AppState {
    const SERVICE_CACHE_TTL: Duration = Duration::from_secs(300);

    /// Cached for five minutes. Empty on failure, which leaves service
    /// extraction to the regex.
    pub async fn known_services(&self) -> Vec<String> {
        if let Ok(cache) = self.service_cache.read() {
            if let Some(at) = cache.refreshed_at {
                if at.elapsed() < Self::SERVICE_CACHE_TTL {
                    return cache.services.clone();
                }
            }
        }

        let services = match self
            .clickhouse
            .query("SELECT DISTINCT service FROM logs")
            .fetch_all::<String>()
            .await
        {
            Ok(rows) => rows,
            Err(e) => {
                tracing::warn!(error = %e, "Failed to refresh service list");
                Vec::new()
            }
        };

        if let Ok(mut cache) = self.service_cache.write() {
            cache.services = services.clone();
            cache.refreshed_at = Some(Instant::now());
        }
        services
    }
}
