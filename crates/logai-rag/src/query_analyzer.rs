// Query Analyzer - extracts time, service, level, and intent from natural language queries

use chrono::{DateTime, Duration, Utc};
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::jev_client::JevClient;

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

/// Which path produced the classification.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Classifier {
    Rules,
    Jev,
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
    /// Per-field confidence. `None` on the rules path. Tracked separately
    /// because a filter and a branch are different decisions.
    pub intent_confidence: Option<f32>,
    pub service_confidence: Option<f32>,
    pub level_confidence: Option<f32>,
    pub classified_by: Classifier,
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
            intent_confidence: None,
            service_confidence: None,
            level_confidence: None,
            classified_by: Classifier::Rules,
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

/// Jev-backed classification. Time parsing stays in chrono: date arithmetic
/// is a documented weakness of the model.
impl QueryAnalyzer {
    /// Filters are gated higher than intent: a wrong filter silently hides
    /// logs, a wrong branch is visible to the user.
    const INTENT_FLOOR: f32 = 0.50;
    const FILTER_FLOOR: f32 = 0.60;

    /// Choice accepts 255 options; leave room for the escape hatch.
    const MAX_SERVICE_OPTIONS: usize = 254;

    /// `known_services` is the real service list. Empty slice leaves service
    /// extraction to the regex. Falls back to the rules path on any failure.
    pub async fn analyze_with_jev(
        &self,
        query: &str,
        jev: &JevClient,
        known_services: &[String],
    ) -> AnalyzedQuery {
        // Rules first: gives us the time range, cleaned query, and a fallback.
        let mut analyzed = self.analyze(query);

        let mut questions = serde_json::Map::new();

        questions.insert("intent".to_string(), json!({
            "type": "choice",
            "instructions": "What kind of answer is the user asking for in `query`?",
            "criteria": {
                "causal":  "Asks why something happened, or what caused or led to it",
                "trace":   "Asks to follow one specific request, trace id or correlation id end to end",
                "summary": "Asks for an overview or aggregate of a period, rather than specific lines",
                "search":  "Asks to find or list log lines matching some description",
                "unclear": "The query does not say enough to tell which of the above it wants"
            }
        }));

        questions.insert("level".to_string(), json!({
            "type": "choice",
            "instructions": "Which log severity is the user asking about in `query`? \
                             Choose `any` unless the query clearly restricts severity.",
            "criteria": {
                "Error": "Asks about errors, failures, crashes, outages or incidents",
                "Warn":  "Asks about warnings specifically",
                "Info":  "Asks about informational messages specifically",
                "Debug": "Asks about debug output specifically",
                "any":   "Does not restrict severity, or asks about everything"
            }
        }));

        let services: Vec<&String> =
            known_services.iter().take(Self::MAX_SERVICE_OPTIONS).collect();
        if !services.is_empty() {
            let mut criteria = serde_json::Map::new();
            for name in &services {
                criteria.insert(
                    (*name).clone(),
                    json!(format!("The query is about the `{name}` service")),
                );
            }
            criteria.insert(
                "none".to_string(),
                json!("The query does not name or clearly imply any one of these services"),
            );
            // Guard. The Choice must return one of its options, so a query
            // naming a non-service still picks the nearest one.
            questions.insert("service_named".to_string(), json!({
                "type": "noul",
                "instructions": "Does `query` name or clearly refer to one of the \
                                 services listed in `known_services`?",
                "criteria": {
                    "true": "Names one of those services, or an unmistakable short form of one",
                    "false": "Names no service, or names a feature, page or concept \
                              that is not one of those services"
                }
            }));

            questions.insert("service".to_string(), json!({
                "type": "choice",
                "instructions": "Which service is `query` asking about? Match on meaning, \
                                 not exact spelling: a query about \"payments\" refers to a \
                                 service named `payments-api`.",
                "criteria": criteria
            }));
        }

        let state = json!({
            "query": query,
            "known_services": services,
        });

        let response = match jev.system_one(&state, &Value::Object(questions)).await {
            Ok(r) => r,
            Err(e) => {
                tracing::warn!(error = %e, "Jev classification failed, using rules path");
                return analyzed;
            }
        };

        tracing::debug!(
            model = %response.model,
            input_tokens = response.usage.input_tokens,
            "Jev classified query"
        );

        if let Ok((choice, confidence)) = response.choice("intent") {
            if confidence >= Self::INTENT_FLOOR {
                let intent = match choice {
                    "causal" => Some(QueryIntent::Causal),
                    "trace" => Some(QueryIntent::Trace),
                    "summary" => Some(QueryIntent::Summary),
                    "search" => Some(QueryIntent::Search),
                    // "unclear" falls through to the rules answer.
                    _ => None,
                };
                if let Some(intent) = intent {
                    analyzed.intent = intent;
                    analyzed.intent_confidence = Some(confidence);
                    analyzed.classified_by = Classifier::Jev;
                }
            }
        }

        if let Ok((choice, confidence)) = response.choice("level") {
            if confidence >= Self::FILTER_FLOOR {
                analyzed.level = match choice {
                    "any" => None,
                    other => Some(other.to_string()),
                };
                analyzed.level_confidence = Some(confidence);
                analyzed.classified_by = Classifier::Jev;
            }
        }

        if let Ok((choice, confidence)) = response.choice("service") {
            analyzed.service_confidence = Some(confidence);

            // Two conditions: which service, and whether it is about a service
            // at all. Below either bar, drop the filter - a wrongly filtered
            // search is indistinguishable from having no matching logs.
            let names_a_service = response
                .noul("service_named")
                .map(|p| p >= 0.5)
                .unwrap_or(true);

            analyzed.service = if names_a_service
                && confidence >= Self::FILTER_FLOOR
                && choice != "none"
            {
                Some(choice.to_string())
            } else {
                None
            };
            analyzed.classified_by = Classifier::Jev;
        }

        analyzed
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