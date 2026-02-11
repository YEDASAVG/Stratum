use clickhouse::Client as ClickHouseClient;
use fastembed::TextEmbedding;
use logai_core::parser::ParserRegistry;
use logai_rag::{RagEngine, Reranker};
use qdrant_client::Qdrant;
use std::collections::HashMap;
use std::sync::{Mutex, RwLock};

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
    pub sessions: RwLock<HashMap<String, ChatSession>>,
}
