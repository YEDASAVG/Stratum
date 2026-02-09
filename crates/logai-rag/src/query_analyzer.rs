// Query Analyzer

// Extracts time ranges and services names from natural language queries

use chrono::{DateTime, Duration, Utc};
use regex::Regex;

// analyzed query with extracted filters
#[derive(Debug, Clone)]
pub struct AnalyzedQuery {
    pub original: String,            // og user query
    pub search_query: String,        // cleaned query for semantic search
    pub from: Option<DateTime<Utc>>, // Start time (if detected)
    pub to: DateTime<Utc>,           // defaults to now
    pub service: Option<String>,     // service name if detected
}

pub struct QueryAnalyzer {
    time_patterns: Vec<(Regex, i64, &'static str)>,
    service_pattern: Regex,
}

impl QueryAnalyzer {
    pub fn new() -> Self {
        // time patterns "last x hrs/min/days"
        let time_patterns = vec![
            (
                Regex::new(r"last\s+(\d+)\s*h(?:our)?s?").unwrap(),
                3600,
                "seconds",
            ),
            (
                Regex::new(r"last\s+(\d+)\s*m(?:in(?:ute)?)?s?").unwrap(),
                60,
                "seconds",
            ),
            (
                Regex::new(r"last\s+(\d+)\s*d(?:ay)?s?").unwrap(),
                86400,
                "seconds",
            ),
            (
                Regex::new(r"past\s+(\d+)\s*h(?:our)?s?").unwrap(),
                3600,
                "seconds",
            ),
            (
                Regex::new(r"past\s+(\d+)\s*m(?:in(?:ute)?)?s?").unwrap(),
                60,
                "seconds",
            ),
        ];
        // service pattern: common service names
        let service_pattern = Regex::new(
            r"\b(nginx|apache|mysql|postgres|redis|kafka|docker|kubernetes|k8s|api|auth|gateway)\b",
        )
        .unwrap();

        Self {
            time_patterns,
            service_pattern,
        }
    }

    // analyze a natural language query
    pub fn analyze(&self, query: &str) -> AnalyzedQuery {
        let query_lower = query.to_lowercase();
        let now = Utc::now();

        // extract time range
        let from = self.extract_time_range(&query_lower, now);

        // extract service
        let service = self.extract_service(&query_lower);

        // Clean query for semantic search (remove time phrases)
        let search_query = self.clean_query(&query_lower);

        AnalyzedQuery {
            original: query.to_string(),
            search_query,
            from,
            to: now,
            service,
        }
    }

    fn extract_time_range(&self, query: &str, now: DateTime<Utc>) -> Option<DateTime<Utc>> {
        for (pattern, multiplier, _) in &self.time_patterns {
            if let Some(caps) = pattern.captures(query) {
                if let Some(num_match) = caps.get(1) {
                    if let Ok(num) = num_match.as_str().parse::<i64>() {
                        let seconds = num * multiplier;
                        return Some(now - Duration::seconds(seconds));
                    }
                }
            }
        }
        None
    }
    fn extract_service(&self, query: &str) -> Option<String> {
        self.service_pattern
        .find(query)
        .map(|m| m.as_str().to_string())
    }

    fn clean_query(&self, query: &str) -> String {
        let mut cleaned = query.to_string();

        //remove time phrases
        let remove_patterns = [
            r"last\s+\d+\s*(?:hours?|minutes?|days?|h|m|d)\s*",
            r"past\s+\d+\s*(?:hours?|minutes?|days?|h|m|d)\s*",
            r"in the last\s+\d+\s*(?:hours?|minutes?|days?)\s*",
        ];

        for pattern in remove_patterns {
            let re = Regex::new(pattern).unwrap();
            cleaned = re.replace_all(&cleaned, " ").to_string();
        }
        // clean white space
        cleaned.split_whitespace().collect::<Vec<_>>().join(" ")
    }
}

impl Default for QueryAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_time_extraction() {
        let analyzer = QueryAnalyzer::new();
        let result = analyzer.analyze("nginx errors last 2 hours");

        assert!(result.from.is_some());
        assert_eq!(result.service, Some("nginx".to_string()));
    }

    #[test]
    fn test_clean_query() {
        let analyzer = QueryAnalyzer::new();
        let result = analyzer.analyze("show me errors last 1 hour");

        assert_eq!(result.search_query, "show me errors");
    }
}