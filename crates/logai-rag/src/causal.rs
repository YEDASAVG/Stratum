// Causal Chain Analyzer - The "Wow Factor"
// 
// Given "Why did X crash?", this module:
// 1. Finds the crash/error (the "effect")
// 2. Searches backward in time for candidate causes
// 3. Uses LLM to score causal relationships
// 4. Builds a chain: crash ← error ← warning ← root_cause
// 5. Generates human-readable explanation

use crate::groq_client::GroqClient;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::time::{sleep, Duration};

/// A single link in the causal chain
/// Example: "OOMKilled" was caused by "Memory at 95%" with 92% confidence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CausalLink {
    pub effect: LogEvent,          // The thing that happened (e.g., crash)
    pub cause: LogEvent,           // What caused it (e.g., memory warning)
    pub confidence: f64,           // 0.0 - 1.0, how sure we are
    pub explanation: String,       // LLM's explanation of the relationship
}

/// A log event in the causal chain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEvent {
    pub timestamp: DateTime<Utc>,
    pub level: String,
    pub service: String,
    pub message: String,
}

impl LogEvent {
    pub fn from_log_line(line: &str) -> Option<Self> {
        // Try to parse JSON log
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(line) {
            let timestamp = parsed.get("timestamp")
                .or(parsed.get("time"))
                .or(parsed.get("ts"))
                .and_then(|v| v.as_str())
                .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(Utc::now);
            
            let level = parsed.get("level")
                .or(parsed.get("severity"))
                .and_then(|v| v.as_str())
                .unwrap_or("info")
                .to_uppercase();
            
            let service = parsed.get("service")
                .or(parsed.get("app"))
                .or(parsed.get("source"))
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string();
            
            let message = parsed.get("message")
                .or(parsed.get("msg"))
                .and_then(|v| v.as_str())
                .unwrap_or(line)
                .to_string();
            
            return Some(Self { timestamp, level, service, message });
        }
        
        // Fallback: detect level from message content
        let level = if line.to_uppercase().contains("CRITICAL") || line.to_uppercase().contains("FATAL") {
            "CRITICAL".to_string()
        } else if line.to_uppercase().contains("ERROR") || line.contains("500 ") || line.contains("failed") || line.contains("Failed") {
            "ERROR".to_string()
        } else if line.to_uppercase().contains("WARN") {
            "WARN".to_string()
        } else if line.to_uppercase().contains("DEBUG") {
            "DEBUG".to_string()
        } else {
            "INFO".to_string()
        };
        
        Some(Self {
            timestamp: Utc::now(),
            level,
            service: "unknown".to_string(),
            message: line.to_string(),
        })
    }
    
    fn severity_score(&self) -> u8 {
        match self.level.to_uppercase().as_str() {
            "FATAL" | "CRITICAL" => 5,
            "ERROR" | "ERR" => 4,
            "WARN" | "WARNING" => 3,
            "INFO" => 2,
            "DEBUG" => 1,
            _ => 0,
        }
    }
}

/// The complete causal chain from effect to root cause
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CausalChain {
    pub query: String,                       // Original user question
    pub effect: LogEvent,                    // The crash/error being investigated
    pub chain: Vec<CausalLink>,             // Links: effect ← cause ← cause
    pub root_cause: Option<LogEvent>,       // The identified root cause
    pub summary: String,                     // Human-readable explanation
    pub recommendation: Option<String>,     // Suggested fix
}

/// LLM response for causality scoring
#[derive(Debug, Deserialize)]
struct CausalityScore {
    score: u8,           // 0-100
    explanation: String,
}

/// Causal Chain Analyzer
pub struct CausalChainAnalyzer {
    client: GroqClient,
    max_chain_depth: usize,
    min_confidence: f64,
}

impl CausalChainAnalyzer {
    pub fn new(client: GroqClient) -> Self {
        Self {
            client,
            max_chain_depth: 3,   // Reduced from 10
            min_confidence: 0.5,  // Lowered slightly
        }
    }
    
