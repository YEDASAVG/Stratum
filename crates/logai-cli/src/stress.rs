//! LogAI Stress Test Tool
//! High-performance log ingestion stress tester
//! Target: 500K+ logs/sec

use chrono::Utc;
use clap::Parser;
use indicatif::{ProgressBar, ProgressStyle};
use rand::prelude::*;
use serde::Serialize;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Semaphore;

#[derive(Parser, Debug)]
#[command(name = "logai-stress")]
#[command(about = "Stress test LogAI ingestion pipeline")]
struct Args {
    /// Target logs per second (0 = unlimited)
    #[arg(short, long, default_value = "100000")]
    rate: u64,

    /// Total logs to send (0 = run forever)
    #[arg(short, long, default_value = "1000000")]
    total: u64,

    /// Batch size (logs per request)
    #[arg(short, long, default_value = "1000")]
    batch: usize,

    /// Number of concurrent workers
    #[arg(short, long, default_value = "100")]
    workers: usize,

    /// API endpoint
    #[arg(short, long, default_value = "http://localhost:3000")]
    endpoint: String,

    /// Log format (structured, apache, nginx, syslog)
    #[arg(short, long, default_value = "structured")]
    format: String,
}

#[derive(Serialize)]
struct LogEntry {
    service: String,
    level: String,
    message: String,
    timestamp: String,
    trace_id: Option<String>,
    metadata: serde_json::Value,
}

#[derive(Serialize)]
struct RawLogRequest {
    format: String,
    service: String,
    lines: Vec<String>,
}

struct Metrics {
    sent: AtomicU64,
    success: AtomicU64,
    failed: AtomicU64,
    latency_sum_us: AtomicU64,
    latency_count: AtomicU64,
}

impl Metrics {
    fn new() -> Self {
        Self {
            sent: AtomicU64::new(0),
            success: AtomicU64::new(0),
            failed: AtomicU64::new(0),
            latency_sum_us: AtomicU64::new(0),
            latency_count: AtomicU64::new(0),
        }
    }

    fn record_success(&self, count: u64, latency: Duration) {
        self.sent.fetch_add(count, Ordering::Relaxed);
        self.success.fetch_add(count, Ordering::Relaxed);
        self.latency_sum_us
            .fetch_add(latency.as_micros() as u64, Ordering::Relaxed);
        self.latency_count.fetch_add(1, Ordering::Relaxed);
    }

    fn record_failure(&self, count: u64) {
        self.sent.fetch_add(count, Ordering::Relaxed);
        self.failed.fetch_add(count, Ordering::Relaxed);
    }

    fn get_stats(&self) -> (u64, u64, u64, f64) {
        let sent = self.sent.load(Ordering::Relaxed);
        let success = self.success.load(Ordering::Relaxed);
        let failed = self.failed.load(Ordering::Relaxed);
        let latency_count = self.latency_count.load(Ordering::Relaxed);
        let avg_latency = if latency_count > 0 {
            self.latency_sum_us.load(Ordering::Relaxed) as f64 / latency_count as f64 / 1000.0
        } else {
            0.0
        };
        (sent, success, failed, avg_latency)
    }
}

fn generate_structured_batch(batch_size: usize) -> Vec<LogEntry> {
    let mut rng = rand::rng();
    let levels = ["info", "warn", "error", "debug"];
    let services = [
        "api-gateway",
        "user-service",
        "payment-service",
        "order-service",
        "auth-service",
    ];
    let messages = [
        "Request processed successfully",
        "Connection timeout to database",
        "User authentication failed",
        "Payment transaction completed",
        "Cache miss for key",
        "Rate limit exceeded",
        "Health check passed",
        "Circuit breaker opened",
        "Retrying failed request",
        "Message published to queue",
    ];

    (0..batch_size)
        .map(|_| LogEntry {
            service: services[rng.random_range(0..services.len())].to_string(),
            level: levels[rng.random_range(0..levels.len())].to_string(),
            message: format!(
                "{} - request_id={} latency={}ms",
                messages[rng.random_range(0..messages.len())],
                uuid::Uuid::new_v4(),
                rng.random_range(1..500)
            ),
            timestamp: Utc::now().to_rfc3339(),
            trace_id: Some(uuid::Uuid::new_v4().to_string()),
            metadata: {
                let regions = ["us-east-1", "us-west-2", "eu-west-1"];
                serde_json::json!({
                    "host": format!("server-{}", rng.random_range(1..100)),
                    "region": regions[rng.random_range(0..3)],
                    "version": "1.2.3",
                    "duration_ms": rng.random_range(1..1000),
                })
            },
        })
        .collect()
}

