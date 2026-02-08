//! Core types for log intelligence system
//! this crate contains shared data strcture used acrosss all components.
pub mod parser;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// LOG LEVEL //

/// Log severity levels (ordered from lowest to highest)

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]

pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
    Fatal,
}

impl LogLevel {
    /// Parse log level from string (case-insensitive)
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "trace" => Some(Self::Trace),
            "debug" => Some(Self::Debug),
            "info" => Some(Self::Info),
            "warn" | "warning" => Some(Self::Warn),
            "error" | "err" => Some(Self::Error),
            "fatal" | "critical" | "crit" => Some(Self::Fatal),
            _ => None,
        }
    }
}

// RAW LOG ENTRY (what API receives)

/// Raw log entry as received from HTTP API
/// This is theunprocessed input from applications

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawLogEntry {
    pub message: String,

    #[serde(default)]
    pub timestamp: Option<DateTime<Utc>>,

    #[serde(default)]
    pub service: Option<String>,

    #[serde(default)]
    pub level: Option<LogLevel>,

    #[serde(default)]
    pub trace_id: Option<String>,

    #[serde(default)]
    pub fields: std::collections::HashMap<String, serde_json::Value>,
}

// parsed Log entry (after processing)

// fully parsed and enriched log entry
// this is stored in ClickHouse and used fooor analysis

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub id: Uuid, // unique identifier

    pub timestamp: DateTime<Utc>, // when the loog was generated

    pub level: LogLevel, // log severity levl

    pub service: String, //Service/application name

    pub message: String, // the log message

    pub raw: String, // original raw log line

    #[serde(default)]
    pub trace_id: Option<String>, // traceid for distributed tracing (if available)

    #[serde(default)]
    pub span_id: Option<String>, // Span ID (if available)

    #[serde(default)]
    pub error_category: Option<ErrorCategory>, // Error categoory by parser

    #[serde(default)]
    pub fields: std::collections::HashMap<String, serde_json::Value>, // additional metadata

    pub ingested_at: DateTime<Utc>,
}
impl LogEntry {
    //create a log entry from rawlogentry
    // this parses and enriches the raw input
    pub fn from_raw(raw: RawLogEntry) -> Self {
        let now = Utc::now();
        
        let raw_json = serde_json::to_string(&raw).unwrap_or_else(|_| raw.message.clone());
        Self {
            id: Uuid::new_v4(),
            timestamp: raw.timestamp.unwrap_or(now),
            level: raw.level.unwrap_or(LogLevel::Info),
            service: raw.service.unwrap_or_else(|| "unknown".to_string()),
            message: raw.message.clone(),
            raw: raw_json,
            trace_id: raw.trace_id,
            span_id: None,
            error_category: None,
            fields: raw.fields,
            ingested_at: now,
        }
    }
}

// Error Categories

// categorized error types for better anaylsis
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]

pub enum ErrorCategory {
    OutOfMemory,     // out of memory heap exahuastion
    Timeout,         // connection, request timeout
    ConnectionError, // connection refused, network unreachable
    HttpError,       // 4xx/5xx HTTP errors
    DatabaseError,   // authentication/authorization errors
    AuthError,       // file not found, permission denied
    Unknown,
}

// LOG chunk (for embeddings/ vector storage)

// A chunk of logs gruped tgether for embeddings explained in .md planfile
// Multiple LogEntries are gruped int chunks fooor semantic search

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogChunk {
    pub id: Uuid,                  //unique chunk identifier
    pub log_ids: Vec<Uuid>,        // IDs f logs in this chunk
    pub start_time: DateTime<Utc>, // Time range: start
    pub end_time: DateTime<Utc>,   // Time range: end
    pub service: String,           // Primary service in this chunk
    pub summary: String,           // Summary text for embedding

    #[serde(default)]
    pub embedding: Option<Vec<f32>>, // the mebedding vector (384 dimensioons for fastemebed)
    pub log_count: usize, // Number of logs in chunk
    pub max_level: LogLevel,

    #[serde(default)]
    pub relevance_score: Option<f32>, // For RRF/reranking later
}