    /// Main entry point: analyze logs and build causal chain
    pub async fn analyze(
        &self,
        query: &str,
        logs: Vec<String>,
        service_filter: Option<&str>,
    ) -> Result<CausalChain, CausalError> {
        if logs.is_empty() {
            return Err(CausalError::NoLogsFound);
        }
        
        tracing::info!(logs_count = logs.len(), "Causal analysis starting");
        
        // Parse all logs into events
        let mut events: Vec<LogEvent> = logs.iter()
            .filter_map(|line| {
                let event = LogEvent::from_log_line(line);
                if let Some(ref e) = event {
                    tracing::debug!(level = %e.level, score = e.severity_score(), msg = %e.message[0..50.min(e.message.len())], "Parsed event");
                }
                event
            })
            .collect();
        
        tracing::info!(events_count = events.len(), "Events parsed");
        
        // Filter by service if specified
        if let Some(svc) = service_filter {
            events.retain(|e| e.service.to_lowercase().contains(&svc.to_lowercase()));
            tracing::info!(after_filter = events.len(), service = svc, "After service filter");
        }
        
        // Count errors
        let error_count = events.iter().filter(|e| e.severity_score() >= 4).count();
        tracing::info!(error_count = error_count, "Error events found");
        
        // Sort by timestamp descending (newest first)
        events.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        
        // Step 1: Find the effect (most severe recent error)
        let effect = self.find_effect(&events)?;
        
        // Step 2: Build chain backward
        let chain = self.build_chain_backward(&effect, &events).await?;
        
        // Step 3: Identify root cause (oldest in chain, or last cause)
        let root_cause = chain.last().map(|link| link.cause.clone());
        
        // Step 4: Generate summary
        let summary = self.generate_summary(query, &effect, &chain, &root_cause).await?;
        
        // Step 5: Generate recommendation
        let recommendation = self.generate_recommendation(&root_cause).await.ok();
        
        Ok(CausalChain {
            query: query.to_string(),
            effect,
            chain,
            root_cause,
            summary,
            recommendation,
        })
    }
    
    /// Find the most severe recent error (the "effect" we're investigating)
    fn find_effect(&self, events: &[LogEvent]) -> Result<LogEvent, CausalError> {
        events.iter()
            .filter(|e| e.severity_score() >= 4) // ERROR or higher
            .max_by_key(|e| (e.severity_score(), e.timestamp))
            .cloned()
            .ok_or(CausalError::NoErrorFound)
    }
    
    /// Build causal chain by going backward in time
    async fn build_chain_backward(
        &self,
        effect: &LogEvent,
        events: &[LogEvent],
    ) -> Result<Vec<CausalLink>, CausalError> {
        let mut chain = Vec::new();
        let mut current_effect = effect.clone();
        
        for _ in 0..self.max_chain_depth {
            // Find candidate causes (logs BEFORE current effect)
            let candidates: Vec<&LogEvent> = events.iter()
                .filter(|e| e.timestamp < current_effect.timestamp)
                .filter(|e| {
                    // Same service or related
                    e.service == current_effect.service || e.severity_score() >= 3
                })
                .take(3) // Limit to 3 candidates to reduce LLM calls
                .collect();
            
            if candidates.is_empty() {
                break;
            }
            
            // Score each candidate for causality
            let mut best_cause: Option<(LogEvent, f64, String)> = None;
            
            for candidate in candidates {
                match self.score_causality(&current_effect, candidate).await {
                    Ok((score, explanation)) => {
                        if score >= self.min_confidence {
                            if best_cause.is_none() || score > best_cause.as_ref().unwrap().1 {
                                best_cause = Some((candidate.clone(), score, explanation));
                            }
                        }
                    }
                    Err(_) => continue,
                }
            }
            
            match best_cause {
                Some((cause, confidence, explanation)) => {
                    chain.push(CausalLink {
                        effect: current_effect.clone(),
                        cause: cause.clone(),
                        confidence,
                        explanation,
                    });
                    
                    // Stop if we hit INFO level (likely root cause)
                    if cause.severity_score() <= 2 {
                        break;
                    }
                    
                    current_effect = cause;
                }
                None => break,
            }
        }
        
        Ok(chain)
    }
    
    /// Ask LLM: "Did A cause B?"
    async fn score_causality(
        &self,
        effect: &LogEvent,
        potential_cause: &LogEvent,
    ) -> Result<(f64, String), CausalError> {
        let prompt = format!(r#"You are analyzing log causality. Given these two log entries:

EFFECT (what happened):
  Time: {}
  Level: {}
  Service: {}
  Message: {}

POTENTIAL CAUSE (happened earlier):
  Time: {}
  Level: {}
  Service: {}
  Message: {}

Rate the likelihood (0-100) that the POTENTIAL CAUSE directly led to the EFFECT.

Respond ONLY with JSON (no markdown):
{{"score": <0-100>, "explanation": "<brief explanation>"}}"#,
            effect.timestamp.format("%H:%M:%S"),
            effect.level,
            effect.service,
            effect.message,
            potential_cause.timestamp.format("%H:%M:%S"),
            potential_cause.level,
            potential_cause.service,
            potential_cause.message
        );
        
        // Retry with backoff for rate limits
        let mut last_error = None;
        for attempt in 0..3 {
            if attempt > 0 {
                sleep(Duration::from_secs(2 * attempt as u64)).await;
            }
            
            match self.client.generate(&prompt).await {
                Ok(response) => {
                    // Parse JSON response
                    let cleaned = response.trim()
                        .trim_start_matches("```json")
                        .trim_start_matches("```")
                        .trim_end_matches("```")
                        .trim();
                    
                    let parsed: CausalityScore = serde_json::from_str(cleaned)
                        .map_err(|e| CausalError::ParseError(format!("Failed to parse LLM response: {} - Response was: {}", e, cleaned)))?;
                    
                    return Ok((parsed.score as f64 / 100.0, parsed.explanation));
                }
                Err(e) => {
                    let err_str = e.to_string();
                    if err_str.contains("rate_limit") || err_str.contains("Rate limit") {
                        last_error = Some(err_str);
                        continue;
                    }
                    return Err(CausalError::LlmError(err_str));
                }
            }
        }
        
        Err(CausalError::LlmError(last_error.unwrap_or_else(|| "Max retries exceeded".to_string())))
    }
    
