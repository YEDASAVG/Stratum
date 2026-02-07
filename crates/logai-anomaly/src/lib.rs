//! LogAI Anomaly Detection & Alerting

pub mod config;
pub mod detection;
pub mod alerting;
pub mod slack;
pub mod runner;

pub use config::AnomalyConfig;
pub use detection::AnomalyDetector;
pub use alerting::AlertEngine;
pub use slack::SlackClient;
pub use runner::AnomalyRunner;