// LLM Client Abstraction - Supports multiple LLM providers

use async_trait::async_trait;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum LlmError {
    #[error("HTTP request failed: {0}")]
    RequestFailed(String),

    #[error("API error: {0}")]
    ApiError(String),

    #[error("Missing configuration: {0}")]
    MissingConfig(String),
}

/// Common trait for all LLM clients
#[async_trait]
pub trait LlmClient: Send + Sync {
    /// Generate text from a prompt
    async fn generate(&self, prompt: &str) -> Result<String, LlmError>;
    
    /// Get the model name
    fn model(&self) -> &str;
    
    /// Get the provider name (e.g., "groq", "ollama")
    fn provider(&self) -> &str;
}

/// LLM Provider type
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LlmProvider {
    Groq,
    Ollama,
}

impl LlmProvider {
    pub fn from_env() -> Self {
        match std::env::var("LLM_PROVIDER").as_deref() {
            Ok("ollama") => LlmProvider::Ollama,
            _ => LlmProvider::Groq, // Default to Groq
        }
    }
    
    pub fn as_str(&self) -> &'static str {
        match self {
            LlmProvider::Groq => "groq",
            LlmProvider::Ollama => "ollama",
        }
    }
}
