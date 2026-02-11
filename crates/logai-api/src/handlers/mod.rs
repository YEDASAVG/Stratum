mod ingest;
mod search;
mod chat;
mod stats;
mod alerts;

pub use ingest::*;
pub use search::*;
pub use chat::*;
pub use stats::*;
pub use alerts::*;

use std::collections::HashMap;

pub fn get_string(
    payload: &HashMap<String, qdrant_client::qdrant::Value>,
    key: &str,
) -> String {
    payload
        .get(key)
        .and_then(|v| v.as_str().map(|s| s.to_string()))
        .unwrap_or_default()
}
