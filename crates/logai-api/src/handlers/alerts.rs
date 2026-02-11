use axum::{
    extract::{Query, State},
    http::StatusCode,
    Json,
};
use std::sync::Arc;
use tracing::info;

use crate::models::{AlertItem, AlertsQuery, AlertsResponse, AnomaliesQuery, AnomaliesResponse, AnomalyItem};
use crate::state::AppState;

pub async fn get_alerts(
    State(state): State<Arc<AppState>>,
    Query(params): Query<AlertsQuery>,
) -> Result<Json<AlertsResponse>, (StatusCode, String)> {
    info!(status = ?params.status, "Alerts request");

    let query = match &params.status {
        Some(status) if status == "firing" => {
            "SELECT service, level, message, timestamp 
             FROM logs 
             WHERE level = 'Error' 
             AND timestamp > now() - INTERVAL 1 HOUR
             ORDER BY timestamp DESC
             LIMIT 20"
        }
        _ => {
            "SELECT service, level, message, timestamp 
             FROM logs 
             WHERE level = 'Error' 
             AND timestamp > now() - INTERVAL 24 HOUR
             ORDER BY timestamp DESC
             LIMIT 50"
        }
    };

    let rows: Vec<(String, String, String, i64)> = state.clickhouse
        .query(query)
        .fetch_all()
        .await
        .unwrap_or_default();

    let alerts: Vec<AlertItem> = rows
        .into_iter()
        .enumerate()
        .map(|(i, (service, level, message, ts))| {
            let severity = if message.to_lowercase().contains("critical") || message.to_lowercase().contains("fatal") {
                "critical"
            } else if level == "Error" {
                "warning"
            } else {
                "info"
            };

            AlertItem {
                id: format!("alert-{}", i),
                service,
                severity: severity.to_string(),
                message: if message.len() > 100 { format!("{}...", &message[..97]) } else { message },
                status: "firing".to_string(),
                fired_at: chrono::DateTime::from_timestamp(ts / 1000, 0)
                    .map(|dt| dt.format("%H:%M:%S").to_string())
                    .unwrap_or_default(),
            }
        })
        .collect();

    info!(count = alerts.len(), "Alerts returned");

    Ok(Json(AlertsResponse { alerts }))
}

pub async fn get_anomalies(
    State(state): State<Arc<AppState>>,
    Query(params): Query<AnomaliesQuery>,
) -> Result<Json<AnomaliesResponse>, (StatusCode, String)> {
    info!(service = ?params.service, "Anomalies request");

    let mut anomalies = Vec::new();
    let now = chrono::Utc::now();

    let services_query = match &params.service {
        Some(s) => format!("SELECT DISTINCT service FROM logs WHERE service = '{}' LIMIT 20", s),
        None => "SELECT DISTINCT service FROM logs LIMIT 20".to_string(),
    };

    let services: Vec<String> = state.clickhouse
        .query(&services_query)
        .fetch_all()
        .await
        .unwrap_or_default();

    for service in services {
        let current_errors: u64 = state.clickhouse
            .query(&format!(
                "SELECT count(*) FROM logs WHERE service = '{}' AND level = 'Error' AND timestamp > now() - INTERVAL 5 MINUTE",
                service
            ))
            .fetch_one()
            .await
            .unwrap_or(0);

        let baseline_errors: f64 = state.clickhouse
            .query(&format!(
                "SELECT avg(error_count) FROM (
                    SELECT count(*) as error_count 
                    FROM logs 
                    WHERE service = '{}' AND level = 'Error' 
                    AND timestamp > now() - INTERVAL 1 HOUR
                    GROUP BY toStartOfFiveMinutes(timestamp)
                )",
                service
            ))
            .fetch_one()
            .await
            .unwrap_or(0.0);

        if baseline_errors > 0.0 && (current_errors as f64) > baseline_errors * 2.0 {
            let severity = if (current_errors as f64) > baseline_errors * 5.0 {
                "critical"
            } else {
                "warning"
            };

            anomalies.push(AnomalyItem {
                service: service.clone(),
                rule: "Error Spike".to_string(),
                severity: severity.to_string(),
                message: format!(
                    "Error count spike: {} errors in last 5 min (baseline: {:.1})",
                    current_errors, baseline_errors
                ),
                current_value: current_errors as f64,
                expected_value: baseline_errors,
            });
        }

        let current_volume: u64 = state.clickhouse
            .query(&format!(
                "SELECT count(*) FROM logs WHERE service = '{}' AND timestamp > now() - INTERVAL 5 MINUTE",
                service
            ))
            .fetch_one()
            .await
            .unwrap_or(0);

        let baseline_volume: f64 = state.clickhouse
            .query(&format!(
                "SELECT avg(log_count) FROM (
                    SELECT count(*) as log_count 
                    FROM logs 
                    WHERE service = '{}' 
                    AND timestamp > now() - INTERVAL 1 HOUR
                    GROUP BY toStartOfFiveMinutes(timestamp)
                )",
                service
            ))
            .fetch_one()
            .await
            .unwrap_or(0.0);

        if baseline_volume > 10.0 && (current_volume as f64) < baseline_volume * 0.1 {
            anomalies.push(AnomalyItem {
                service: service.clone(),
                rule: "Volume Drop".to_string(),
                severity: "warning".to_string(),
                message: format!(
                    "Log volume dropped: {} logs in last 5 min (baseline: {:.1})",
                    current_volume, baseline_volume
                ),
                current_value: current_volume as f64,
                expected_value: baseline_volume,
            });
        }
    }

    info!(count = anomalies.len(), "Anomalies detected");

    Ok(Json(AnomaliesResponse {
        anomalies,
        checked_at: now.format("%Y-%m-%dT%H:%M:%SZ").to_string(),
    }))
}
