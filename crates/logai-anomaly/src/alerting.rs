//! Alert engine with deduplication

use crate::config::Severity;
use crate::detection::Anomaly;
use chrono::{DateTime, Duration, Utc};
use std::collections::HashMap;
use uuid::Uuid;

// unique key to identify an alert (rule + service)
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct AlertKey {
    pub rule_name: String,
    pub service: String,
}

// Current state of an alert
#[derive(Debug, Clone, PartialEq)]
pub enum AlertState {
    Firing,       // alert is active needs attention
    Acknowledged, // user has seen it
    Resolved,     // Problem fixed
}

// an active alert being tracked
#[derive(Debug, Clone)]
pub struct ActiveAlert {
    pub id: Uuid,
    pub key: AlertKey,
    pub state: AlertState,
    pub severity: Severity,
    pub message: String,
    pub firing_at: DateTime<Utc>,
    pub last_notified_at: DateTime<Utc>,
    pub acknowledged_at: Option<DateTime<Utc>>,
}

// Main alert engine - manages all active alerts
pub struct AlertEngine {
    // Currently active alerts (key -> alert)
    active_alerts: HashMap<AlertKey, ActiveAlert>,

    // cooldown periods per rule
    cooldowns: HashMap<String, u64>,
}

impl AlertEngine {
    // create new alert engine
    pub fn new() -> Self {
        Self {
            active_alerts: HashMap::new(),
            cooldowns: HashMap::new(),
        }
    }

    // set cooldown for a rule called when loading config
    pub fn set_cooldown(&mut self, rule_name: &str, minutes: u64) {
        self.cooldowns.insert(rule_name.to_string(), minutes);
    }

    // process detected anomalies and return alerts that should be sent
    pub fn process_anomalies(&mut self, anomalies: Vec<Anomaly>) -> Vec<ActiveAlert> {
        let mut alerts_to_send = Vec::new();
        let now = Utc::now();

        for anomaly in anomalies {
            let key = AlertKey {
                rule_name: anomaly.rule_name.clone(),
                service: anomaly.service.clone(),
            };
            
            // Check if alert exists and if we should send
            let should_send = if let Some(existing) = self.active_alerts.get(&key) {
                self.should_alert(existing, &anomaly.rule_name, now)
            } else {
                true // New alert, always send
            };
            
            if should_send {
                if let Some(existing) = self.active_alerts.get_mut(&key) {
                    // Update existing alert
                    existing.last_notified_at = now;
                    existing.message = anomaly.message.clone();
                    alerts_to_send.push(existing.clone());
                } else {
                    // New alert - create and track it
                    let alert = ActiveAlert {
                        id: anomaly.id,
                        key: key.clone(),
                        state: AlertState::Firing,
                        severity: anomaly.severity,
                        message: anomaly.message.clone(),
                        firing_at: now,
                        last_notified_at: now,
                        acknowledged_at: None,
                    };
                    self.active_alerts.insert(key, alert.clone());
                    alerts_to_send.push(alert);
                }
            }
        }
        alerts_to_send
    }

    // check if we should send alert (cooldown check)
    fn should_alert(&self, alert: &ActiveAlert, rule_name: &str, now: DateTime<Utc>) -> bool {
        //if acknowledged, dont re-alert
        if alert.state == AlertState::Acknowledged {
            return false;
        }

        // get cooldown for this rule (defualt 5 minutes)
        let cooldown_minutes = self.cooldowns.get(rule_name).copied().unwrap_or(5);
        let cooldown = Duration::minutes(cooldown_minutes as i64);

        // check if enough time has passed since last notification
        now - alert.last_notified_at >= cooldown
    }

    // Acknowledge an alert (user clicked "acknowledge" in Slack)
    pub fn acknowledge(&mut self, key: &AlertKey) -> bool {
        if let Some(alert) = self.active_alerts.get_mut(key) {
            alert.state = AlertState::Acknowledged;
            alert.acknowledged_at = Some(Utc::now());
            true
        } else {
            false
        }
    }

    // Resolve and remove an alert
    pub fn resolve(&mut self, key: &AlertKey) -> Option<ActiveAlert> {
        self.active_alerts.remove(key)
    }

    // Get all active alerts
    pub fn get_active_alerts(&self) -> Vec<&ActiveAlert> {
        self.active_alerts.values().collect()
    }
}
