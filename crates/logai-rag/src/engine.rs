// RAG engine
// Orchestrates: Query Analysis -> Semantic Search -> Context Building -> LLM response

use crate::llm_client::{LlmError, OllamaClient};
use crate::query_analyzer::{AnalyzedQuery, QueryAnalyzer};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum RagError {
    #[error("LLM error: {0}")]
    Llm(#[from] LlmError),

    #[error("Search failed: {0}")]
    SearchFailed(String),

    #[error("No relevant logs found")]
    NoLogsFound,
}

// RAG engine configuration
#[derive(Debug, Clone)]
pub struct RagConfig {
    pub ollama_url: String,
    pub model: String,
    pub qdrant_url: String,
    pub max_context_logs: usize,
}

impl Default for RagConfig {
    fn default() -> Self {
        Self {
            ollama_url: "http://localhost:11434".to_string(),
            model: "qwen3:8b".to_string(),
            qdrant_url: "http://localhost:6334".to_string(),
            max_context_logs: 20,
        }
    }
}

// RAG response with answer and sources
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RagResponse {
    pub answer: String,
    pub query_analysis: QueryAnalysis,
    pub sources_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryAnalysis {
    pub original_query: String,
    pub search_query: String,
    pub time_filter: Option<String>,
    pub service_filter: Option<String>,
}

// main RAg engine
pub struct RagEngine {
    config: RagConfig,
    llm: OllamaClient,
    analyzer: QueryAnalyzer,
}

impl RagEngine {
    pub fn new(config: RagConfig) -> Self {
        let llm = OllamaClient::new(&config.ollama_url, &config.model);
        let analyzer = QueryAnalyzer::new();

        Self {
            config,
            llm,
            analyzer,
        }
    }

    // process a natural lang query aboout logs
    pub async fn query(
        &self,
        user_query: &str,
        logs: Vec<String>,
    ) -> Result<RagResponse, RagError> {
        let analyzed = self.analyzer.analyze(user_query); // analyze the query

        if logs.is_empty() {
            return Err(RagError::NoLogsFound); // check if we have logs
        }

        let context = self.build_context(&logs); // build the context from logs

        let prompt = self.build_prompt(user_query, &context); // Build prompt

        let answer = self.llm.generate(&prompt).await?;

        // Building the response
        Ok(RagResponse {
            answer,
            query_analysis: QueryAnalysis {
                original_query: analyzed.original,
                search_query: analyzed.search_query,
                time_filter: analyzed.from.map(|t| t.to_rfc3339()),
                service_filter: analyzed.service,
            },
            sources_count: logs.len(),
        })
    }

    // get analyzed query (for API to use in search)
    pub fn analyze_query(&self, query: &str) -> AnalyzedQuery {
        self.analyzer.analyze(query)
    }

    fn build_context(&self, logs: &[String]) -> String {
        let max_logs = self.config.max_context_logs.min(logs.len());
        logs[..max_logs].join("\n")
    }

    fn build_prompt(&self, query: &str, context: &str) -> String {
        format!(
            r#"You are a log analysis expert. Analyze the following logs and answer the user's question.

USER QUESTION: {}

RELEVANT LOGS:
{}

INSTRUCTIONS:
1. Analyze the logs carefully
2. Identify patterns, errors, or anomalies relevant to the question
3. Provide a clear, concise answer
4. If you see errors, explain the likely cause
5. Suggest fixes if applicable

ANSWER:"#,
            query, context
        )
    }
}
