// Syslog log parser (RFC 3164 / BSD format)

use super::{LogParser, ParseError};
use crate::{LogLevel, RawLogEntry};
use chrono::{DateTime, NaiveDateTime, Utc};
use regex::Regex;
use std::collections::HashMap;

pub struct SyslogParser {
    // BSD syslog: <priority>Mon DD HH:MM:SS hostname process[pid]: message
    // Or without priority: Mon DD HH:MM:SS hostname process[pid]: message
    pattern: Regex,
    // Simple pattern for when hostname is missing
    simple_pattern: Regex,
}

impl SyslogParser {
    pub fn new() -> Self {
        Self {
            // Full BSD syslog pattern
            pattern: Regex::new(
                r"^(?:<(\d+)>)?(\w{3}\s+\d{1,2}\s+\d{2}:\d{2}:\d{2})\s+(\S+)\s+(\S+?)(?:\[(\d+)\])?:\s*(.+)$"
            ).unwrap(),
            // Simpler pattern without hostname
            simple_pattern: Regex::new(
                r"^(?:<(\d+)>)?(\w{3}\s+\d{1,2}\s+\d{2}:\d{2}:\d{2})\s+(\S+?)(?:\[(\d+)\])?:\s*(.+)$"
            ).unwrap(),
        }
    }

    fn parse_timestamp(ts: &str) -> Option<DateTime<Utc>> {
        // Format: Oct 11 22:14:15 (no year, assume current year)
        let current_year = Utc::now().format("%Y").to_string();
        let ts_with_year = format!("{} {}", ts, current_year);
        
        NaiveDateTime::parse_from_str(&ts_with_year, "%b %d %H:%M:%S %Y")
            .ok()
            .map(|naive| DateTime::<Utc>::from_naive_utc_and_offset(naive, Utc))
    }

    fn priority_to_level(priority: u8) -> LogLevel {
        // Syslog severity is priority % 8
        let severity = priority % 8;
        match severity {
            0 => LogLevel::Error,  // Emergency
            1 => LogLevel::Error,  // Alert
            2 => LogLevel::Error,  // Critical
            3 => LogLevel::Error,  // Error
            4 => LogLevel::Warn,   // Warning
            5 => LogLevel::Info,   // Notice
            6 => LogLevel::Info,   // Informational
            7 => LogLevel::Debug,  // Debug
            _ => LogLevel::Info,
        }
    }
}

impl LogParser for SyslogParser {
    fn name(&self) -> &'static str {
        "syslog"
    }

    fn parse(&self, raw: &str) -> Result<RawLogEntry, ParseError> {
        // Try full pattern with hostname
        if let Some(caps) = self.pattern.captures(raw) {
            let priority: Option<u8> = caps.get(1)
                .and_then(|m| m.as_str().parse().ok());
            let timestamp_str = caps.get(2).map(|m| m.as_str()).unwrap_or("");
            let hostname = caps.get(3).map(|m| m.as_str()).unwrap_or("");
            let process = caps.get(4).map(|m| m.as_str()).unwrap_or("");
            let pid = caps.get(5).map(|m| m.as_str());
            let message = caps.get(6).map(|m| m.as_str()).unwrap_or(raw);

            let mut fields = HashMap::new();
            fields.insert("hostname".to_string(), serde_json::json!(hostname));
            fields.insert("process".to_string(), serde_json::json!(process));
            if let Some(p) = pid {
                fields.insert("pid".to_string(), serde_json::json!(p));
            }
            if let Some(pri) = priority {
                fields.insert("priority".to_string(), serde_json::json!(pri));
                fields.insert("facility".to_string(), serde_json::json!(pri / 8));
            }

            let level = priority
                .map(Self::priority_to_level)
                .unwrap_or(LogLevel::Info);

            return Ok(RawLogEntry {
                message: message.to_string(),
                timestamp: Self::parse_timestamp(timestamp_str),
                service: Some(process.to_string()),
                level: Some(level),
                trace_id: None,
                fields,
            });
        }

        // Try simple pattern without hostname
        if let Some(caps) = self.simple_pattern.captures(raw) {
            let priority: Option<u8> = caps.get(1)
                .and_then(|m| m.as_str().parse().ok());
            let timestamp_str = caps.get(2).map(|m| m.as_str()).unwrap_or("");
            let process = caps.get(3).map(|m| m.as_str()).unwrap_or("");
            let pid = caps.get(4).map(|m| m.as_str());
            let message = caps.get(5).map(|m| m.as_str()).unwrap_or(raw);

            let mut fields = HashMap::new();
            fields.insert("process".to_string(), serde_json::json!(process));
            if let Some(p) = pid {
                fields.insert("pid".to_string(), serde_json::json!(p));
            }

            let level = priority
                .map(Self::priority_to_level)
                .unwrap_or(LogLevel::Info);

            return Ok(RawLogEntry {
                message: message.to_string(),
                timestamp: Self::parse_timestamp(timestamp_str),
                service: Some(process.to_string()),
                level: Some(level),
                trace_id: None,
                fields,
            });
        }

        // Fallback: treat as plain message
        Ok(RawLogEntry {
            message: raw.to_string(),
            timestamp: None,
            service: Some("syslog".to_string()),
            level: Some(LogLevel::Info),
            trace_id: None,
            fields: HashMap::new(),
        })
    }
}
