use axum::{
    extract::{Query, State},
    http::StatusCode,
    Json,
};
use std::sync::Arc;
use tracing::info;

use crate::models::{RecentLogRow, RecentLogsQuery, StatsResponse};
use crate::state::{AppState, COLLECTION_NAME};

pub async fn get_stats(
    State(state): State<Arc<AppState>>,
) -> Result<Json<StatsResponse>, (StatusCode, String)> {
    info!("Stats request");

    let total_logs: u64 = state.clickhouse
        .query("SELECT count(*) FROM logs")
        .fetch_one()
        .await
        .unwrap_or(0);

    let logs_24h: u64 = state.clickhouse
        .query("SELECT count(*) FROM logs WHERE timestamp > now() - INTERVAL 1 DAY")
        .fetch_one()
        .await
        .unwrap_or(0);

    let error_count: u64 = state.clickhouse
        .query("SELECT count(*) FROM logs WHERE level = 'Error'")
        .fetch_one()
        .await
        .unwrap_or(0);

    let services_count: u64 = state.clickhouse
        .query("SELECT count(DISTINCT service) FROM logs")
        .fetch_one()
        .await
        .unwrap_or(0);

    let embeddings_count = match state.qdrant.collection_info(COLLECTION_NAME).await {
        Ok(info) => info.result.map(|r| r.points_count.unwrap_or(0)).unwrap_or(0),
        Err(_) => 0,
    };

    let storage_mb = (total_logs as f64 * 0.5) / 1024.0;

    Ok(Json(StatsResponse {
        total_logs,
        logs_24h,
        error_count,
        services_count,
        embeddings_count,
        storage_mb,
    }))
}

pub async fn get_services(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<String>>, (StatusCode, String)> {
    info!("Services request");

    let services: Vec<String> = state.clickhouse
        .query("SELECT DISTINCT service FROM logs ORDER BY service")
        .fetch_all()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(services))
}

pub async fn get_recent_logs(
    State(state): State<Arc<AppState>>,
    Query(params): Query<RecentLogsQuery>,
) -> Result<Json<Vec<RecentLogRow>>, (StatusCode, String)> {
    let limit = params.limit.unwrap_or(100).min(500);
    info!(limit, service = ?params.service, level = ?params.level, "Recent logs request");

    let mut conditions = vec!["1=1".to_string()];
    if let Some(ref service) = params.service {
        conditions.push(format!("service = '{}'", service.replace('\'', "''")));
    }
    if let Some(ref level) = params.level {
        conditions.push(format!("level = '{}'", level.replace('\'', "''")));
    }

    let query = format!(
        "SELECT toString(id) as log_id, service, level, message, toString(timestamp) as timestamp 
         FROM logs 
         WHERE {} 
         ORDER BY timestamp DESC 
         LIMIT {}",
        conditions.join(" AND "),
        limit
    );

    let logs: Vec<RecentLogRow> = state.clickhouse
        .query(&query)
        .fetch_all()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(logs))
}
