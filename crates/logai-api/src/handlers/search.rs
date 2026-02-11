use axum::{
    extract::{Query, State},
    http::StatusCode,
    Json,
};
use qdrant_client::qdrant::{Condition, Filter, Range, SearchPointsBuilder};
use std::sync::Arc;
use std::time::Instant;
use tracing::info;

use crate::handlers::get_string;
use crate::models::{AskQuery, AskResponse, CausalChainResponse, QueryAnalysisResponse, SearchQuery, SearchResult};
use crate::state::{AppState, COLLECTION_NAME};

pub async fn search_logs(
    State(state): State<Arc<AppState>>,
    Query(params): Query<SearchQuery>,
) -> Result<Json<Vec<SearchResult>>, (StatusCode, String)> {
    info!(query = %params.q, limit = params.limit, "Search request");

    let query_vector = {
        let mut model = state.model.lock().unwrap();
        let embeddings = model
            .embed(vec![params.q.clone()], None)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        embeddings.into_iter().next().ok_or((
            StatusCode::INTERNAL_SERVER_ERROR,
            "No embedding".to_string(),
        ))?
    };

    let mut conditions = vec![];

    if let Some(from) = params.from {
        conditions.push(Condition::range(
            "timestamp_unix",
            Range {
                gte: Some(from as f64),
                ..Default::default()
            },
        ));
    }
    if let Some(to) = params.to {
        conditions.push(Condition::range(
            "timestamp_unix",
            Range {
                lte: Some(to as f64),
                ..Default::default()
            },
        ));
    }
    if let Some(ref service) = params.service {
        conditions.push(Condition::matches("service", service.clone()));
    }

    let filter = if conditions.is_empty() {
        None
    } else {
        Some(Filter::must(conditions))
    };

    let mut search_builder =
        SearchPointsBuilder::new(COLLECTION_NAME, query_vector, params.limit).with_payload(true);

    if let Some(f) = filter {
        search_builder = search_builder.filter(f);
    }

    let results = state
        .qdrant
        .search_points(search_builder)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let search_results: Vec<SearchResult> = results
        .result
        .into_iter()
        .map(|point| {
            let payload = point.payload;
            SearchResult {
                score: point.score,
                log_id: get_string(&payload, "log_id"),
                service: get_string(&payload, "service"),
                level: get_string(&payload, "level"),
                message: get_string(&payload, "message"),
                timestamp: get_string(&payload, "timestamp"),
            }
        })
        .collect();

    info!(results = search_results.len(), "Search Complete");
    Ok(Json(search_results))
}

pub async fn ask_logs(
    State(state): State<Arc<AppState>>,
    Query(params): Query<AskQuery>,
) -> Result<Json<AskResponse>, (StatusCode, String)> {
    let start = Instant::now();
    info!(query = %params.q, "ASK request");

    let analyzed = state.rag_engine.analyze_query(&params.q);

    let query_vector = {
        let mut model = state.model.lock().unwrap();
        let embeddings = model
            .embed(vec![analyzed.search_query.clone()], None)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        embeddings.into_iter().next().ok_or((
            StatusCode::INTERNAL_SERVER_ERROR,
            "No embedding".to_string(),
        ))?
    };

    let mut conditions = vec![];
    if let Some(from) = analyzed.from {
        conditions.push(Condition::range(
            "timestamp_unix",
            Range {
                gte: Some(from.timestamp() as f64),
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
        SearchPointsBuilder::new(COLLECTION_NAME, query_vector, 30).with_payload(true);
    if let Some(f) = filter {
        search_builder = search_builder.filter(f);
    }

    let results = state
        .qdrant
        .search_points(search_builder)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

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

    info!(logs_found = logs_with_scores.len(), "Logs retrieved from Qdrant");

    if logs_with_scores.is_empty() {
        return Err((StatusCode::NOT_FOUND, "No relevant logs found".to_string()));
    }

    let reranked = state.reranker.rerank(&params.q, logs_with_scores, 10);
    let logs: Vec<String> = reranked.into_iter().map(|r| r.message).collect();

    info!(reranked_count = logs.len(), "Logs reranked");

    let rag_response = state
        .rag_engine
        .query(&params.q, logs)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let elapsed = start.elapsed().as_millis();
    info!(sources = rag_response.sources_count, provider = %rag_response.provider, time_ms = elapsed, "ASK complete");

    Ok(Json(AskResponse {
        answer: rag_response.answer,
        sources_count: rag_response.sources_count,
        response_time_ms: elapsed,
        provider: rag_response.provider,
        query_analysis: QueryAnalysisResponse {
            search_query: rag_response.query_analysis.search_query,
            time_filter: rag_response.query_analysis.time_filter,
            service_filter: rag_response.query_analysis.service_filter,
        },
        causal_chain: rag_response.causal_chain.map(CausalChainResponse::from),
    }))
}
