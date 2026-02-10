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
    pub to: Option<DateTime<Utc>>,   // End time (for ranges like "yesterday")
    pub service: Option<String>,     // service name if detected
    pub level: Option<String>,       // log level if detected (Error, Warn, etc.)
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
        let (from, to) = self.extract_time_range(&query_lower, now);

        // extract service
        let service = self.extract_service(&query_lower);

        // extract log level
        let level = self.extract_level(&query_lower);

        // Clean query for semantic search (remove time phrases)
        let search_query = self.clean_query(&query_lower);

        AnalyzedQuery {
            original: query.to_string(),
            search_query,
            from,
            to,
            service,
            level,
        }
    }

    fn extract_time_range(&self, query: &str, now: DateTime<Utc>) -> (Option<DateTime<Utc>>, Option<DateTime<Utc>>) {
        // Check for keyword-based time references first
        if query.contains("yesterday") {
            // For dev/testing: yesterday = last 24 hours to get meaningful results
            return (Some(now - Duration::hours(24)), None);
        }
        if query.contains("today") {
            return (Some(now - Duration::hours(12)), None);
        }
        if query.contains("this week") {
            return (Some(now - Duration::days(7)), None);
        }
        if query.contains("this month") {
            return (Some(now - Duration::days(30)), None);
        }

        // Check for pattern-based time references
        for (pattern, multiplier, _) in &self.time_patterns {
            if let Some(caps) = pattern.captures(query) {
                if let Some(num_match) = caps.get(1) {
                    if let Ok(num) = num_match.as_str().parse::<i64>() {
                        let seconds = num * multiplier;
                        return (Some(now - Duration::seconds(seconds)), None);
                    }
                }
            }
        }
        (None, None)
    }
    fn extract_service(&self, query: &str) -> Option<String> {
        self.service_pattern
        .find(query)
        .map(|m| m.as_str().to_string())
    }

    fn extract_level(&self, query: &str) -> Option<String> {
        // Map natural language to log levels
        if query.contains("error") || query.contains("errors") || query.contains("failure") || query.contains("failed") {
            Some("Error".to_string())
        } else if query.contains("warn") || query.contains("warning") || query.contains("warnings") {
            Some("Warn".to_string())
        } else if query.contains("debug") {
            Some("Debug".to_string())
        } else if query.contains("info") && !query.contains("information about") {
            Some("Info".to_string())
        } else if query.contains("anomal") || query.contains("problem") || query.contains("issue") 
            || query.contains("what happened") || query.contains("what went wrong") 
            || query.contains("incident") || query.contains("outage") || query.contains("down")
            || query.contains("critical") || query.contains("urgent") || query.contains("alert")
            || (query.contains("summar") && (query.contains("yesterday") || query.contains("today") || query.contains("last"))) {
            // For anomaly/incident queries, look for errors
            Some("Error".to_string())
        } else {
            None
        }
    }

    fn clean_query(&self, query: &str) -> String {
        let mut cleaned = query.to_string();

        // Remove time phrases
        let remove_patterns = [
            r"last\s+\d+\s*(?:hours?|minutes?|days?|h|m|d)\s*",
            r"past\s+\d+\s*(?:hours?|minutes?|days?|h|m|d)\s*",
            r"in the last\s+\d+\s*(?:hours?|minutes?|days?)\s*",
            r"\byesterday\b",
            r"\btoday\b",
            r"\bthis\s+week\b",
            r"\bthis\s+month\b",
        ];

        for pattern in remove_patterns {
            let re = Regex::new(pattern).unwrap();
            cleaned = re.replace_all(&cleaned, " ").to_string();
        }

        // Remove common filler phrases that hurt semantic search
        let filler_patterns = [
            r"^show\s+me\s+",
            r"^give\s+me\s+",
            r"^what\s+are\s+(?:the\s+)?",
            r"^what\s+is\s+(?:the\s+)?",
            r"^can\s+you\s+show\s+",
            r"^please\s+show\s+",
            r"^find\s+(?:me\s+)?",
            r"^get\s+(?:me\s+)?",
            r"^list\s+(?:all\s+)?",
            r"^display\s+",
            r"^tell\s+me\s+about\s+",
            r"^i\s+want\s+to\s+see\s+",
            r"\s+please$",
        ];

        for pattern in filler_patterns {
            let re = Regex::new(pattern).unwrap();
            cleaned = re.replace_all(&cleaned, "").to_string();
        }

        // Clean whitespace
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
    fn test_30_minutes_extraction() {
        let analyzer = QueryAnalyzer::new();
        let result = analyzer.analyze("errors in last 30 minutes");

        println!("Query: errors in last 30 minutes");
        println!("From: {:?}", result.from);
        println!("Search query: {}", result.search_query);
        
        assert!(result.from.is_some(), "Time should be extracted from 'last 30 minutes'");
    }

    #[test]
    fn test_clean_query() {
        let analyzer = QueryAnalyzer::new();
        let result = analyzer.analyze("show me errors last 1 hour");

        // "show me" is removed as filler, "last 1 hour" is removed as time phrase
        assert_eq!(result.search_query, "errors");
    }
}