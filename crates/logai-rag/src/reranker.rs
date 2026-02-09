// Reranker - Improve retrival quality

// combines semantic score with keyword overlap foor better ranking
// Reranks loogs based on query relevance

pub struct Reranker;

#[derive(Debug, Clone)]
pub struct RankedLog{
    pub message: String,
    pub semantic_score: f32,
    pub keyword_score: f32,
    pub final_score: f32,
}

impl Reranker {
    pub fn new() -> Self {
        Self
    }

    // Rerank logs by combining semantic score with keyword overlap most imp
    pub fn rerank(
        &self,
        query: &str,
        logs: Vec<(String, f32)>, // message, semantic-score
        top_k: usize,
    ) -> Vec<RankedLog>{
        let query_lower = query.to_lowercase();
        let query_words: Vec<&str> = query_lower
            .split_whitespace()
            .collect();
        let mut ranked: Vec<RankedLog> = logs
        .into_iter()
        .map(|(message, semantic_score)| {
            let keyword_score = self.compute_keyword_score(&query_words, &message);

            // weightd combination 70% semantic + 30% keyword
            let final_score = (semantic_score * 0.7) + (keyword_score * 0.3);

            RankedLog{
                message,
                semantic_score,
                keyword_score,
                final_score,
            }
        })
        .collect();
    // sort by final score descending
    ranked.sort_by(|a, b| b.final_score.partial_cmp(&a.final_score).unwrap());

    // return top_k
    ranked.into_iter().take(top_k).collect()
    }

    fn compute_keyword_score(&self, query_words: &[&str], log: &str) -> f32 {
        let log_lower = log.to_lowercase();

        let mut weighted_matches = 0.0;

        for word in query_words {
            if log_lower.contains(word) {
                // Boost important keywords
                let weight = match *word {
                    "error" | "fail" | "failed" | "exception" => 2.0,
                    "warn" | "warning" | "timeout" => 1.5,
                    "critical" | "fatal" | "crash" => 2.5,
                    _ => 1.0,
                };
                weighted_matches += weight;
            }
        }
        if query_words.is_empty(){
            0.0
        } else {
            // normalize to 0-1 range
            (weighted_matches / (query_words.len() as f32 * 2.5)).min(1.0)
        }
    }
}

impl Default for Reranker {
    fn default() -> Self {
        Self::new()
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reranking() {
        let reranker = Reranker::new();
        
        let logs = vec![
            ("GET /health 200 OK".to_string(), 0.8),
            ("ERROR: Payment failed timeout".to_string(), 0.6),
            ("User logged in".to_string(), 0.7),
        ];

        let result = reranker.rerank("payment error", logs, 2);
        
        // "Payment failed" should be first despite lower semantic score
        assert!(result[0].message.contains("Payment"));
        assert_eq!(result.len(), 2);
    }
}