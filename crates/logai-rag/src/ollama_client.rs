// Ollama Local LLM client

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::llm_client::{LlmClient, LlmError};

#[derive(Debug, Clone)]
pub struct OllamaClient {
    client: Client,
    base_url: String,
    model: String,
}

#[derive(Serialize)]
struct GenerateRequest<'a> {
    model: &'a str,
    prompt: &'a str,
    stream: bool,
    options: GenerateOptions,
}

#[derive(Serialize)]
struct GenerateOptions {
    temperature: f32,
    num_predict: u32,
}

#[derive(Deserialize)]
struct GenerateResponse {
    response: String,
}

impl OllamaClient {
    pub fn new(base_url: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            client: Client::new(),
            base_url: base_url.into(),
            model: model.into(),
        }
    }

    /// Create from environment variables
    /// - OLLAMA_URL: Base URL (default: http://localhost:11434)
    /// - OLLAMA_MODEL: Model name (default: llama3.2:3b)
    pub fn from_env() -> Result<Self, LlmError> {
        let base_url = std::env::var("OLLAMA_URL")
            .unwrap_or_else(|_| "http://localhost:11434".to_string());
        let model = std::env::var("OLLAMA_MODEL")
            .unwrap_or_else(|_| "llama3.2:3b".to_string());
        
        Ok(Self::new(base_url, model))
    }
}

#[async_trait]
impl LlmClient for OllamaClient {
    async fn generate(&self, prompt: &str) -> Result<String, LlmError> {
        let url = format!("{}/api/generate", self.base_url);
        
        let request = GenerateRequest {
            model: &self.model,
            prompt: &format!("You are a log analysis expert. Be concise and actionable.\n\n{}", prompt),
            stream: false,
            options: GenerateOptions {
                temperature: 0.3,
                num_predict: 1024,
            },
        };

        let response = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| LlmError::RequestFailed(e.to_string()))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(LlmError::ApiError(error_text));
        }

        let result: GenerateResponse = response
            .json()
            .await
            .map_err(|e| LlmError::ApiError(format!("Failed to parse response: {}", e)))?;
        
        Ok(result.response)
    }

    fn model(&self) -> &str {
        &self.model
    }

    fn provider(&self) -> &str {
        "ollama"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = OllamaClient::new("http://localhost:11434", "llama3.2:3b");
        assert_eq!(client.model(), "llama3.2:3b");
        assert_eq!(client.provider(), "ollama");
    }
}
