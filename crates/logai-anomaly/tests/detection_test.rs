use clickhouse::Client;
use logai_anomaly::config::{load_config, Severity};
use logai_anomaly::detection::{Anomaly,AnomalyDetector};
use logai_anomaly::alerting::{AlertEngine, AlertKey};
use chrono::Utc;
use uuid::Uuid;
use logai_anomaly::slack::SlackClient;

#[tokio::test]
async fn test_detect_error_spike() {
    // Clickhouse connect
    let client = Client::default()
    .with_url("http://localhost:8123")
    .with_database("logai");

    // Detectoor banao
    let detector = AnomalyDetector::new(client);

    // load config file
    let config = load_config("../../config/anomaly-rules.toml")
    .expect("Failed to looad config");

    println!("\n=== Anomaly Detection Test ===");
    println!("Loaded {} rules\n", config.rules.len());

    // Debug: Print all rules and their services
    for (i, rule) in config.rules.iter().enumerate() {
        println!("Rule {}: {} | services: {:?} | enabled: {}", 
            i, rule.name, rule.services, rule.enabled);
    }
    println!();

    // check every rule

    for rule in &config.rules {
        println!("Checking Rule: {} (services: {:?})", rule.name, rule.services);

        match detector.check_rule(rule).await {
            Ok(anomalies) => {
                if anomalies.is_empty() {
                    println!("  ✓ No anomalies\n");
                } else {
                    println!("  ⚠ Found {} anomalies:", anomalies.len());
                    for a in &anomalies {
                        println!("    Services: {}", a.service);
                        println!("    Message: {}", a.message);
                        println!("    Current: {:1}, Expected: {:.1}\n", a.current_value, a.expected_value);
                    }
                }
            }
            Err(e) => {
                println!("  ✗ Error: {}\n", e);
            }
        }
    }
}

#[tokio::test]
async fn test_alert_engine() {
    // create allert engine
    let mut engine = AlertEngine::new();

    // set cooldown for 5 minutes
    engine.set_cooldown("Error Spike", 5);

    // Fake anomaly
    let anomaly = Anomaly{
        id: Uuid::new_v4(),
        rule_name: "Error Spike".to_string(),
        service: "payment-api".to_string(),
        severity: Severity::Critical,
        message: "Test error".to_string(),
        current_value: 50.0,
        expected_value: 10.0,
        detected_at: Utc::now(),
    };

    // process
    let alerts = engine.process_anomalies(vec![anomaly.clone()]);
    println!("First time: {} alerts", alerts.len());

    let alert2 = engine.process_anomalies(vec![anomaly]);
    println!("Second Time: {} alerts (should be 0)", alert2.len());
}

#[tokio::test]
async fn test_slack_client() {
    dotenv::dotenv().ok();
    let webhook_url = std::env::var("SLACK_WEBHOOK_URL")
        .expect("SLACK_WEBHOOK_URL must be set for this test");

    let client = SlackClient::new(webhook_url, true);

    let alert = logai_anomaly::alerting::ActiveAlert {
        id: Uuid::new_v4(),
        key: AlertKey{
            rule_name: "Error Spike".to_string(),
            service: "payment-api".to_string(),
        },
        state: logai_anomaly::alerting::AlertState::Firing,
        severity: Severity::Critical,
        message: "Error count spike: 50 errors in 5 minutes".to_string(),
        firing_at: Utc::now(),
        last_notified_at: Utc::now(),
        acknowledged_at: None,
    };
    match client.send_alert(&alert).await {
        Ok(_) => println!("✅ Alert sent to Slack!"),
        Err(e) => println!("❌ Failed: {}", e),
    }
}