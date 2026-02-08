// Apache log parser

use super::{LogParser, ParseError};
use crate::{LogLevel, RawLogEntry};
use regex::Regex;
use std::collections::HashMap;
use chrono::{DateTime, NaiveDateTime, Utc};
pub struct ApacheParser {
    // apache error log pattern
    error_pattern: Regex,
}

impl ApacheParser {
    pub fn new() -> Self {
        Self {
            error_pattern: Regex::new(r"^\[([^\]]+)\] \[(\w+)\] (.+)$").unwrap(),
        }
    }
}

impl LogParser for ApacheParser {
    fn name(&self) -> &'static str {
        "apache"
    }

    fn parse(&self, raw: &str) -> Result<RawLogEntry, ParseError> {
        // try to match apache error log format
        if let Some(caps) = self.error_pattern.captures(raw) {
            let timestamp_str = caps.get(1).map(|m| m.as_str()).unwrap_or("");
            let level_str = caps.get(2).map(|m| m.as_str()).unwrap_or("info");
            let message = caps.get(3).map(|m| m.as_str()).unwrap_or(raw);

            let level = LogLevel::from_str(level_str);
            let timestamp = NaiveDateTime::parse_from_str(timestamp_str, "%a %b %d %H:%M:%S %Y")
            .ok()
            .map(|naive| DateTime::<Utc>::from_naive_utc_and_offset(naive, Utc));

            Ok(RawLogEntry {
                message: message.to_string(),
                timestamp: timestamp, 
                service: Some("apache".to_string()),
                level,
                trace_id: None,
                fields: HashMap::new(),
            })
        } else {
            // fallback treat as plain message
            Ok(RawLogEntry {
                message: raw.to_string(),
                timestamp: None,
                service: Some("apache".to_string()),
                level: Some(LogLevel::Info),
                trace_id: None,
                fields: HashMap::new(),
            })
        }
    }
}
