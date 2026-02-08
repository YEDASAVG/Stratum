//! Statistical anomaly detection logic

use crate::config::{Detection, Metric, Rule, Severity};
use chrono::{DateTime, Utc};
use clickhouse::Client;
use uuid::Uuid;

// represnts a detected anomaly
#[derive(Debug, Clone)]
pub struct Anomaly {
    pub id: Uuid,                   //unique id
    pub rule_name: String,          // which rule triggered
    pub service: String,            // which service
    pub severity: Severity,         // from rule config
    pub message: String,            // Human readable description
    pub current_value: f64,         // Actual value found
    pub expected_value: f64,        // what was expected like (avg or threshold)
    pub detected_at: DateTime<Utc>, // when detected
}

// mian anomaly detector
pub struct AnomalyDetector {
    clickhouse: Client,
}

impl AnomalyDetector {
    pub fn new(clickhouse: Client) -> Self {
        Self { clickhouse }
    }

    //check a single rule and return any detected anomalies
    pub async fn check_rule(
        &self,
        rule: &Rule,
    ) -> Result<Vec<Anomaly>, Box<dyn std::error::Error>> {
        // skip if disabled
        if !rule.enabled {
            return Ok(vec![]);
        }

        let mut anomalies = Vec::new();

        //get list of services from ClickHouese

        let services = self.get_services(&rule.services).await?;

        for service in services {
            //check based on detection type
            let anomaly = match &rule.detection {
                Detection::Statistical {
                    metric,
                    sensitivity,
                    baseline_window_minutes,
                } => {
                    self.check_statistical(
                        rule,
                        &service,
                        *metric,
                        *sensitivity,
                        *baseline_window_minutes,
                    )
                    .await?
                }
                Detection::Threshold {
                    metric,
                    operator,
                    value,
                    window_minutes,
                } => {
                    self.check_threshold(
                        rule,
                        &service,
                        *metric,
                        *operator,
                        *value,
                        *window_minutes,
                    )
                    .await?
                }
            };
            if let Some(a) = anomaly {
                anomalies.push(a);
            }
        }

        Ok(anomalies)
    }

    // Get list of services matching the patterns
    async fn get_services(
        &self,
        patterns: &[String],
    ) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        //if pattern is * then get all services otherwise filter by pattern

        let has_wildcard = patterns.iter().any(|p| p == "*");

