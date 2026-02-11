//! Benchmark for log parsing operations
//! Run: cargo bench -p logai-core --bench parsing

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use logai_core::parser::{ApacheParser, LogParser, NginxParser, ParserRegistry, SyslogParser};
use logai_core::{LogEntry, RawLogEntry};
use std::collections::HashMap;
use std::hint::black_box;

// Sample log lines for benchmarking
const APACHE_LOG: &str = r#"[Tue Feb 10 14:30:45.123 2026] [error] [client 192.168.1.100] Failed to connect to database: connection refused"#;
const NGINX_LOG: &str = r#"192.168.1.50 - alice [10/Feb/2026:14:30:45 +0000] "GET /api/users/123 HTTP/1.1" 500 1234 "-" "Mozilla/5.0""#;
const SYSLOG_LOG: &str = r#"Feb 10 14:30:45 server-01 sshd[12345]: Failed password for invalid user admin from 10.0.0.1 port 22"#;

fn bench_apache_parser(c: &mut Criterion) {
    let parser = ApacheParser::new();
    
    c.bench_function("apache_parser_single", |b| {
        b.iter(|| parser.parse(black_box(APACHE_LOG)))
    });
}

fn bench_nginx_parser(c: &mut Criterion) {
    let parser = NginxParser::new();
    
    c.bench_function("nginx_parser_single", |b| {
        b.iter(|| parser.parse(black_box(NGINX_LOG)))
    });
}

fn bench_syslog_parser(c: &mut Criterion) {
    let parser = SyslogParser::new();
    
    c.bench_function("syslog_parser_single", |b| {
        b.iter(|| parser.parse(black_box(SYSLOG_LOG)))
    });
}

fn bench_parser_registry(c: &mut Criterion) {
    let mut registry = ParserRegistry::new();
    registry.register(Box::new(ApacheParser::new()));
    registry.register(Box::new(NginxParser::new()));
    registry.register(Box::new(SyslogParser::new()));
    
    let mut group = c.benchmark_group("parser_registry");
    
    group.bench_function("apache_via_registry", |b| {
        b.iter(|| registry.parse("apache", black_box(APACHE_LOG)))
    });
    
    group.bench_function("nginx_via_registry", |b| {
        b.iter(|| registry.parse("nginx", black_box(NGINX_LOG)))
    });
    
    group.bench_function("syslog_via_registry", |b| {
        b.iter(|| registry.parse("syslog", black_box(SYSLOG_LOG)))
    });
    
    group.finish();
}

fn bench_log_entry_conversion(c: &mut Criterion) {
    let raw = RawLogEntry {
        message: "Test error message".to_string(),
        timestamp: Some(chrono::Utc::now()),
        service: Some("test-service".to_string()),
        level: Some(logai_core::LogLevel::Error),
        trace_id: Some("abc-123-xyz".to_string()),
        fields: HashMap::from([
            ("user_id".to_string(), serde_json::json!("u123")),
            ("endpoint".to_string(), serde_json::json!("/api/test")),
        ]),
    };
    
    c.bench_function("raw_to_log_entry", |b| {
        b.iter(|| LogEntry::from_raw(black_box(raw.clone())))
    });
}

fn bench_batch_parsing(c: &mut Criterion) {
    let parser = NginxParser::new();
    
    // Generate batch of logs
    let batch_sizes = [10, 100, 1000, 10000];
    
    let mut group = c.benchmark_group("batch_parsing");
    
    for size in batch_sizes {
        let logs: Vec<String> = (0..size)
            .map(|i| format!(
                r#"192.168.1.{} - user{} [10/Feb/2026:14:30:45 +0000] "GET /api/test/{} HTTP/1.1" 200 1234 "-" "Mozilla/5.0""#,
                i % 255, i, i
            ))
            .collect();
        
        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(BenchmarkId::new("nginx", size), &logs, |b, logs| {
            b.iter(|| {
                logs.iter()
                    .map(|log| parser.parse(log))
                    .collect::<Vec<_>>()
            })
        });
    }
    
    group.finish();
}

fn bench_json_serialization(c: &mut Criterion) {
    let raw = RawLogEntry {
        message: "Connection timeout to database".to_string(),
        timestamp: Some(chrono::Utc::now()),
        service: Some("payment-service".to_string()),
        level: Some(logai_core::LogLevel::Error),
        trace_id: Some("trace-123".to_string()),
        fields: HashMap::from([
            ("user_id".to_string(), serde_json::json!("u999")),
            ("amount".to_string(), serde_json::json!(99.99)),
            ("currency".to_string(), serde_json::json!("USD")),
        ]),
    };
    let entry = LogEntry::from_raw(raw);
    
    let mut group = c.benchmark_group("serialization");
    
    group.bench_function("log_entry_to_json", |b| {
        b.iter(|| serde_json::to_string(black_box(&entry)))
    });
    
    let json_str = serde_json::to_string(&entry).unwrap();
    group.bench_function("json_to_log_entry", |b| {
        b.iter(|| serde_json::from_str::<LogEntry>(black_box(&json_str)))
    });
    
    group.finish();
}

criterion_group!(
    benches,
    bench_apache_parser,
    bench_nginx_parser,
    bench_syslog_parser,
    bench_parser_registry,
    bench_log_entry_conversion,
    bench_batch_parsing,
    bench_json_serialization,
);

criterion_main!(benches);