fn generate_raw_batch(format: &str, batch_size: usize) -> Vec<String> {
    let mut rng = rand::rng();

    match format {
        "apache" => (0..batch_size)
            .map(|_| {
                let levels = ["error", "warn", "notice", "info"];
                format!(
                    "[{}] [{}] [client {}.{}.{}.{}] {}",
                    Utc::now().format("%a %b %d %H:%M:%S%.3f %Y"),
                    levels[rng.random_range(0..levels.len())],
                    rng.random_range(1..255),
                    rng.random_range(0..255),
                    rng.random_range(0..255),
                    rng.random_range(1..255),
                    [
                        "File does not exist",
                        "Connection refused",
                        "Permission denied",
                        "Timeout waiting for output",
                    ][rng.random_range(0..4)]
                )
            })
            .collect(),

        "nginx" => (0..batch_size)
            .map(|_| {
                let status = [200, 201, 301, 302, 400, 401, 403, 404, 500, 502, 503]
                    [rng.random_range(0..11)];
                let paths = ["/api/users", "/api/orders", "/health", "/static/app.js"];
                format!(
                    "{}.{}.{}.{} - - [{}] \"GET {} HTTP/1.1\" {} {} \"-\" \"Mozilla/5.0\"",
                    rng.random_range(1..255),
                    rng.random_range(0..255),
                    rng.random_range(0..255),
                    rng.random_range(1..255),
                    Utc::now().format("%d/%b/%Y:%H:%M:%S +0000"),
                    paths[rng.random_range(0..paths.len())],
                    status,
                    rng.random_range(100..50000)
                )
            })
            .collect(),

        "syslog" => (0..batch_size)
            .map(|_| {
                let facilities = ["sshd", "kernel", "cron", "sudo", "systemd"];
                format!(
                    "{} server-{:02} {}[{}]: {}",
                    Utc::now().format("%b %d %H:%M:%S"),
                    rng.random_range(1..50),
                    facilities[rng.random_range(0..facilities.len())],
                    rng.random_range(1000..65000),
                    [
                        "Connection established",
                        "Authentication failure",
                        "Session opened",
                        "Command executed",
                    ][rng.random_range(0..4)]
                )
            })
            .collect(),

        _ => vec![],
    }
}

async fn send_structured_batch(
    client: &reqwest::Client,
    endpoint: &str,
    batch: Vec<LogEntry>,
    metrics: &Metrics,
) {
    let count = batch.len() as u64;
    let start = Instant::now();

    match client
        .post(format!("{}/api/logs", endpoint))
        .json(&batch)
        .send()
        .await
    {
        Ok(resp) if resp.status().is_success() => {
            metrics.record_success(count, start.elapsed());
        }
        _ => {
            metrics.record_failure(count);
        }
    }
}

