// Proxmox VE log parser (pve-proxy, pveproxy, pvedaemon, etc.)
// Supports both BSD syslog format and systemd journal ISO8601 format

use super::{LogParser, ParseError};
use crate::{LogLevel, RawLogEntry};
use chrono::{DateTime, NaiveDateTime, Utc};
use regex::Regex;
use std::collections::HashMap;

pub struct ProxmoxParser {
    // ISO8601 format: 2024-02-23T10:23:45.123456+00:00 hostname process[pid]: message
    iso_pattern: Regex,
    // BSD syslog format: Feb 23 10:23:45 hostname process[pid]: message
    bsd_pattern: Regex,
    // Simple format: Feb 23 10:23:45 process[pid]: message (no hostname)
    simple_pattern: Regex,
}

impl ProxmoxParser {
    pub fn new() -> Self {
        Self {
            // ISO8601 timestamp pattern (systemd journal)
            iso_pattern: Regex::new(
                r"^(\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d+)?(?:Z|[+-]\d{2}:?\d{2}))\s+(\S+)\s+([^\[\s]+)(?:\[(\d+)\])?:\s*(.+)$"
            ).unwrap(),
            // BSD syslog pattern with hostname
            bsd_pattern: Regex::new(
                r"^(\w{3}\s+\d{1,2}\s+\d{2}:\d{2}:\d{2})\s+(\S+)\s+([^\[\s]+)(?:\[(\d+)\])?:\s*(.+)$"
            ).unwrap(),
            // Simple pattern without hostname
            simple_pattern: Regex::new(
                r"^(\w{3}\s+\d{1,2}\s+\d{2}:\d{2}:\d{2})\s+([^\[\s]+)(?:\[(\d+)\])?:\s*(.+)$"
            ).unwrap(),
        }
    }

    fn parse_iso_timestamp(ts: &str) -> Option<DateTime<Utc>> {
        DateTime::parse_from_rfc3339(ts)
            .ok()
            .map(|dt| dt.with_timezone(&Utc))
            .or_else(|| {
                // Try without timezone
                NaiveDateTime::parse_from_str(ts, "%Y-%m-%dT%H:%M:%S%.f")
                    .ok()
                    .map(|naive| DateTime::<Utc>::from_naive_utc_and_offset(naive, Utc))
            })
    }

    fn parse_bsd_timestamp(ts: &str) -> Option<DateTime<Utc>> {
        let current_year = Utc::now().format("%Y").to_string();
        let ts_with_year = format!("{} {}", ts, current_year);
        
        NaiveDateTime::parse_from_str(&ts_with_year, "%b %d %H:%M:%S %Y")
            .ok()
            .map(|naive| DateTime::<Utc>::from_naive_utc_and_offset(naive, Utc))
    }

    fn detect_level(message: &str) -> LogLevel {
        let msg_lower = message.to_lowercase();
        if msg_lower.contains("error") || msg_lower.contains("failed") || msg_lower.contains("fatal") {
            LogLevel::Error
        } else if msg_lower.contains("warn") || msg_lower.contains("warning") {
            LogLevel::Warn
        } else if msg_lower.contains("debug") {
            LogLevel::Debug
        } else {
            LogLevel::Info
        }
    }
}

