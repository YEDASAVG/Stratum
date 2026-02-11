use axum::{
    extract::{Query, State},
    http::StatusCode,
    Json,
};
use chrono::{DateTime, Utc};
use qdrant_client::qdrant::{Condition, Filter, Range, ScrollPointsBuilder, SearchPointsBuilder};
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Instant;
use tracing::info;

use crate::handlers::get_string;
use crate::models::{ApiError, ChatApiResponse, ChatMessage, ChatRequest, CausalChainResponse, SessionInfo, SessionQuery};
use crate::state::{AppState, ChatSession, QueryIntent, COLLECTION_NAME};

// Import RAG's QueryIntent (different from our local one)
use logai_rag::QueryIntent as RagQueryIntent;

pub async fn chat_logs(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ChatRequest>,
) -> Result<Json<ChatApiResponse>, (StatusCode, Json<ApiError>)> {
    let start = Instant::now();
    info!(session = %req.session_id, message = %req.message, "CHAT request");
    
    // Configurable max logs (default: 20)
    let max_context_logs: usize = std::env::var("LOGAI_MAX_CONTEXT_LOGS")
        .ok().and_then(|s| s.parse().ok()).unwrap_or(20);

    let msg_lower = req.message.to_lowercase().trim().to_string();
    let greetings = ["hi", "hello", "hey", "good morning", "good afternoon", "good evening", "howdy", "sup", "what's up", "yo"];
    let is_greeting = greetings.iter().any(|g| msg_lower == *g || msg_lower.starts_with(&format!("{} ", g)));

    let gibberish_patterns = ["asdf", "qwer", "zxcv", "hjkl", "jkl;"];
    let is_gibberish = gibberish_patterns.iter().any(|p| msg_lower.contains(p));

    let log_keywords = ["error", "log", "warn", "debug", "info", "service", "api", "database", "db",
        "timeout", "slow", "failed", "failure", "crash", "down", "outage", "issue", "problem",
        "anomal", "incident", "alert", "critical", "auth", "payment", "nginx", "redis", "kafka",
        "query", "connection", "latency", "performance", "traffic", "request", "response",
        "yesterday", "today", "last hour", "last minute", "recent", "happened", "show me", "find"];
    let has_log_context = log_keywords.iter().any(|k| msg_lower.contains(k));

    let is_offtopic = if !has_log_context && msg_lower.len() > 5 {
        let classification = state.rag_engine.classify(&format!(
            r#"Is this question about analyzing logs, debugging, system errors, or infrastructure monitoring?
Question: "{}"
Answer YES or NO only."#,
            req.message
        )).await;

        match classification {
            Ok(response) => !response.to_uppercase().contains("YES"),
            Err(_) => false,
        }
    } else {
        false
    };

    if is_greeting {
        let elapsed = start.elapsed().as_millis();
        return Ok(Json(ChatApiResponse {
            answer: "Hello! I'm LogAI, your log analysis assistant. Ask me about errors, performance issues, or anomalies in your logs. For example:\n\n• \"Show me errors in the last hour\"\n• \"What happened yesterday?\"\n• \"Why is the payment service slow?\"\n• \"Summarize auth failures\"".to_string(),
            sources_count: 0,
            response_time_ms: elapsed,
            provider: "system".to_string(),
            context_logs: 0,
            conversation_turn: 1,
            source_logs: vec![],
            causal_chain: None,
        }));
    }

    if is_gibberish || is_offtopic {
        let elapsed = start.elapsed().as_millis();
        return Ok(Json(ChatApiResponse {
            answer: "I'm LogAI - I specialize in analyzing your system logs. I can help with:\n\n• Finding errors and warnings\n• Investigating performance issues\n• Summarizing anomalies and incidents\n• Debugging service failures\n\nTry: \"Show me errors in the last hour\" or \"Why is the database slow?\"".to_string(),
            sources_count: 0,
            response_time_ms: elapsed,
            provider: "system".to_string(),
            context_logs: 0,
            conversation_turn: 1,
            source_logs: vec![],
            causal_chain: None,
        }));
    }

    let (history, last_logs, last_query, turn) = {
        let mut sessions = state.sessions.write().unwrap();
        let session = sessions.entry(req.session_id.clone()).or_insert_with(|| {
            ChatSession {
                history: Vec::new(),
                last_logs: Vec::new(),
                last_query: String::new(),
                created_at: std::time::Instant::now(),
            }
        });
        if !req.history.is_empty() && session.history.is_empty() {
            session.history = req.history.clone();
        }
        (
            session.history.clone(),
            session.last_logs.clone(),
            session.last_query.clone(),
            session.history.len() / 2 + 1,
        )
    };

    let intent = classify_query_intent(&state.rag_engine, &last_query, &req.message).await;
    info!(intent = ?intent, "Query intent classified");

    // Always check if current message is a causal query (even for follow-ups)
    let analyzed = state.rag_engine.analyze_query(&req.message);
    let is_causal_query = analyzed.intent == RagQueryIntent::Causal;
    info!(
        is_causal = is_causal_query, 
        rag_intent = ?analyzed.intent, 
        will_use_cached = (intent == QueryIntent::FollowUp && !last_logs.is_empty() && !is_causal_query),
        "RAG intent detected"
    );

    // For causal queries, always fetch fresh logs with temporal context
    // For non-causal follow-ups, use cached logs
    let logs = if intent == QueryIntent::FollowUp && !last_logs.is_empty() && !is_causal_query {
        info!("Using cached logs from previous turn (non-causal follow-up)");
        last_logs
    } else {
        info!(
            is_follow_up = (intent == QueryIntent::FollowUp),
            is_causal = is_causal_query,
            has_last_logs = !last_logs.is_empty(),
            "Fetching fresh logs for causal query or new search"
        );

        let query_vector = {
            let mut model = state.model.lock().unwrap();
            let embeddings = model
                .embed(vec![analyzed.search_query.clone()], None)
                .map_err(|e| ApiError::internal(e.to_string()))?;
            embeddings.into_iter().next().ok_or_else(|| ApiError::internal("No embedding"))?
        };

        let mut conditions = vec![];
        if let Some(from) = analyzed.from {
            info!(from = %from, "Time filter: FROM");
            conditions.push(Condition::range(
                "timestamp_unix",
                Range {
                    gte: Some(from.timestamp() as f64),
                    ..Default::default()
                },
            ));
        }
        if let Some(to) = analyzed.to {
            info!(to = %to, "Time filter: TO");
            conditions.push(Condition::range(
                "timestamp_unix",
                Range {
                    lte: Some(to.timestamp() as f64),
                    ..Default::default()
                },
            ));
        }
        // Note: service/level filters removed - semantic search handles relevance

        let filter = if conditions.is_empty() {
            None
        } else {
            Some(Filter::must(conditions))
        };

        let mut search_builder =
            SearchPointsBuilder::new(COLLECTION_NAME, query_vector, 100).with_payload(true);
        if let Some(f) = filter.clone() {
            search_builder = search_builder.filter(f);
        }

        let results = state
            .qdrant
            .search_points(search_builder)
            .await
            .map_err(|e| ApiError::internal(e.to_string()))?;

        // Build JSON log strings with full metadata for causal analysis
        let logs_with_scores: Vec<(String, f32)> = results
            .result
            .iter()
            .map(|point| {
                let payload = &point.payload;
                let log_json = serde_json::json!({
                    "timestamp": get_string(payload, "timestamp"),
                    "level": get_string(payload, "level"),
                    "service": get_string(payload, "service"),
                    "message": get_string(payload, "message"),
                });
                (log_json.to_string(), point.score)
            })
            .collect();

        info!(logs_found = logs_with_scores.len(), "Logs retrieved via semantic search");

        if logs_with_scores.is_empty() {
            return Err(ApiError::not_found("No relevant logs found for your query. Try broadening your search."));
        }

        // For CAUSAL queries: augment with time-window retrieval
        let final_logs = if is_causal_query {
            info!("Causal query detected - fetching temporal context");
            
            // Find the most recent ERROR from semantic search results
            let effect_timestamp = find_effect_timestamp(&logs_with_scores);
            
            if let Some(effect_time) = effect_timestamp {
                info!(effect_time = %effect_time, "Found effect timestamp, fetching 5-min window");
                
                // Fetch all logs from (effect_time - 5 minutes) to effect_time
                let window_start = effect_time.timestamp() - 300; // 5 minutes before
                let window_end = effect_time.timestamp();
                
                let time_filter = Filter::must(vec![
                    Condition::range(
                        "timestamp_unix",
                        Range {
                            gte: Some(window_start as f64),
                            lte: Some(window_end as f64),
                            ..Default::default()
                        },
                    ),
                ]);
                
                // Scroll to get ALL logs in the time window (not just semantically similar)
                let scroll_request = ScrollPointsBuilder::new(COLLECTION_NAME)
                    .filter(time_filter)
                    .limit(200)
                    .with_payload(true);
                
                let scroll_result = state
                    .qdrant
                    .scroll(scroll_request)
                    .await
                    .map_err(|e| ApiError::internal(format!("Scroll failed: {}", e)))?;
                
                let window_logs: Vec<(String, f32)> = scroll_result
                    .result
                    .iter()
                    .map(|point| {
                        let payload = &point.payload;
                        let log_json = serde_json::json!({
                            "timestamp": get_string(payload, "timestamp"),
                            "level": get_string(payload, "level"),
                            "service": get_string(payload, "service"),
                            "message": get_string(payload, "message"),
                        });
                        // Give time-window logs a base score of 0.5
                        (log_json.to_string(), 0.5_f32)
                    })
                    .collect();
                
                info!(window_logs_count = window_logs.len(), "Time-window logs retrieved");
                
                // Merge semantic results + time-window results, deduplicate
                let mut seen = HashSet::new();
                let mut merged: Vec<(String, f32)> = Vec::new();
                
                // Add semantic results first (higher priority)
                for (log, score) in logs_with_scores {
                    if seen.insert(log.clone()) {
                        merged.push((log, score));
                    }
                }
                
                // Add time-window results
                for (log, score) in window_logs {
                    if seen.insert(log.clone()) {
                        merged.push((log, score));
                    }
                }
                
                info!(merged_count = merged.len(), "Merged logs for causal analysis");
                
                // For causal analysis, we want more logs (not just top 10)
                // Take up to 50 unique logs for richer causal context
                let reranked = state.reranker.rerank(&req.message, merged, 50);
                reranked.into_iter().map(|r| r.message).collect()
            } else {
                // No effect found, fall back to normal behavior
                info!("No ERROR timestamp found, using semantic results only");
                let mut seen = HashSet::new();
                let unique_logs: Vec<(String, f32)> = logs_with_scores
                    .into_iter()
                    .filter(|(msg, _)| seen.insert(msg.clone()))
                    .collect();
                let reranked = state.reranker.rerank(&req.message, unique_logs, max_context_logs);
                reranked.into_iter().map(|r| r.message).take(max_context_logs).collect()
            }
        } else {
            // Normal (non-causal) query - existing behavior
            let mut seen = HashSet::new();
            let unique_logs: Vec<(String, f32)> = logs_with_scores
                .into_iter()
                .filter(|(msg, _)| seen.insert(msg.clone()))
                .collect();

            let reranked = state.reranker.rerank(&req.message, unique_logs, max_context_logs);
            reranked.into_iter().map(|r| r.message).take(max_context_logs).collect()
        };
        
        final_logs
    };

    let context_logs = logs.len();
    let conversation_context = build_conversation_context(&history);

    let full_query = if conversation_context.is_empty() {
        req.message.clone()
    } else {
        format!(
            "Previous conversation:\n{}\n\nCurrent question: {}",
            conversation_context, req.message
        )
    };

    // Pass intent override for follow-up queries where conversation context breaks intent detection
    let intent_override = if is_causal_query {
        Some(RagQueryIntent::Causal)
    } else {
        None
    };
    
    info!(
        intent_override = ?intent_override,
        logs_count = logs.len(),
        "Calling RAG engine query_with_intent"
    );

    let rag_response = state
        .rag_engine
        .query_with_intent(&full_query, logs.clone(), intent_override)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?;
    
    info!(
        has_causal_chain = rag_response.causal_chain.is_some(),
        chain_len = rag_response.causal_chain.as_ref().map(|c| c.chain.len()).unwrap_or(0),
        "RAG response received"
    );

    let response_logs = logs.clone();

    {
        let mut sessions = state.sessions.write().unwrap();
        if let Some(session) = sessions.get_mut(&req.session_id) {
            session.history.push(ChatMessage {
                role: "user".to_string(),
                content: req.message.clone(),
            });
            session.history.push(ChatMessage {
                role: "assistant".to_string(),
                content: rag_response.answer.clone(),
            });
            session.last_logs = logs;
            session.last_query = req.message.clone();
            if session.history.len() > 20 {
                session.history.drain(0..2);
            }
        }
    }

    let elapsed = start.elapsed().as_millis();
    info!(
        turn = turn,
        sources = rag_response.sources_count,
        provider = %rag_response.provider,
        time_ms = elapsed,
        "CHAT complete"
    );

    Ok(Json(ChatApiResponse {
        answer: rag_response.answer,
        sources_count: rag_response.sources_count,
        response_time_ms: elapsed,
        provider: rag_response.provider,
        context_logs,
        conversation_turn: turn,
        source_logs: response_logs,
        causal_chain: rag_response.causal_chain.map(CausalChainResponse::from),
    }))
}

