// TypeSafe Jev client - typed decisions with calibrated probabilities
// POST https://api.typesafe.ai/v1/systemone

use std::collections::HashMap;
use std::time::Duration;

use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum JevError {
    #[error("HTTP request failed: {0}")]
    RequestFailed(#[from] reqwest::Error),

    #[error("TypeSafe API error ({status}): {body}")]
    ApiError { status: u16, body: String },

    #[error("Missing TYPESAFE_API_KEY")]
    MissingApiKey,

    #[error("Answer {0:?} missing from response")]
    MissingAnswer(String),

    #[error("Answer {0:?} had unexpected type")]
    WrongAnswerType(String),
}

/// One typed answer, discriminated by the `type` field.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Answer {
    Noul {
        noul: f32,
    },
    Choice {
        choice: String,
        confidence: f32,
        probabilities: HashMap<String, f32>,
    },
    Score {
        score: f32,
        confidence: f32,
        legend: HashMap<String, String>,
        probabilities: HashMap<String, f32>,
    },
}

impl Answer {
    pub fn as_noul(&self) -> Option<f32> {
        match self {
            Answer::Noul { noul } => Some(*noul),
            _ => None,
        }
    }

    pub fn as_choice(&self) -> Option<(&str, f32)> {
        match self {
            Answer::Choice { choice, confidence, .. } => Some((choice.as_str(), *confidence)),
            _ => None,
        }
    }

    pub fn as_score(&self) -> Option<(f32, f32)> {
        match self {
            Answer::Score { score, confidence, .. } => Some((*score, *confidence)),
            _ => None,
        }
    }