impl LogParser for ProxmoxParser {
    fn name(&self) -> &'static str {
        "proxmox"
    }

    fn parse(&self, raw: &str) -> Result<RawLogEntry, ParseError> {
        // Try ISO8601 format first (systemd journal)
        if let Some(caps) = self.iso_pattern.captures(raw) {
            let timestamp_str = caps.get(1).map(|m| m.as_str()).unwrap_or("");
            let hostname = caps.get(2).map(|m| m.as_str()).unwrap_or("");
            let process = caps.get(3).map(|m| m.as_str()).unwrap_or("");
            let pid = caps.get(4).map(|m| m.as_str());
            let message = caps.get(5).map(|m| m.as_str()).unwrap_or(raw);

            let mut fields = HashMap::new();
            fields.insert("hostname".to_string(), serde_json::json!(hostname));
            fields.insert("process".to_string(), serde_json::json!(process));
            if let Some(p) = pid {
                fields.insert("pid".to_string(), serde_json::json!(p));
            }

            return Ok(RawLogEntry {
                message: message.to_string(),
                timestamp: Self::parse_iso_timestamp(timestamp_str),
                service: Some(process.to_string()),
                level: Some(Self::detect_level(message)),
                trace_id: None,
                fields,
            });
        }

        // Try BSD syslog format with hostname
        if let Some(caps) = self.bsd_pattern.captures(raw) {
            let timestamp_str = caps.get(1).map(|m| m.as_str()).unwrap_or("");
            let hostname = caps.get(2).map(|m| m.as_str()).unwrap_or("");
            let process = caps.get(3).map(|m| m.as_str()).unwrap_or("");
            let pid = caps.get(4).map(|m| m.as_str());
            let message = caps.get(5).map(|m| m.as_str()).unwrap_or(raw);

            let mut fields = HashMap::new();
            fields.insert("hostname".to_string(), serde_json::json!(hostname));
            fields.insert("process".to_string(), serde_json::json!(process));
            if let Some(p) = pid {
                fields.insert("pid".to_string(), serde_json::json!(p));
            }

            return Ok(RawLogEntry {
                message: message.to_string(),
                timestamp: Self::parse_bsd_timestamp(timestamp_str),
                service: Some(process.to_string()),
                level: Some(Self::detect_level(message)),
                trace_id: None,
                fields,
            });
        }

        // Try simple pattern without hostname
        if let Some(caps) = self.simple_pattern.captures(raw) {
            let timestamp_str = caps.get(1).map(|m| m.as_str()).unwrap_or("");
            let process = caps.get(2).map(|m| m.as_str()).unwrap_or("");
            let pid = caps.get(3).map(|m| m.as_str());
            let message = caps.get(4).map(|m| m.as_str()).unwrap_or(raw);

            let mut fields = HashMap::new();
            fields.insert("process".to_string(), serde_json::json!(process));
            if let Some(p) = pid {
                fields.insert("pid".to_string(), serde_json::json!(p));
            }

            return Ok(RawLogEntry {
                message: message.to_string(),
                timestamp: Self::parse_bsd_timestamp(timestamp_str),
                service: Some(process.to_string()),
                level: Some(Self::detect_level(message)),
                trace_id: None,
                fields,
            });
        }

        // Fallback: treat entire line as message
        Ok(RawLogEntry {
            message: raw.to_string(),
            timestamp: None,
            service: Some("proxmox".to_string()),
            level: Some(Self::detect_level(raw)),
            trace_id: None,
            fields: HashMap::new(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bsd_format() {
        let parser = ProxmoxParser::new();
        let line = "Feb 23 10:23:45 pve pveproxy[12345]: starting server";
        let result = parser.parse(line).unwrap();
        assert_eq!(result.service, Some("pveproxy".to_string()));
        assert!(result.message.contains("starting server"));
        assert_eq!(result.fields.get("pid"), Some(&serde_json::json!("12345")));
        assert_eq!(result.fields.get("hostname"), Some(&serde_json::json!("pve")));
    }

    #[test]
    fn test_iso_format() {
        let parser = ProxmoxParser::new();
        let line = "2024-02-23T10:23:45.123+00:00 pve pveproxy[12345]: starting server";
        let result = parser.parse(line).unwrap();
        assert_eq!(result.service, Some("pveproxy".to_string()));
        assert!(result.timestamp.is_some());
    }

    #[test]
    fn test_pve_daemon() {
        let parser = ProxmoxParser::new();
        let line = "Feb 23 10:23:45 pve pvedaemon[1234]: test message";
        let result = parser.parse(line).unwrap();
        assert_eq!(result.service, Some("pvedaemon".to_string()));
    }

    #[test]
    fn test_error_detection() {
        let parser = ProxmoxParser::new();
        let line = "Feb 23 10:24:15 pve pveproxy[12345]: error: connection timeout";
        let result = parser.parse(line).unwrap();
        assert_eq!(result.level, Some(LogLevel::Error));
    }

    #[test]
    fn test_warning_detection() {
        let parser = ProxmoxParser::new();
        let line = "Feb 23 10:25:00 pve pveceph[5678]: warning: disk usage high";
        let result = parser.parse(line).unwrap();
        assert_eq!(result.level, Some(LogLevel::Warn));
    }

    #[test]
    fn test_spiceproxy() {
        let parser = ProxmoxParser::new();
        let line = "Feb 23 10:26:30 proxmox spiceproxy[9999]: client connected";
        let result = parser.parse(line).unwrap();
        assert_eq!(result.service, Some("spiceproxy".to_string()));
        assert_eq!(result.fields.get("hostname"), Some(&serde_json::json!("proxmox")));
    }

    #[test]
    fn test_fallback_plain_text() {
        let parser = ProxmoxParser::new();
        let line = "Some random log line without timestamp";
        let result = parser.parse(line).unwrap();
        assert_eq!(result.message, "Some random log line without timestamp");
        assert_eq!(result.service, Some("proxmox".to_string()));
    }
}
