//! Slack webhook integration

use crate::alerting::ActiveAlert;
use crate::config::Severity;
use reqwest::Client;
use serde::Serialize;

// Slack client for sending alerts
pub struct SlackClient {
    client: Client,
    webhook_url: String,
    enabled: bool,
}

// slack message payload
#[derive(Serialize)]
struct SlackMessage {
    text: String,
    attachments: Vec<SlackAttachment>,
}

// slack attachment (colored sidebar with details)
#[derive(Serialize)]
struct SlackAttachment {
    color: String,
    title: String,
    text: String,
    fields: Vec<SlackField>,
    footer: String,
    ts: i64,
}

// slack field (key value in attachment)
#[derive(Serialize)]
struct SlackField {
    title: String,
    value: String,
    short: bool,
}

impl SlackClient {
    // create a new Slack client
    pub fn new(webhook_url: String, enabled: bool) -> Self {
        Self {
            client: Client::new(),
            webhook_url,
            enabled,
        }
    }

    // send an alert to stack
    pub async fn send_alert(&self, alert: &ActiveAlert) -> Result<(), Box<dyn std::error::Error>> {
        //skip if disabled
        if !self.enabled {
            return Ok(());
        }
        // build the message
        let message = self.build_message(alert);

        // send to slack
        let response = self
            .client
            .post(&self.webhook_url)
            .json(&message)
            .send()
            .await?;

        // Check response
        if response.status().is_success() {
            Ok(())
        } else {
            let error_text = response.text().await.unwrap_or_default();
            Err(format!("Slack API error: {}", error_text).into())
        }
    }

    // Build Slack message form alert
    fn build_message(&self, alert: &ActiveAlert) -> SlackMessage {
        let emoji = match alert.severity {
            Severity::Critical => "🚨",
            Severity::Warning => "⚠️",
            Severity::Info => "ℹ️",
        };

        let color = self.severity_to_color(&alert.severity);
        SlackMessage {
            text: format!("{} Alert: {}", emoji, alert.key.rule_name),
            attachments: vec![SlackAttachment {
                color,
                title: alert.message.clone(),
                text: format!("Detected at {}", alert.firing_at.format("%Y-%m-%d %H:%M:%S UTC")),
                fields: vec![
                    SlackField{
                        title: "Service".to_string(),
                        value: alert.key.service.clone(),
                        short: true,
                    },
                    SlackField{
                        title: "Severity".to_string(),
                        value: format!("{:?}", alert.severity),
                        short: true,
                    },
                ],
                footer: "LogAI Anomaly Detection".to_string(),
                ts: alert.firing_at.timestamp(),
            }],
        }
    }

    // Convert severity to slack color
    fn severity_to_color(&self, severity: &Severity) -> String {
        match severity {
            Severity::Critical => "danger".to_string(),
            Severity::Warning => "warning".to_string(),
            Severity::Info => "good".to_string(),
        }
    }
}