    /// Generate human-readable summary
    async fn generate_summary(
        &self,
        query: &str,
        effect: &LogEvent,
        chain: &[CausalLink],
        root_cause: &Option<LogEvent>,
    ) -> Result<String, CausalError> {
        let chain_text = chain.iter()
            .enumerate()
            .map(|(i, link)| format!(
                "{}. {} {} → {} {} (confidence: {}%)",
                i + 1,
                link.cause.timestamp.format("%H:%M:%S"),
                link.cause.message.chars().take(50).collect::<String>(),
                link.effect.timestamp.format("%H:%M:%S"),
                link.effect.message.chars().take(50).collect::<String>(),
                (link.confidence * 100.0) as u8
            ))
            .collect::<Vec<_>>()
            .join("\n");
        
        let root_text = root_cause
            .as_ref()
            .map(|r| format!("Root cause: {} at {}", r.message, r.timestamp.format("%H:%M:%S")))
            .unwrap_or_else(|| "Root cause: Unknown".to_string());
        
        let prompt = format!(r#"Based on this causal chain analysis, generate a clear explanation:

Question: {}
Effect: {} at {} - {}
Causal Chain:
{}
{}

Write 2-3 sentences explaining what happened and why. Be specific and actionable."#,
            query,
            effect.level, effect.timestamp.format("%H:%M:%S"), effect.message,
            chain_text,
            root_text
        );
        
        self.client.generate(&prompt).await
            .map_err(|e| CausalError::LlmError(e.to_string()))
    }
    
    /// Generate fix recommendation
    async fn generate_recommendation(&self, root_cause: &Option<LogEvent>) -> Result<String, CausalError> {
        let root = root_cause.as_ref().ok_or(CausalError::NoRootCause)?;
        
        let prompt = format!(r#"Given this root cause log entry:
Level: {}
Service: {}
Message: {}

Suggest 1-2 actionable fixes. Be specific. Include commands if applicable."#,
            root.level, root.service, root.message
        );
        
        self.client.generate(&prompt).await
            .map_err(|e| CausalError::LlmError(e.to_string()))
    }
}

#[derive(Debug)]
pub enum CausalError {
    NoLogsFound,
    NoErrorFound,
    NoRootCause,
    LlmError(String),
    ParseError(String),
}

impl std::fmt::Display for CausalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CausalError::NoLogsFound => write!(f, "No logs found"),
            CausalError::NoErrorFound => write!(f, "No error logs found to investigate"),
            CausalError::NoRootCause => write!(f, "Could not identify root cause"),
            CausalError::LlmError(e) => write!(f, "LLM error: {}", e),
            CausalError::ParseError(e) => write!(f, "Parse error: {}", e),
        }
    }
}

impl std::error::Error for CausalError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_event_parsing() {
        let json_log = r#"{"timestamp":"2026-02-10T03:00:05Z","level":"ERROR","service":"payment","message":"OOMKilled"}"#;
        let event = LogEvent::from_log_line(json_log).unwrap();
        
        assert_eq!(event.level, "ERROR");
        assert_eq!(event.service, "payment");
        assert_eq!(event.message, "OOMKilled");
    }

    #[test]
    fn test_severity_score() {
        let fatal = LogEvent {
            timestamp: Utc::now(),
            level: "FATAL".to_string(),
            service: "test".to_string(),
            message: "crash".to_string(),
        };
        assert_eq!(fatal.severity_score(), 5);
        
        let error = LogEvent {
            timestamp: Utc::now(),
            level: "ERROR".to_string(),
            service: "test".to_string(),
            message: "error".to_string(),
        };
        assert_eq!(error.severity_score(), 4);
    }
}
