// Query Analyzer - extracts time, service, level, and intent from natural language queries

use chrono::{DateTime, Duration, Utc};
use regex::Regex;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum QueryIntent {
    Search,   // "show me errors" → normal semantic search + RAG
    Causal,   // "why did X crash" → backward causal chain analysis  
    Summary,  // "summarize yesterday" → aggregate overview
    Trace,    // "trace request abc123" → distributed trace view
}

impl Default for QueryIntent {
    fn default() -> Self {
        QueryIntent::Search
    }
}

#[derive(Debug, Clone)]
pub struct AnalyzedQuery {
    pub original: String,
    pub search_query: String,
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
    pub service: Option<String>,
    pub level: Option<String>,
    pub intent: QueryIntent,
}

pub struct QueryAnalyzer {
    time_patterns: Vec<(Regex, i64, &'static str)>,
    service_pattern: Regex,
}

impl QueryAnalyzer {
    pub fn new() -> Self {
        let time_patterns = vec![
            (Regex::new(r"last\s+(\d+)\s*h(?:our)?s?").unwrap(), 3600, "seconds"),
            (Regex::new(r"last\s+(\d+)\s*m(?:in(?:ute)?)?s?").unwrap(), 60, "seconds"),
            (Regex::new(r"last\s+(\d+)\s*d(?:ay)?s?").unwrap(), 86400, "seconds"),
            (Regex::new(r"past\s+(\d+)\s*h(?:our)?s?").unwrap(), 3600, "seconds"),
            (Regex::new(r"past\s+(\d+)\s*m(?:in(?:ute)?)?s?").unwrap(), 60, "seconds"),
        ];
        let service_pattern = Regex::new(
            r"\b(nginx|apache|mysql|postgres|redis|kafka|docker|kubernetes|k8s|api|auth|gateway|payment|order|user|checkout)\b",
        ).unwrap();

        Self { time_patterns, service_pattern }
    }

    pub fn analyze(&self, query: &str) -> AnalyzedQuery {
        let query_lower = query.to_lowercase();
        let now = Utc::now();

        let (from, to) = self.extract_time_range(&query_lower, now);
        let service = self.extract_service(&query_lower);
        let level = self.extract_level(&query_lower);
        let search_query = self.clean_query(&query_lower);
        let intent = self.detect_intent(&query_lower);

        AnalyzedQuery {
            original: query.to_string(),
            search_query,
            from,
            to,
            service,
            level,
            intent,
        }
    }

    fn detect_intent(&self, query: &str) -> QueryIntent {
        // Causal: WHY questions - needs backward chain analysis
        if query.starts_with("why") 
            || query.contains("what caused")
            || query.contains("root cause")
            || query.contains("reason for")
            || query.contains("what led to")
            || query.contains("explain the crash")
            || query.contains("what happened before")
        {
            return QueryIntent::Causal;
        }
        
        // Trace: distributed tracing
        if query.contains("trace") || query.contains("request id") || query.contains("trace-id") {
            return QueryIntent::Trace;
        }
        
        // Summary: aggregate view
        if query.starts_with("summarize") || query.starts_with("summary") || query.contains("overview") {
            return QueryIntent::Summary;
        }
        
        QueryIntent::Search
    }

    fn extract_time_range(&self, query: &str, now: DateTime<Utc>) -> (Option<DateTime<Utc>>, Option<DateTime<Utc>>) {
        if query.contains("yesterday") {
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
        if query.contains("last hour") || query.contains("past hour") || query.contains("the hour") {
            return (Some(now - Duration::hours(1)), None);
        }
        if query.contains("last minute") || query.contains("past minute") {
            return (Some(now - Duration::minutes(1)), None);
        }
        if query.contains("last day") || query.contains("past day") {
            return (Some(now - Duration::days(1)), None);
        }
        if query.contains("recent") {
            return (Some(now - Duration::minutes(30)), None);
        }

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
        self.service_pattern.find(query).map(|m| m.as_str().to_string())
    }

    fn extract_level(&self, query: &str) -> Option<String> {
        if query.contains("error") || query.contains("errors") || query.contains("failure") || query.contains("failed") || query.contains("crash") {
            Some("Error".to_string())
        } else if query.contains("warn") || query.contains("warning") {
            Some("Warn".to_string())
        } else if query.contains("debug") {
            Some("Debug".to_string())
        } else if query.contains("info") && !query.contains("information about") {
            Some("Info".to_string())
        } else if query.contains("anomal") || query.contains("problem") || query.contains("issue") 
            || query.contains("what happened") || query.contains("incident") || query.contains("outage") {
            Some("Error".to_string())
        } else {
            None
        }
    }

    fn clean_query(&self, query: &str) -> String {
        let mut cleaned = query.to_string();

        let remove_patterns = [
            r"last\s+\d+\s*(?:hours?|minutes?|days?|h|m|d)\s*",
            r"past\s+\d+\s*(?:hours?|minutes?|days?|h|m|d)\s*",
            r"in the last\s+\d+\s*(?:hours?|minutes?|days?)\s*",
            r"\byesterday\b", r"\btoday\b", r"\bthis\s+week\b", r"\bthis\s+month\b",
        ];

        for pattern in remove_patterns {
            let re = Regex::new(pattern).unwrap();
            cleaned = re.replace_all(&cleaned, " ").to_string();
        }

        let filler_patterns = [
            r"^show\s+me\s+", r"^give\s+me\s+", r"^what\s+are\s+(?:the\s+)?",
            r"^what\s+is\s+(?:the\s+)?", r"^can\s+you\s+show\s+", r"^please\s+show\s+",
            r"^find\s+(?:me\s+)?", r"^get\s+(?:me\s+)?", r"^list\s+(?:all\s+)?",
            r"^display\s+", r"^tell\s+me\s+about\s+", r"^i\s+want\s+to\s+see\s+", r"\s+please$",
        ];

        for pattern in filler_patterns {
            let re = Regex::new(pattern).unwrap();
            cleaned = re.replace_all(&cleaned, "").to_string();
        }

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
    fn test_intent_detection() {
        let analyzer = QueryAnalyzer::new();
        
        let result = analyzer.analyze("why did payment crash at 3am");
        assert_eq!(result.intent, QueryIntent::Causal);
        assert_eq!(result.service, Some("payment".to_string()));
        
        let result = analyzer.analyze("show me errors last hour");
        assert_eq!(result.intent, QueryIntent::Search);
        
        let result = analyzer.analyze("what caused the outage");
        assert_eq!(result.intent, QueryIntent::Causal);
        
        let result = analyzer.analyze("summarize yesterday");
        assert_eq!(result.intent, QueryIntent::Summary);
    }

    #[test]
    fn test_clean_query() {
        let analyzer = QueryAnalyzer::new();
        let result = analyzer.analyze("show me errors last 1 hour");
        assert_eq!(result.search_query, "errors");
    }
}