fn build_conversation_context(history: &[ChatMessage]) -> String {
    if history.is_empty() {
        return String::new();
    }
    let recent: Vec<&ChatMessage> = history.iter().rev().take(6).collect::<Vec<_>>().into_iter().rev().collect();
    recent
        .iter()
        .map(|msg| {
            let role = if msg.role == "user" { "User" } else { "AI" };
            format!("{}: {}", role, msg.content)
        })
        .collect::<Vec<_>>()
        .join("\n")
}

async fn classify_query_intent(rag_engine: &logai_rag::RagEngine, last_query: &str, new_query: &str) -> QueryIntent {
    if last_query.is_empty() {
        return QueryIntent::NewSearch;
    }

    let new_lower = new_query.to_lowercase();

    let new_topic_indicators = [
        "show me", "find", "list", "get", "what are", "search for",
        "auth", "database", "payment", "nginx", "api", "error", "warning",
        "timeout", "connection", "failure", "crash", "security",
        "last hour", "last 2", "last 30", "yesterday", "today",
    ];

    let followup_indicators = [
        "explain", "tell me more", "what caused", "why did", "how to fix",
        "first one", "second one", "third one", "this", "that", "it",
        "the error", "the issue", "more details", "elaborate", "expand",
    ];

    for indicator in new_topic_indicators {
        if new_lower.contains(indicator) && !last_query.to_lowercase().contains(indicator) {
            return QueryIntent::NewSearch;
        }
    }

    for indicator in followup_indicators {
        if new_lower.contains(indicator) {
            return QueryIntent::FollowUp;
        }
    }

    let prompt = format!(
        r#"Previous query: "{}"
New query: "{}"

Is the new query a FOLLOW_UP (asking about same topic/logs) or NEW_SEARCH (different topic)?
Answer with one word only: FOLLOW_UP or NEW_SEARCH"#,
        last_query, new_query
    );

    match rag_engine.classify(&prompt).await {
        Ok(response) if response.to_uppercase().contains("FOLLOW") => QueryIntent::FollowUp,
        _ => QueryIntent::NewSearch,
    }
}

