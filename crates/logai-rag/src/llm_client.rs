// Ollama LLM client
// HTTP client for Ollama's /api/generate endpoint

use reqwest::Client;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum LlmError {
    #[error("HTTP request failed: {0}")]
    RequestFailed(#[from] reqwest::Error),

    #[error("Ollama returned error: {0}")]
    OllamaError(String),
}

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
}

#[derive(Deserialize)]
struct GenerateResponse {
    response: String,
    #[allow(dead_code)]
    done: bool,
}

impl OllamaClient {
    // create new client

    pub fn new(base_url: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            client: Client::new(),
            base_url: base_url.into(),
            model: model.into(),
        }
    }

    // generate text from prompt
    pub async fn generate(&self, prompt: &str) -> Result<String, LlmError> {
        let url = format!("{}/api/generate", self.base_url);

        let request = GenerateRequest {
            model: &self.model,
            prompt,
            stream: false,
        };
        let response = self.client
        .post(&url)
        .json(&request)
        .send()
        .await?;

        if !response.status().is_success(){
            let error_text = response.text().await.unwrap_or_default();
            return Err(LlmError::OllamaError(error_text));
        }
        let result: GenerateResponse = response.json().await?;
        Ok(result.response)
    }
    // get model name
    pub fn model(&self) -> &str{
        &self.model
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = OllamaClient::new("http://localhost:11434", "qwen3:8b");
        assert_eq!(client.model(), "qwen3:8b");
    }
}