    pub fn probabilities(&self) -> Option<&HashMap<String, f32>> {
        match self {
            Answer::Choice { probabilities, .. } | Answer::Score { probabilities, .. } => {
                Some(probabilities)
            }
            Answer::Noul { .. } => None,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Usage {
    pub input_tokens: u32,
    pub output_tokens: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SystemOneResponse {
    pub model: String,
    pub answers: HashMap<String, Answer>,
    pub usage: Usage,
}

impl SystemOneResponse {
    pub fn noul(&self, id: &str) -> Result<f32, JevError> {
        self.answers
            .get(id)
            .ok_or_else(|| JevError::MissingAnswer(id.to_string()))?
            .as_noul()
            .ok_or_else(|| JevError::WrongAnswerType(id.to_string()))
    }

    pub fn choice(&self, id: &str) -> Result<(&str, f32), JevError> {
        self.answers
            .get(id)
            .ok_or_else(|| JevError::MissingAnswer(id.to_string()))?
            .as_choice()
            .ok_or_else(|| JevError::WrongAnswerType(id.to_string()))
    }

    pub fn score(&self, id: &str) -> Result<(f32, f32), JevError> {
        self.answers
            .get(id)
            .ok_or_else(|| JevError::MissingAnswer(id.to_string()))?
            .as_score()
            .ok_or_else(|| JevError::WrongAnswerType(id.to_string()))
    }
}

#[derive(Serialize)]
struct SystemOneRequest<'a> {
    state: &'a Value,
    model: &'a str,
    questions: &'a Value,
}

#[derive(Debug, Clone)]
pub struct JevClient {
    client: Client,
    api_key: String,
    model: String,
}

impl JevClient {
    const BASE_URL: &'static str = "https://api.typesafe.ai/v1/systemone";
    const DEFAULT_MODEL: &'static str = "jev-latest";

    /// Tight on purpose: Jev answers in ~100ms, so a slow call means fall
    /// back rather than stall a user-facing request.
    const TIMEOUT: Duration = Duration::from_secs(5);

    pub fn new(api_key: impl Into<String>, model: impl Into<String>) -> Self {
        let model = model.into();
        let model = if model.trim().is_empty() {
            Self::DEFAULT_MODEL.to_string()
        } else {
            model
        };
        Self {
            client: Client::builder()
                .timeout(Self::TIMEOUT)
                .build()
                .unwrap_or_default(),
            api_key: api_key.into(),
            model,
        }
    }

    /// From `TYPESAFE_API_KEY` and `JEV_MODEL`. `MissingApiKey` means "not
    /// configured" and callers fall back to their rules path.
    pub fn from_env() -> Result<Self, JevError> {
        let api_key = std::env::var("TYPESAFE_API_KEY")
            .ok()
            .filter(|k| !k.trim().is_empty())
            .ok_or(JevError::MissingApiKey)?;
        // `JEV_MODEL=` in .env is Ok(""), not Err, and the API rejects an
        // empty model on every request.
        let model = std::env::var("JEV_MODEL")
            .ok()
            .map(|m| m.trim().to_string())
            .filter(|m| !m.is_empty())
            .unwrap_or_else(|| Self::DEFAULT_MODEL.to_string());
        Ok(Self::new(api_key, model))
    }

    pub fn model(&self) -> &str {
        &self.model
    }

    /// Evaluate one `state` against a map of typed `questions`. Questions are
    /// answered in parallel, so batch everything the caller might need.
    pub async fn system_one(
        &self,
        state: &Value,
        questions: &Value,
    ) -> Result<SystemOneResponse, JevError> {
        let request = SystemOneRequest {
            state,
            model: &self.model,
            questions,
        };

        // Retry once on the documented transient statuses.
        let mut attempt = 0;
        loop {
            let response = self
                .client
                .post(Self::BASE_URL)
                .header("Authorization", format!("Bearer {}", self.api_key))
                .header("Content-Type", "application/json")
                .json(&request)
                .send()
                .await?;

            let status = response.status();
            if status.is_success() {
                return Ok(response.json().await?);
            }

            let transient = status == StatusCode::TOO_MANY_REQUESTS || status.as_u16() == 529;
            if transient && attempt == 0 {
                attempt += 1;
                tokio::time::sleep(Duration::from_millis(250)).await;
                continue;
            }

            let body = response.text().await.unwrap_or_default();
            return Err(JevError::ApiError {
                status: status.as_u16(),
                body,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = JevClient::new("test-key", "jev-latest");
        assert_eq!(client.model(), "jev-latest");
    }

    #[test]
    fn test_blank_model_falls_back_to_default() {
        let client = JevClient::new("test-key", "");
        assert_eq!(client.model(), "jev-latest");
        let client = JevClient::new("test-key", "   ");
        assert_eq!(client.model(), "jev-latest");
    }

    #[test]
    fn test_parse_choice_answer() {
        let raw = r#"{
            "model": "jev-1.13.0",
            "answers": {
                "intent": {
                    "type": "choice",
                    "choice": "causal",
                    "confidence": 0.82,
                    "probabilities": {"causal": 0.9, "search": 0.1}
                },
                "urgent": {"type": "noul", "noul": 0.93}
            },
            "usage": {"input_tokens": 312, "output_tokens": 48}
        }"#;

        let parsed: SystemOneResponse = serde_json::from_str(raw).unwrap();
        assert_eq!(parsed.choice("intent").unwrap(), ("causal", 0.82));
        assert_eq!(parsed.noul("urgent").unwrap(), 0.93);
        assert_eq!(parsed.usage.input_tokens, 312);
        assert!(parsed.noul("intent").is_err());
        assert!(parsed.choice("missing").is_err());
    }

    #[test]
    fn test_parse_score_answer() {
        let raw = r#"{
            "model": "jev-1.13.0",
            "answers": {
                "sev": {
                    "type": "score",
                    "score": 1.3,
                    "confidence": 0.54,
                    "legend": {"0": "low", "1": "mid", "2": "high"},
                    "probabilities": {"0": 0.0, "1": 0.7, "2": 0.3}
                }
            },
            "usage": {"input_tokens": 10, "output_tokens": 2}
        }"#;

        let parsed: SystemOneResponse = serde_json::from_str(raw).unwrap();
        let (score, confidence) = parsed.score("sev").unwrap();
        assert_eq!(score, 1.3);
        assert_eq!(confidence, 0.54);
    }
}
