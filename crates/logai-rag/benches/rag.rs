//! Benchmarks for RAG operations
//! Run: cargo bench -p logai-rag --bench rag

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use std::hint::black_box;

// Note: These benchmarks test query analysis and reranking without network calls
// For full RAG benchmarks with LLM, use the stress test tool

// Query samples for benchmarking
const QUERIES: &[&str] = &[
    "show me errors in nginx last 2 hours",
    "why did the payment service crash at 3am yesterday",
    "what caused the database timeout this week",
    "summarize all authentication failures today",
    "trace request abc-123-xyz through the system",
    "find connection refused errors in redis",
    "list all warnings from api-gateway past 30 minutes",
    "what happened before the OOM kill at 2:30am",
    "show me kafka consumer lag alerts",
    "why are users getting 502 errors on checkout",
];

fn bench_query_analysis(c: &mut Criterion) {
    // Import at runtime to avoid compilation issues
    use std::process::Command;
    
    // Since logai-rag has complex dependencies, we'll benchmark query patterns
    // For actual QueryAnalyzer benchmarks, run via the API
    
    let mut group = c.benchmark_group("query_analysis");
    
    // Benchmark regex patterns (simulating query analysis)
    let time_patterns = [
        regex::Regex::new(r"last\s+(\d+)\s*h(?:our)?s?").unwrap(),
        regex::Regex::new(r"last\s+(\d+)\s*m(?:in(?:ute)?)?s?").unwrap(),
        regex::Regex::new(r"last\s+(\d+)\s*d(?:ay)?s?").unwrap(),
    ];
    
    let service_pattern = regex::Regex::new(
        r"\b(nginx|apache|mysql|postgres|redis|kafka|api|auth|gateway|payment|order)\b"
    ).unwrap();
    
    for (i, query) in QUERIES.iter().enumerate() {
        group.bench_with_input(
            BenchmarkId::new("time_extraction", i),
            query,
            |b, query| {
                b.iter(|| {
                    for pattern in &time_patterns {
                        pattern.captures(black_box(*query));
                    }
                })
            },
        );
        
        group.bench_with_input(
            BenchmarkId::new("service_extraction", i),
            query,
            |b, query| {
                b.iter(|| service_pattern.find(black_box(*query)))
            },
        );
    }
    
    group.finish();
}

fn bench_reranking_simulation(c: &mut Criterion) {
    // Simulate reranking logic without logai-rag dependency
    
    fn compute_keyword_score(query_words: &[&str], log: &str) -> f32 {
        let log_lower = log.to_lowercase();
        let mut weighted_matches = 0.0;
        
        for word in query_words {
            if log_lower.contains(word) {
                let weight = match *word {
                    "error" | "fail" | "failed" | "exception" => 2.0,
                    "warn" | "warning" | "timeout" => 1.5,
                    "critical" | "fatal" | "crash" => 2.5,
                    _ => 1.0,
                };
                weighted_matches += weight;
            }
        }
        
        if query_words.is_empty() {
            0.0
        } else {
            (weighted_matches / (query_words.len() as f32 * 2.5)).min(1.0)
        }
    }
    
    fn rerank(query: &str, logs: &[(String, f32)]) -> Vec<(String, f32, f32)> {
        let query_lower = query.to_lowercase();
        let query_words: Vec<&str> = query_lower.split_whitespace().collect();
        
        let mut ranked: Vec<(String, f32, f32)> = logs
            .iter()
            .map(|(msg, semantic_score)| {
                let keyword_score = compute_keyword_score(&query_words, msg);
                let final_score = (semantic_score * 0.7) + (keyword_score * 0.3);
                (msg.clone(), keyword_score, final_score)
            })
            .collect();
        
        ranked.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap());
        ranked
    }
    
    // Generate sample logs
    let sample_logs: Vec<(String, f32)> = (0..100)
        .map(|i| {
            let messages = [
                "GET /health 200 OK",
                "ERROR: Payment processing failed with timeout",
                "User authentication successful",
                "WARN: Database connection pool exhausted",
                "Connection refused to redis://:6379",
                "Request completed in 234ms",
                "Critical: Memory threshold exceeded 95%",
                "Debug: Cache miss for key user:123",
            ];
            (
                messages[i % messages.len()].to_string(),
                0.5 + (i as f32 / 200.0), // Simulated semantic scores
            )
        })
        .collect();
    
    let sizes = [10, 50, 100, 500, 1000];
    let mut group = c.benchmark_group("reranking");
    
    for size in sizes {
        let logs: Vec<(String, f32)> = sample_logs.iter().cycle().take(size).cloned().collect();
        
        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(
            BenchmarkId::new("rerank_logs", size),
            &logs,
            |b, logs| {
                b.iter(|| rerank(black_box("payment error timeout"), black_box(logs)))
            },
        );
    }
    
    group.finish();
}

fn bench_intent_detection(c: &mut Criterion) {
    fn detect_intent(query: &str) -> &'static str {
        if query.starts_with("why")
            || query.contains("what caused")
            || query.contains("root cause")
            || query.contains("reason for")
            || query.contains("what led to")
        {
            return "Causal";
        }
        
        if query.contains("trace") || query.contains("request id") {
            return "Trace";
        }
        
        if query.starts_with("summarize") || query.contains("overview") {
            return "Summary";
        }
        
        "Search"
    }
    
    c.bench_function("intent_detection", |b| {
        b.iter(|| {
            for query in QUERIES {
                detect_intent(black_box(query));
            }
        })
    });
}

fn bench_query_cleaning(c: &mut Criterion) {
    let filler_patterns: Vec<regex::Regex> = [
        r"^show\s+me\s+",
        r"^give\s+me\s+",
        r"^what\s+are\s+(?:the\s+)?",
        r"^find\s+(?:me\s+)?",
        r"^list\s+(?:all\s+)?",
    ]
    .iter()
    .map(|p| regex::Regex::new(p).unwrap())
    .collect();
    
    let time_patterns: Vec<regex::Regex> = [
        r"last\s+\d+\s*(?:hours?|minutes?|days?|h|m|d)\s*",
        r"\byesterday\b",
        r"\btoday\b",
    ]
    .iter()
    .map(|p| regex::Regex::new(p).unwrap())
    .collect();
    
    c.bench_function("query_cleaning", |b| {
        b.iter(|| {
            for query in QUERIES {
                let mut cleaned = query.to_string();
                for pattern in &filler_patterns {
                    cleaned = pattern.replace_all(&cleaned, "").to_string();
                }
                for pattern in &time_patterns {
                    cleaned = pattern.replace_all(&cleaned, " ").to_string();
                }
                black_box(cleaned.split_whitespace().collect::<Vec<_>>().join(" "));
            }
        })
    });
}

criterion_group!(
    benches,
    bench_query_analysis,
    bench_reranking_simulation,
    bench_intent_detection,
    bench_query_cleaning,
);

criterion_main!(benches);
