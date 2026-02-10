// RAG engine
// Orchestrates: Query Analysis -> Semantic Search -> Context Building -> LLM response

use crate::groq_client::{GroqClient, GroqError};
use crate::llm_client::{LlmError, OllamaClient};
use crate::query_analyzer::{AnalyzedQuery, QueryAnalyzer};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum RagError {
    #[error("Ollama error: {0}")]
    Ollama(#[from] LlmError),

    #[error("Groq error: {0}")]
    Groq(#[from] GroqError),

    #[error("Search failed: {0}")]
    SearchFailed(String),

    #[error("No relevant logs found")]
    NoLogsFound,
}

/// LLM Provider selection
#[derive(Debug, Clone, Default)]
pub enum LlmProvider {
    #[default]
    Ollama,
    Groq,
}

// RAG engine configuration
#[derive(Debug, Clone)]
pub struct RagConfig {
    pub provider: LlmProvider,
    pub ollama_url: String,
    pub ollama_model: String,
    pub groq_model: String,
    pub max_context_logs: usize,
}

impl Default for RagConfig {
    fn default() -> Self {
        Self {
            provider: LlmProvider::Ollama,
            ollama_url: "http://localhost:11434".to_string(),
            ollama_model: "qwen3:8b".to_string(),
            groq_model: "llama-3.3-70b-versatile".to_string(),
            max_context_logs: 10,
        }
    }
}

impl RagConfig {
    /// Create config for Groq provider
    pub fn with_groq() -> Self {
        Self {
            provider: LlmProvider::Groq,
            ..Default::default()
        }
    }

    /// Create config for Ollama provider
    pub fn with_ollama() -> Self {
        Self {
            provider: LlmProvider::Ollama,
            ..Default::default()
        }
    }
}

/// LLM Backend - holds active client
enum LlmBackend {
    Ollama(OllamaClient),
    Groq(GroqClient),
}

impl LlmBackend {
    async fn generate(&self, prompt: &str) -> Result<String, RagError> {
        match self {
            LlmBackend::Ollama(client) => Ok(client.generate(prompt).await?),
            LlmBackend::Groq(client) => Ok(client.generate(prompt).await?),
        }
    }
}

// RAG response with answer and sources
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RagResponse {
    pub answer: String,
    pub query_analysis: QueryAnalysis,
    pub sources_count: usize,
    pub provider: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryAnalysis {
    pub original_query: String,
    pub search_query: String,
    pub time_filter: Option<String>,
    pub service_filter: Option<String>,
}

// main RAG engine
pub struct RagEngine {
    config: RagConfig,
    backend: LlmBackend,
    analyzer: QueryAnalyzer,
}

impl RagEngine {
    pub fn new(config: RagConfig) -> Self {
        let backend = match &config.provider {
            LlmProvider::Ollama => {
                let client = OllamaClient::new(&config.ollama_url, &config.ollama_model);
                LlmBackend::Ollama(client)
            }
            LlmProvider::Groq => {
                // Load from environment
                let client = GroqClient::from_env(&config.groq_model)
                    .expect("GROQ_API_KEY must be set for Groq provider");
                LlmBackend::Groq(client)
            }
        };
        let analyzer = QueryAnalyzer::new();

        Self {
            config,
            backend,
            analyzer,
        }
    }

    /// Get provider name
    fn provider_name(&self) -> String {
        match &self.backend {
            LlmBackend::Ollama(_) => "ollama".to_string(),
            LlmBackend::Groq(_) => "groq".to_string(),
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

        let answer = self.backend.generate(&prompt).await?;

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
            provider: self.provider_name(),
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
            r#"You are LogAI, an elite Site Reliability Engineer with 15+ years of experience debugging complex distributed systems at companies like Google, Netflix, and Amazon.

## YOUR EXPERTISE
- Root cause analysis of production incidents
- Pattern recognition across microservices
- Performance bottleneck identification
- Security threat detection in logs
- Correlation of events across distributed systems

## RELEVANT LOGS
```
{}
```

## USER QUERY
{}

## ANALYSIS FRAMEWORK

### For Error Investigation:
1. **Identify**: What specific error(s) occurred? Extract error codes, messages, stack traces
2. **Timeline**: When did it start? Is it ongoing or resolved?
3. **Scope**: Which services/users/endpoints are affected?
4. **Root Cause**: What triggered this? (deployment, traffic spike, dependency failure, resource exhaustion)
5. **Impact**: What's the blast radius? User-facing? Data integrity?
6. **Fix**: Immediate mitigation + permanent solution

### For Performance Issues:
1. **Baseline**: What's normal vs current behavior?
2. **Bottleneck**: CPU? Memory? I/O? Network? Database?
3. **Pattern**: Sudden spike or gradual degradation?
4. **Correlation**: What changed before the issue started?

### For Security Concerns:
1. **Threat Type**: Brute force? Injection? Unauthorized access?
2. **Attack Vector**: Which endpoint/service is targeted?
3. **Indicators**: IPs, user agents, request patterns
4. **Severity**: Critical/High/Medium/Low

## RESPONSE GUIDELINES
- Be DIRECT and ACTIONABLE - engineers are debugging under pressure
- Use bullet points for clarity
- Include specific log lines as evidence
- Suggest concrete next steps with commands when applicable
- If logs are insufficient, say what additional data would help
- For follow-up questions, reference previous context naturally

## RESPONSE FORMAT
Start with a 1-line summary, then detailed analysis. No fluff."#,
            context, query
        )
    }

    pub async fn classify(&self, prompt: &str) -> Result<String, RagError> {
        self.backend.generate(prompt).await
    }
}
