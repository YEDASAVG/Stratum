// RAG Engine - Routes queries to appropriate handler based on intent

use crate::causal::{CausalChain, CausalChainAnalyzer};
use crate::groq_client::{GroqClient, GroqError};
use crate::query_analyzer::{AnalyzedQuery, QueryAnalyzer, QueryIntent};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum RagError {
    #[error("Groq error: {0}")]
    Groq(#[from] GroqError),

    #[error("Search failed: {0}")]
    SearchFailed(String),

    #[error("No relevant logs found")]
    NoLogsFound,
    
    #[error("Causal analysis failed: {0}")]
    CausalError(String),
}

// RAG engine configuration
#[derive(Debug, Clone)]
pub struct RagConfig {
    pub groq_model: String,
    pub max_context_logs: usize,
}

impl Default for RagConfig {
    fn default() -> Self {
        Self {
            groq_model: "llama-3.3-70b-versatile".to_string(),
            max_context_logs: 10,
        }
    }
}

impl RagConfig {
    /// Create config from environment variables
    /// 
    /// Environment variables:
    /// - LOGAI_GROQ_MODEL: Groq model name (default: "llama-3.3-70b-versatile")
    /// - LOGAI_MAX_CONTEXT_LOGS: Max logs in context (default: 10)
    pub fn from_env() -> Self {
        let groq_model = std::env::var("LOGAI_GROQ_MODEL")
            .unwrap_or_else(|_| "llama-3.3-70b-versatile".to_string());

        let max_context_logs = std::env::var("LOGAI_MAX_CONTEXT_LOGS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(10);

        Self {
            groq_model,
            max_context_logs,
        }
    }
}



#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RagResponse {
    pub answer: String,
    pub query_analysis: QueryAnalysis,
    pub sources_count: usize,
    pub provider: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub causal_chain: Option<CausalChain>,  // Present when intent is Causal
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryAnalysis {
    pub original_query: String,
    pub search_query: String,
    pub time_filter: Option<String>,
    pub service_filter: Option<String>,
    pub level_filter: Option<String>,
    pub intent: String,
}

pub struct RagEngine {
    config: RagConfig,
    client: GroqClient,
    analyzer: QueryAnalyzer,
    causal_analyzer: CausalChainAnalyzer,
}

impl RagEngine {
    pub fn new(config: RagConfig) -> Self {
        let client = GroqClient::from_env(&config.groq_model)
            .expect("GROQ_API_KEY must be set");
        let causal_client = GroqClient::from_env(&config.groq_model)
            .expect("GROQ_API_KEY must be set");
        let analyzer = QueryAnalyzer::new();
        let causal_analyzer = CausalChainAnalyzer::new(causal_client);

        Self {
            config,
            client,
            analyzer,
            causal_analyzer,
        }
    }

    pub async fn query(
        &self,
        user_query: &str,
        logs: Vec<String>,
    ) -> Result<RagResponse, RagError> {
        let analyzed = self.analyzer.analyze(user_query);

        if logs.is_empty() {
            return Err(RagError::NoLogsFound);
        }

        // Route based on intent
        match analyzed.intent {
            QueryIntent::Causal => self.handle_causal_query(user_query, logs, &analyzed).await,
            _ => self.handle_search_query(user_query, logs, &analyzed).await,
        }
    }

    async fn handle_causal_query(
        &self,
        user_query: &str,
        logs: Vec<String>,
        analyzed: &AnalyzedQuery,
    ) -> Result<RagResponse, RagError> {
        // Try causal analysis, but fall back to normal search if it fails (e.g., rate limit)
        match self.causal_analyzer
            .analyze(user_query, logs.clone(), analyzed.service.as_deref())
            .await
        {
            Ok(chain) => Ok(RagResponse {
                answer: chain.summary.clone(),
                query_analysis: self.build_query_analysis(analyzed),
                sources_count: logs.len(),
                provider: "groq".to_string(),
                causal_chain: Some(chain),
            }),
            Err(e) => {
                // Log the error but fall back to normal search
                tracing::warn!(error = %e, "Causal analysis failed, falling back to search");
                self.handle_search_query(user_query, logs, analyzed).await
            }
        }
    }

    async fn handle_search_query(
        &self,
        user_query: &str,
        logs: Vec<String>,
        analyzed: &AnalyzedQuery,
    ) -> Result<RagResponse, RagError> {
        let context = self.build_context(&logs);
        let prompt = self.build_prompt(user_query, &context);
        let answer = self.client.generate(&prompt).await?;

        Ok(RagResponse {
            answer,
            query_analysis: self.build_query_analysis(analyzed),
            sources_count: logs.len(),
            provider: "groq".to_string(),
            causal_chain: None,
        })
    }

    fn build_query_analysis(&self, analyzed: &AnalyzedQuery) -> QueryAnalysis {
        QueryAnalysis {
            original_query: analyzed.original.clone(),
            search_query: analyzed.search_query.clone(),
            time_filter: analyzed.from.map(|t| t.to_rfc3339()),
            service_filter: analyzed.service.clone(),
            level_filter: analyzed.level.clone(),
            intent: format!("{:?}", analyzed.intent),
        }
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
            r#"You are LogAI, an expert SRE assistant. Analyze logs and answer questions directly.

LOGS:
```
{}
```

QUESTION: {}

RULES:
- Answer the specific question asked - don't follow a template
- Be concise. Skip sections that don't apply
- For "show me X" requests: summarize what you found, highlight patterns
- For "why" questions: give the root cause directly
- For "how to fix" questions: give actionable commands
- Quote specific log lines as evidence when relevant
- If you see the same error repeated, just mention the count, don't list all
- Vary your response structure based on what the user actually asked"#,
            context, query
        )
    }

    pub async fn classify(&self, prompt: &str) -> Result<String, RagError> {
        Ok(self.client.generate(prompt).await?)
    }
}