pub async fn get_session(
    State(state): State<Arc<AppState>>,
    Query(params): Query<SessionQuery>,
) -> Result<Json<SessionInfo>, (StatusCode, Json<ApiError>)> {
    let sessions = state.sessions.read().unwrap();

    match sessions.get(&params.session_id) {
        Some(session) => Ok(Json(SessionInfo {
            session_id: params.session_id,
            turns: session.history.len() / 2,
            last_logs_count: session.last_logs.len(),
            age_seconds: session.created_at.elapsed().as_secs(),
        })),
        None => Err(ApiError::not_found("Session not found")),
    }
}

/// Find the timestamp of the most severe ERROR from search results
/// This will be used as the "effect" for causal chain analysis
fn find_effect_timestamp(logs_with_scores: &[(String, f32)]) -> Option<DateTime<Utc>> {
    // Parse logs and find the most recent ERROR/FATAL
    let mut best_timestamp: Option<DateTime<Utc>> = None;
    let mut best_severity: u8 = 0;
    
    for (log_json, _score) in logs_with_scores {
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(log_json) {
            let level = parsed.get("level")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_uppercase();
            
            let severity = match level.as_str() {
                "FATAL" | "CRITICAL" => 5,
                "ERROR" | "ERR" => 4,
                "WARN" | "WARNING" => 3,
                _ => 0,
            };
            
            // Only consider ERROR or higher
            if severity >= 4 {
                if let Some(ts_str) = parsed.get("timestamp").and_then(|v| v.as_str()) {
                    if let Ok(ts) = DateTime::parse_from_rfc3339(ts_str) {
                        let ts_utc = ts.with_timezone(&Utc);
                        // Pick the most severe, or if same severity, the most recent
                        if severity > best_severity || 
                           (severity == best_severity && best_timestamp.map_or(true, |best| ts_utc > best)) {
                            best_severity = severity;
                            best_timestamp = Some(ts_utc);
                        }
                    }
                }
            }
        }
    }
    
    best_timestamp
}
