// Nginx log parser

use super::{LogParser, ParseError};
use crate::{LogLevel, RawLogEntry};
use chrono::{DateTime, NaiveDateTime, Utc};
use regex::Regex;
use std::collections::HashMap;

pub struct NginxParser {
    error_pattern: Regex,  // Nginx error log: 2024/02/08 10:30:00 [error] 12345#0: ...
    access_pattern: Regex, // Nginx access log (combined): IP - - [timestamp] "method path" status size
}

impl NginxParser {
    pub fn new() -> Self {
        Self {
            // Error log pattern
            error_pattern: Regex::new(
                r"^(\d{4}/\d{2}/\d{2} \d{2}:\d{2}:\d{2}) \[(\w+)\] (\d+)#\d+: (.+)$"
            ).unwrap(),
            // Access log pattern (combined format)
            access_pattern: Regex::new(
                r#"^(\S+) \S+ \S+ \[([^\]]+)\] "(\S+) ([^"]*)" (\d+) (\d+)"#
            ).unwrap(),
        }
    }

    fn parse_error_timestamp(ts: &str) -> Option<DateTime<Utc>> {
        NaiveDateTime::parse_from_str(ts, "%Y/%m/%d %H:%M:%S")
            .ok()
            .map(|naive| DateTime::<Utc>::from_naive_utc_and_offset(naive, Utc))
    }

    fn parse_access_timestamp(ts: &str) -> Option<DateTime<Utc>> {
        // Format: 10/Oct/2000:13:55:36 -0700
        // Simplified: parse without timezone
        NaiveDateTime::parse_from_str(
            ts.split_whitespace().next().unwrap_or(ts),
            "%d/%b/%Y:%H:%M:%S"
        )
        .ok()
        .map(|naive| DateTime::<Utc>::from_naive_utc_and_offset(naive, Utc))
    }

    fn status_to_level(status: u16) -> LogLevel {
        match status {
            200..=299 => LogLevel::Info,
            300..=399 => LogLevel::Info,
            400..=499 => LogLevel::Warn,
            500..=599 => LogLevel::Error,
            _ => LogLevel::Info,
        }
    }
}

impl LogParser for NginxParser {
    fn name(&self) -> &'static str {
        "nginx"
    }

    fn parse(&self, raw: &str) -> Result<RawLogEntry, ParseError> {
        // Try error log format first
        if let Some(caps) = self.error_pattern.captures(raw) {
            let timestamp_str = caps.get(1).map(|m| m.as_str()).unwrap_or("");
            let level_str = caps.get(2).map(|m| m.as_str()).unwrap_or("info");
            let pid = caps.get(3).map(|m| m.as_str()).unwrap_or("");
            let message = caps.get(4).map(|m| m.as_str()).unwrap_or(raw);

            let mut fields = HashMap::new();
            fields.insert("pid".to_string(), serde_json::json!(pid));

            return Ok(RawLogEntry {
                message: message.to_string(),
                timestamp: Self::parse_error_timestamp(timestamp_str),
                service: Some("nginx".to_string()),
                level: Some(LogLevel::from_str(level_str).unwrap_or(LogLevel::Info)),
                trace_id: None,
                fields,
            });
        }

        // Try access log format
        if let Some(caps) = self.access_pattern.captures(raw) {
            let ip = caps.get(1).map(|m| m.as_str()).unwrap_or("");
            let timestamp_str = caps.get(2).map(|m| m.as_str()).unwrap_or("");
            let method = caps.get(3).map(|m| m.as_str()).unwrap_or("");
            let path = caps.get(4).map(|m| m.as_str()).unwrap_or("");
            let status: u16 = caps.get(5)
                .and_then(|m| m.as_str().parse().ok())
                .unwrap_or(200);
            let size = caps.get(6).map(|m| m.as_str()).unwrap_or("0");

            let mut fields = HashMap::new();
            fields.insert("ip".to_string(), serde_json::json!(ip));
            fields.insert("method".to_string(), serde_json::json!(method));
            fields.insert("path".to_string(), serde_json::json!(path));
            fields.insert("status".to_string(), serde_json::json!(status));
            fields.insert("size".to_string(), serde_json::json!(size));

            let message = format!("{} {} {} {}", method, path, status, size);

            return Ok(RawLogEntry {
                message,
                timestamp: Self::parse_access_timestamp(timestamp_str),
                service: Some("nginx".to_string()),
                level: Some(Self::status_to_level(status)),
                trace_id: None,
                fields,
            });
        }

        // Fallback: treat as plain message
        Ok(RawLogEntry {
            message: raw.to_string(),
            timestamp: None,
            service: Some("nginx".to_string()),
            level: Some(LogLevel::Info),
            trace_id: None,
            fields: HashMap::new(),
        })
    }
}