async fn send_raw_batch(
    client: &reqwest::Client,
    endpoint: &str,
    format: &str,
    lines: Vec<String>,
    metrics: &Metrics,
) {
    let count = lines.len() as u64;
    let start = Instant::now();

    let req = RawLogRequest {
        format: format.to_string(),
        service: format!("stress-test-{}", format),
        lines,
    };

    match client
        .post(format!("{}/api/logs/raw", endpoint))
        .json(&req)
        .send()
        .await
    {
        Ok(resp) if resp.status().is_success() => {
            metrics.record_success(count, start.elapsed());
        }
        _ => {
            metrics.record_failure(count);
        }
    }
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    println!("╔═══════════════════════════════════════════════════════════╗");
    println!("║           LogAI Stress Test Tool v0.1.0                    ║");
    println!("╠═══════════════════════════════════════════════════════════╣");
    println!(
        "║ Target Rate:    {:>10} logs/sec                       ║",
        args.rate
    );
    println!(
        "║ Total Logs:     {:>10}                                ║",
        args.total
    );
    println!(
        "║ Batch Size:     {:>10}                                ║",
        args.batch
    );
    println!(
        "║ Workers:        {:>10}                                ║",
        args.workers
    );
    println!("║ Format:         {:>10}                                ║", args.format);
    println!(
        "║ Endpoint:       {:>42} ║",
        args.endpoint
    );
    println!("╚═══════════════════════════════════════════════════════════╝");
    println!();

    // Create HTTP client with connection pooling
    let client = reqwest::Client::builder()
        .pool_max_idle_per_host(args.workers)
        .pool_idle_timeout(Duration::from_secs(30))
        .timeout(Duration::from_secs(30))
        .build()
        .expect("Failed to create HTTP client");

    let metrics = Arc::new(Metrics::new());
    let semaphore = Arc::new(Semaphore::new(args.workers));
    let start_time = Instant::now();

    // Progress bar
    let progress = if args.total > 0 {
        let pb = ProgressBar::new(args.total);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({per_sec}) | Success: {msg}")
                .expect("Invalid template")
                .progress_chars("█▓▒░"),
        );
        Some(pb)
    } else {
        None
    };

    // Spawn metrics reporter
    let metrics_clone = metrics.clone();
    let progress_clone = progress.clone();
    let reporter = tokio::spawn(async move {
        let mut last_sent = 0u64;
        let mut interval = tokio::time::interval(Duration::from_secs(1));

        loop {
            interval.tick().await;
            let (sent, success, failed, avg_latency) = metrics_clone.get_stats();
            let rate = sent - last_sent;
            last_sent = sent;

            if let Some(ref pb) = progress_clone {
                pb.set_position(sent);
                pb.set_message(format!(
                    "{} | Failed: {} | Rate: {}/s | Latency: {:.1}ms",
                    success, failed, rate, avg_latency
                ));
            } else {
                println!(
                    "📊 Sent: {} | Success: {} | Failed: {} | Rate: {}/s | Avg Latency: {:.1}ms",
                    sent, success, failed, rate, avg_latency
                );
            }
        }
    });

    // Calculate batches needed
    let total_batches = if args.total > 0 {
        (args.total as usize + args.batch - 1) / args.batch
    } else {
        usize::MAX
    };

    // Rate limiting
    let batch_interval = if args.rate > 0 {
        Some(Duration::from_secs_f64(
            args.batch as f64 / args.rate as f64,
        ))
    } else {
        None
    };

    let mut handles = vec![];

    for _batch_num in 0..total_batches {
        // Check if we should stop
        if args.total > 0 {
            let sent = metrics.sent.load(Ordering::Relaxed);
            if sent >= args.total {
                break;
            }
        }

        // Rate limiting
        if let Some(interval) = batch_interval {
            tokio::time::sleep(interval).await;
        }

        let permit = semaphore.clone().acquire_owned().await.unwrap();
        let client = client.clone();
        let endpoint = args.endpoint.clone();
        let format = args.format.clone();
        let batch_size = args.batch;
        let metrics = metrics.clone();

        let handle = tokio::spawn(async move {
            if format == "structured" {
                let batch = generate_structured_batch(batch_size);
                send_structured_batch(&client, &endpoint, batch, &metrics).await;
            } else {
                let lines = generate_raw_batch(&format, batch_size);
                send_raw_batch(&client, &endpoint, &format, lines, &metrics).await;
            }
            drop(permit);
        });

        handles.push(handle);

        // Limit outstanding handles
        if handles.len() >= args.workers * 2 {
            if let Some(h) = handles.pop() {
                let _ = h.await;
            }
        }
    }

    // Wait for all remaining tasks
    for handle in handles {
        let _ = handle.await;
    }

    reporter.abort();

    let elapsed = start_time.elapsed();
    let (sent, success, failed, avg_latency) = metrics.get_stats();

    if let Some(pb) = progress {
        pb.finish_with_message("Complete!");
    }

    println!();
    println!("╔═══════════════════════════════════════════════════════════╗");
    println!("║                    STRESS TEST RESULTS                     ║");
    println!("╠═══════════════════════════════════════════════════════════╣");
    println!(
        "║ Total Sent:     {:>10} logs                          ║",
        sent
    );
    println!(
        "║ Successful:     {:>10} logs                          ║",
        success
    );
    println!(
        "║ Failed:         {:>10} logs                          ║",
        failed
    );
    println!(
        "║ Duration:       {:>10.2} seconds                      ║",
        elapsed.as_secs_f64()
    );
    println!(
        "║ Throughput:     {:>10.0} logs/sec                     ║",
        sent as f64 / elapsed.as_secs_f64()
    );
    println!(
        "║ Avg Latency:    {:>10.2} ms                           ║",
        avg_latency
    );
    println!(
        "║ Success Rate:   {:>10.2}%                             ║",
        if sent > 0 {
            success as f64 / sent as f64 * 100.0
        } else {
            0.0
        }
    );
    println!("╚═══════════════════════════════════════════════════════════╝");
}
