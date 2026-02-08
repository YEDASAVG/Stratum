use crate::alerting::AlertEngine;
use crate::config::{AnomalyConfig, load_config};
use crate::detection::AnomalyDetector;
use crate::slack::SlackClient;
use clickhouse::Client;
use std::path::Path;
use std::time::Duration;
use tokio::time::interval;

// main runnder that orchestrates anomaly detection

pub struct AnomalyRunner {
    config: AnomalyConfig,
    detector: AnomalyDetector,
    alert_engine: AlertEngine,
    slack_client: SlackClient,
}

impl AnomalyRunner {
    // create a new runner from config file
    pub fn new<P: AsRef<Path>>(
        config_path: P,
        clickhouse_url: &str,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        //load coonfig
        let config = load_config(config_path)?;

        // Create Clickhouse Client
        let clickhouse = Client::default().with_url(clickhouse_url);

        // create components
        let detector = AnomalyDetector::new(clickhouse);
        let mut alert_engine = AlertEngine::new();

        // Set coooldowns from config
        for rule in &config.rules {
            alert_engine.set_cooldown(&rule.name, rule.alert.cooldown_minutes);
        }

        // create Slack Client
        let slack_client = SlackClient::new(config.slack.webhook_url.clone(), config.slack.enabled);

        Ok(Self {
            config,
            detector,
            alert_engine,
            slack_client,
        })
    }

    pub async fn run(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let check_interval = Duration::from_secs(self.config.check_interval_seconds);
        let mut ticker = interval(check_interval);

        println!("Starting anomaly detection loop (interval: {}s",
        self.config.check_interval_seconds);

        loop {
            ticker.tick().await;

            // check each rule
            for rule in &self.config.rules {
                match self.detector.check_rule(rule).await {
                    Ok(anomalies) => {
                        if !anomalies.is_empty() {
                            println!("Detected {} anomalies for rule '{}", anomalies.len(), rule.name);

                            // process through alerts engine deduplication
                            let alerts = self.alert_engine.process_anomalies(anomalies);

                            // send to Slack
                            for alert in alerts {
                                if let Err(e) = self.slack_client.send_alert(&alert).await {
                                    eprintln!("Failed to send Slack alert: {}", e);
                                }
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Error checking rule '{}': {}", rule.name, e);
                    }
                }
            }
        }
    }
}
