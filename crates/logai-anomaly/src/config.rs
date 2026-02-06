//! Configuration parsing for anomaly detection rules

use serde::Deserialize;
use std::fs;
use std::path::Path;


// Main config structure
#[derive(Debug, Deserialize)]
pub struct AnomalyConfig {
    // frequency of cheking anomalies (in secs)
    pub check_interval_seconds: u64,

    // Slack config
    pub slack: SlackConfig,

    // list of anomaly detection rules
    #[serde(default)]
    pub rules: Vec<Rule>,
}

// Slcak webhook config

#[derive(Debug, Deserialize)]
pub struct SlackConfig {
    // check slack notifications are enabled
    pub enabled: bool,

    // webhook url
    pub webhook_url: String,
}

// A single anomaly detection rule
#[derive(Debug, Deserialize)]
pub struct Rule {
    // unique name for this rule
    pub name: String,

    // whether this rule is active
    #[serde(default = "default_true")]
    pub enabled: bool,

    // Services to monitor (supports "*" for all, or "payment-*" patterns)
    pub services: Vec<String>,

    // Detection configuration
    pub detection: Detection,

    pub alert: AlertSettings,
}

// Detection type and parameters
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Detection {
    // Statistical detection using standard deviation
    Statistical {
        // which metric to monitor
        metric: Metric,
        sensitivity: Sensitivity,
        baseline_window_minutes: u64,
    },
    Threshold {
        metric: Metric,      // which metric to monitor
        operator: Operator,  // comparison operator
        value: f64,          // threshold value
        window_minutes: u64, // time window in minutes
    },
}

// Metrics that can be monitored

#[derive(Debug, Deserialize, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum Metric {
    ErrorCount, // count of errror-level logs
    ErrorRate,  // % of logs that are errors
    LogVolume,  // total log volume
}

#[derive(Debug, Deserialize, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum Sensitivity {
    Low,    // 3 standard deviation means fewer alerts
    Medium, // balanced
    High,   // More alerts
}

impl Sensitivity {
    // convert sensitivity to standard deviation multiplier
    pub fn to_sigma(&self) -> f64 {
        match self {
            Sensitivity::Low => 3.0,
            Sensitivity::Medium => 2.0,
            Sensitivity::High => 1.5,
        }
    }
}

// comparison operators for threshold detection
#[derive(Debug, Deserialize, Clone, Copy)]
pub enum Operator {
    #[serde(rename = ">")]
    GreaterThan,
    #[serde(rename = "<")]
    LessThan,
    #[serde(rename = ">=")]
    GreaterOrEqual,
    #[serde(rename = "<=")]
    LessOrEqual,
    #[serde(rename = "==")]
    Equal,
}

impl Operator {
    // evaluate comparison
    pub fn evaluate(&self, current: f64, threshold: f64) -> bool {
        match self {
            Operator::GreaterThan => current > threshold,
            Operator::LessThan => current < threshold,
            Operator::GreaterOrEqual => current >= threshold,
            Operator::LessOrEqual => current <= threshold,
            Operator::Equal => (current - threshold).abs() < f64::EPSILON,
        }
    }
}

// alerrt severity levels
#[derive(Debug, Deserialize, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Info,
    Warning,
    Critical,
}

// alert config for a rule
#[derive(Debug, Deserialize)] 
pub struct AlertSettings {
    // severity level of alerts from this rule
    pub severity: Severity,

    // cooldown period in minutes
    pub cooldown_minutes: u64,
}

// defualt value helper for serde
fn default_true() -> bool {
    true
}

// Load configuration from a TOML file

pub fn load_config<P: AsRef<Path>>(path: P) -> Result<AnomalyConfig, Box<dyn std::error::Error>> {
    let content = fs::read_to_string(path)?;
    let config: AnomalyConfig = toml::from_str(&content)?;
    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_config() {
        let toml_content = r#"
check_interval_seconds = 60

[slack]
enabled = false
webhook_url = ""

[[rules]]
name = "Error Spike"
enabled = true
services = ["*"]

[rules.detection]
type = "statistical"
metric = "error_count"
sensitivity = "medium"
baseline_window_minutes = 60

[rules.alert]
severity = "warning"
cooldown_minutes = 10
"#;
        let config: AnomalyConfig = toml::from_str(toml_content).unwrap();
        assert_eq!(config.check_interval_seconds, 60);
        assert_eq!(config.rules.len(), 1);
        assert_eq!(config.rules[0].name, "Error Spike");
    }
}