        if has_wildcard {
            // Get all unique services from logs
            let query = "SELECT DISTINCT service FROM logs";
            let services: Vec<String> = self.clickhouse.query(query).fetch_all::<String>().await?;
            Ok(services)
        } else {
            // Return the specific services listed
            Ok(patterns.to_vec())
        }
    }

    /// Statistical detection: check if current value exceeds baseline + (sigma + stddev)
    async fn check_statistical(
        &self,
        rule: &Rule,
        service: &str,
        metric: Metric,
        sensitivity: crate::config::Sensitivity,
        baseline_windows_minutes: u64,
    ) -> Result<Option<Anomaly>, Box<dyn std::error::Error>> {
        // Get current value (last 5 minutes)
        let current = self.get_metric(service, metric, 5).await?;

        // get baseline (avg and stddev)
        let (avg, stddev) = self
            .get_baseline(service, metric, baseline_windows_minutes)
            .await?;

        // calculate threshold
        let sigma = sensitivity.to_sigma();
        let threshold = avg + (sigma * stddev);

        // check if anomaly
        let is_anomaly = if stddev > 0.0 {
            current > threshold
        } else {
            current > 15.0
        };

        if is_anomaly {
            let message = format!(
                "{} spike detected: current={:.1}, expected={:.1} (threshold={:.1})",
                metric_name(metric),
                current,
                avg,
                threshold
            );

            Ok(Some(Anomaly {
                id: Uuid::new_v4(),
                rule_name: rule.name.clone(),
                service: service.to_string(),
                severity: rule.alert.severity,
                message,
                current_value: current,
                expected_value: avg,
                detected_at: Utc::now(),
            }))
        } else {
            Ok(None)
        }
    }

    // Threshold detection check if current value matches operators condition

    async fn check_threshold(
        &self,
        rule: &Rule,
        service: &str,
        metric: Metric,
        operator: crate::config::Operator,
        value: f64,
        window_minutes: u64,
    ) -> Result<Option<Anomaly>, Box<dyn std::error::Error>> {
        // get current value
        let current = self.get_metric(service, metric, window_minutes).await?;

        // check using operator
        if operator.evaluate(current, value) {
            let message = format!(
                "{} threshold breached: current={:.1} {} {:.1}",
                metric_name(metric),
                current,
                operator_symbol(&operator),
                value,
            );
            Ok(Some(Anomaly {
                id: Uuid::new_v4(),
                rule_name: rule.name.clone(),
                service: service.to_string(),
                severity: rule.alert.severity,
                message,
                current_value: current,
                expected_value: value,
                detected_at: Utc::now(),
            }))
        } else {
            Ok(None)
        }
    }

    // get metric value from clickhouese
    async fn get_metric(
        &self,
        service: &str,
        metric: Metric,
        minutes: u64,
    ) -> Result<f64, Box<dyn std::error::Error>> {
        let query = match metric {
            Metric::ErrorCount => {
                format!(
                    "SELECT toFloat64(count(*)) FROM logs WHERE service = '{}' AND level = 'Error' AND timestamp > now() - INTERVAL {} MINUTE",
                    service, minutes
                )
            }
            Metric::ErrorRate => {
                format!(
                    "SELECT countIf(level = 'Error') * 100.0 / count(*) FROM logs WHERE service = '{}' AND timestamp > now() - INTERVAL {} MINUTE",
                    service, minutes
                )
            }
            Metric::LogVolume => {
                format!(
                    "SELECT toFloat64(count(*)) FROM logs WHERE service = '{}' AND timestamp > now() - INTERVAL {} MINUTE",
                    service, minutes
                )
            }
        };
        
        let result: f64 = self
            .clickhouse
            .query(&query)
            .fetch_one()
            .await
            .unwrap_or(0.0);
        
        Ok(result)
    }

    // Get baseline avg and std deviation from clikchouse
    async fn get_baseline(
        &self,
        service: &str,
        metric: Metric,
        minutes: u64,
    ) -> Result<(f64, f64), Box<dyn std::error::Error>> {
        let inner_select = match metric {
            Metric::ErrorCount => "countIf(level = 'Error') as val",
            Metric::ErrorRate => "countIf(level = 'Error') * 100.0 / count(*) as val",
            Metric::LogVolume => "count(*) as val",
        };
        let query = format!(
            "SELECT avg(val) as avg_val, stddevPop(val) as stddev_val FROM (
                SELECT 
                    toStartOfMinute(timestamp) as minute,
                    {}
                FROM logs
                WHERE service = '{}'
                AND timestamp > now() - INTERVAL {} MINUTE
                GROUP BY minute
            )",
            inner_select, service, minutes
        );
        // this return two values: avg and stddev
        let result: (f64, f64) = self
            .clickhouse
            .query(&query)
            .fetch_one()
            .await
            .unwrap_or((0.0, 0.0));

        Ok(result)
    }
}

// Helper get human readable metric name

fn metric_name(metric: Metric) -> &'static str {
    match metric {
        Metric::ErrorCount => "Error count",
        Metric::ErrorRate => "Error rate",
        Metric::LogVolume => "Log volume",
    }
}

// Helper get operator symbol for message
fn operator_symbol(op: &crate::config::Operator) -> &'static str {
    use crate::config::Operator;
    match op {
        Operator::GreaterThan => ">",
        Operator::LessThan => "<",
        Operator::GreaterOrEqual => ">=",
        Operator::LessOrEqual => "<=",
        Operator::Equal => "==",
    }
}
