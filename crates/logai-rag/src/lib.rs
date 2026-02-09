// log ai RAG engine

// Natural language query processing for log anaylses
// semantic search + LLM-powered analysis

pub mod llm_client;
pub mod query_analyzer;
pub mod engine;

pub use llm_client::OllamaClient;
pub use query_analyzer::{AnalyzedQuery, QueryAnalyzer};
pub use engine::{RagEngine, RagConfig, RagResponse, QueryAnalysis};
