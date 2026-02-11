// Groq Cloud LLM client

use std::vec;

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::llm_client::{LlmClient, LlmError};

#[derive(Error, Debug)]
pub enum GroqError {
    #[error("HTTP request failed: {0}")]
    RequestFailed(#[from] reqwest::Error),

    #[error("Groq API error: {0}")]
    ApiError(String),

    #[error("Missing API key")]
    MissingApiKey,
}

#[derive(Debug, Clone)]
pub struct GroqClient {
    client: Client,
    api_key: String,
    model: String,
}

#[derive(Serialize)]
struct ChatRequest<'a> {
    model: &'a str,
    messages: Vec<Message<'a>>,
    temperature: f32,
    max_tokens: u32,
}

#[derive(Serialize)]
struct Message<'a> {
    role: &'a str,
    content: &'a str,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: ResponseMessage,
}

#[derive(Deserialize)]
struct ResponseMessage {
    content: String,
}

impl GroqClient {
    const BASE_URL: &'static str = "https://api.groq.com/openai/v1/chat/completions";

    pub fn new(api_key: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            client: Client::new(),
            api_key: api_key.into(),
            model: model.into(),
        }
    }
    
    /// Create from env GROQ_API_KEY
    pub fn from_env(model: impl Into<String>) -> Result<Self, GroqError> {
        let api_key = std::env::var("GROQ_API_KEY").map_err(|_| GroqError::MissingApiKey)?;
        Ok(Self::new(api_key, model))
    }

    /// Generate text from prompt (returns GroqError for internal use)
    pub async fn generate(&self, prompt: &str) -> Result<String, GroqError> {
        let request = ChatRequest {
            model: &self.model,
            messages: vec![
                Message {
                    role: "system",
                    content: "You are a log analysis expert. Be concise and actionable.",
                },
                Message {
                    role: "user",
                    content: prompt,
                },
            ],
            temperature: 0.3,
            max_tokens: 1024,
        };
        let response = self
            .client
            .post(Self::BASE_URL)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(GroqError::ApiError(error_text));
        }
        let result: ChatResponse = response.json().await?;
        result
            .choices
            .into_iter()
            .next()
            .map(|c| c.message.content)
            .ok_or_else(|| GroqError::ApiError("No response".to_string()))
    }
    
    /// Get model name
    pub fn model_name(&self) -> &str {
        &self.model
    }
}

#[async_trait]
impl LlmClient for GroqClient {
    async fn generate(&self, prompt: &str) -> Result<String, LlmError> {
        // Call the inherent method and convert error
        GroqClient::generate(self, prompt)
            .await
            .map_err(|e| LlmError::ApiError(e.to_string()))
    }

    fn model(&self) -> &str {
        &self.model
    }

    fn provider(&self) -> &str {
        "groq"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = GroqClient::new("test-key", "llama-3.3-70b-versatile");
        assert_eq!(client.model(), "llama-3.3-70b-versatile");
    }
}
