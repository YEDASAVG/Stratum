// LogAI Log Simulator - Generates realistic logs for testing
// Supports multiple scenarios with correlated logs across services

use chrono::Utc;
use clap::{Parser, ValueEnum};
use colored::Colorize;
use rand::prelude::*;
use serde::Serialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::time::Duration;
use uuid::Uuid;

const API_URL: &str = "http://localhost:3000";

// 5 Services
#[allow(dead_code)]
const SERVICES: [&str; 5] = [
    "api-gateway",
    "auth-service", 
    "user-service",
    "payment-service",
    "database-service",
];

#[derive(Parser)]
#[command(name = "logai-simulate")]
#[command(about = "Generate realistic logs for LogAI testing")]
struct Args {
    /// Scenario to simulate
    #[arg(short, long, default_value = "normal")]
    scenario: Scenario,

    /// Interval between log batches (seconds)
    #[arg(short, long, default_value = "3")]
    interval: u64,

    /// Error rate percentage (0-100)
    #[arg(short, long, default_value = "10")]
    error_rate: u8,

    /// Run duration in seconds (0 = forever)
    #[arg(short, long, default_value = "0")]
    duration: u64,

    /// Burst mode - send logs faster
    #[arg(long)]
    burst: bool,
}

#[derive(Clone, Copy, Debug, ValueEnum, Default)]
enum Scenario {
    #[default]
    Normal,
    PaymentOutage,
    DatabaseSlow,
    AuthAttack,
    HighTraffic,
}

// Matches RawLogEntry from logai-core
#[derive(Serialize)]
struct LogEntry {
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    timestamp: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    service: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    level: Option<String>,  // lowercase: info, warn, error, debug
    #[serde(skip_serializing_if = "Option::is_none")]
    trace_id: Option<String>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    fields: HashMap<String, Value>,
}

impl LogEntry {
    fn new(message: impl Into<String>, service: &str, level: &str) -> Self {
        Self {
            message: message.into(),
            timestamp: Some(Utc::now().to_rfc3339()),
            service: Some(service.to_string()),
            level: Some(level.to_lowercase()),  // Convert to lowercase
            trace_id: None,
            fields: HashMap::new(),
        }
    }
    
    fn with_trace_id(mut self, id: &str) -> Self {
        self.trace_id = Some(id.to_string());
        self
    }
    
    fn with_field(mut self, key: &str, value: Value) -> Self {
        self.fields.insert(key.to_string(), value);
        self
    }
}

struct SimulatorState {
    phase: u32,           // Current phase of scenario
    tick: u64,            // Ticks since start
    error_rate: u8,       // Current error rate
    base_latency: u64,    // Base latency for DB
}

impl Default for SimulatorState {
    fn default() -> Self {
        Self {
            phase: 0,
            tick: 0,
            error_rate: 10,
            base_latency: 50,
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let client = reqwest::Client::new();
    
    println!();
    println!("{}", "╔══════════════════════════════════════════════════╗".cyan());
    println!("{}", "║         🚀 LogAI Log Simulator                   ║".cyan().bold());
    println!("{}", "╠══════════════════════════════════════════════════╣".cyan());
    println!("║  Scenario: {:<37} ║", format!("{:?}", args.scenario).yellow());
    println!("║  Interval: {:<37} ║", format!("{}s", args.interval).green());
    println!("║  Error Rate: {:<35} ║", format!("{}%", args.error_rate).red());
    println!("{}", "╚══════════════════════════════════════════════════╝".cyan());
    println!();
    println!("{}", "Press Ctrl+C to stop".dimmed());
    println!();

    let mut state = SimulatorState {
        error_rate: args.error_rate,
        ..Default::default()
    };

    let interval = if args.burst {
        Duration::from_millis(500)
    } else {
        Duration::from_secs(args.interval)
    };

    let start = std::time::Instant::now();

    loop {
        // Check duration limit
        if args.duration > 0 && start.elapsed().as_secs() >= args.duration {
            println!("\n{} Duration limit reached.", "✓".green());
            break;
        }

        // Update state based on scenario
        update_state(&args.scenario, &mut state);

        // Generate logs for this tick
        let logs = generate_logs(&args.scenario, &mut state);

        // Send logs to API
        for log in &logs {
            match send_log(&client, log).await {
                Ok(_) => {
                    let level = log.level.as_deref().unwrap_or("info");
                    let level_colored = match level {
                        "error" => format!("[{}]", level.to_uppercase()).red().to_string(),
                        "warn" => format!("[{}]", level.to_uppercase()).yellow().to_string(),
                        "info" => format!("[{}]", level.to_uppercase()).green().to_string(),
                        _ => format!("[{}]", level.to_uppercase()).blue().to_string(),
                    };
                    let service = log.service.as_deref().unwrap_or("unknown");
                    println!(
                        "{} {} {} {}",
                        Utc::now().format("%H:%M:%S").to_string().dimmed(),
                        level_colored,
                        service.cyan(),
                        truncate(&log.message, 50)
                    );
                }
                Err(e) => {
                    println!("{} Failed to send log: {}", "✗".red(), e);
                }
            }
        }

        state.tick += 1;
        tokio::time::sleep(interval).await;
    }

    Ok(())
}

fn update_state(scenario: &Scenario, state: &mut SimulatorState) {
    match scenario {
        Scenario::Normal => {
            // Stable state
        }
        Scenario::PaymentOutage => {
            // Phase progression
            if state.tick < 10 {
                state.phase = 0; // Normal
                state.error_rate = 10;
            } else if state.tick < 20 {
                state.phase = 1; // DB slowing
                state.base_latency = 200 + (state.tick * 50) as u64;
            } else if state.tick < 30 {
                state.phase = 2; // Payment failing
                state.error_rate = 50;
            } else {
                state.phase = 3; // Full outage
                state.error_rate = 80;
            }
        }
        Scenario::DatabaseSlow => {
            state.base_latency = 50 + (state.tick * 100) as u64;
            if state.base_latency > 5000 {
                state.error_rate = 60;
            }
        }
        Scenario::AuthAttack => {
            // Sustained attack pattern
            state.error_rate = 30;
        }
        Scenario::HighTraffic => {
            // Gradually increasing stress
            if state.tick > 20 {
                state.error_rate = 20;
            }
        }
    }
}

fn generate_logs(scenario: &Scenario, state: &mut SimulatorState) -> Vec<LogEntry> {
    let mut logs = Vec::new();
    let mut rng = rand::rng();
    
    // Generate a request flow (correlated logs)
    let request_id = Uuid::new_v4().to_string()[..8].to_string();
    let user_id = format!("user_{}", rng.random_range(1000..9999));

    match scenario {
        Scenario::Normal => {
            logs.extend(generate_normal_flow(&request_id, &user_id, state, &mut rng));
        }
        Scenario::PaymentOutage => {
            logs.extend(generate_payment_outage_flow(&request_id, &user_id, state, &mut rng));
        }
        Scenario::DatabaseSlow => {
            logs.extend(generate_db_slow_flow(&request_id, &user_id, state, &mut rng));
        }
        Scenario::AuthAttack => {
            logs.extend(generate_auth_attack_flow(&request_id, state, &mut rng));
        }
        Scenario::HighTraffic => {
            // Multiple concurrent requests
            for _ in 0..5 {
                let req_id = Uuid::new_v4().to_string()[..8].to_string();
                logs.extend(generate_normal_flow(&req_id, &user_id, state, &mut rng));
            }
        }
    }

    logs
}

fn generate_normal_flow(request_id: &str, user_id: &str, state: &SimulatorState, rng: &mut impl Rng) -> Vec<LogEntry> {
    let mut logs = Vec::new();
    let is_error = rng.random_ratio(state.error_rate as u32, 100);
    
    let endpoints = ["/api/users", "/api/orders", "/api/products", "/api/checkout", "/health"];
    let endpoint = endpoints[rng.random_range(0..endpoints.len())];
    
    // 1. Gateway receives request
    logs.push(LogEntry::new(
        format!("Incoming request GET {}", endpoint),
        "api-gateway",
        "info"
    ).with_trace_id(request_id)
     .with_field("user_id", json!(user_id))
     .with_field("endpoint", json!(endpoint)));

    // 2. Auth check
    logs.push(LogEntry::new(
        format!("Token validated for {}", user_id),
        "auth-service",
        "info"
    ).with_trace_id(request_id)
     .with_field("user_id", json!(user_id))
     .with_field("latency_ms", json!(rng.random_range(5..20))));

    // 3. DB query
    let db_latency = state.base_latency + rng.random_range(0..50);
    let db_level = if db_latency > 1000 { "warn" } else { "debug" };
    logs.push(LogEntry::new(
        format!("Query executed: SELECT * FROM users WHERE id = {}", rng.random_range(1..1000)),
        "database-service",
        db_level
    ).with_trace_id(request_id)
     .with_field("latency_ms", json!(db_latency)));

    // 4. Response
    if is_error {
        let errors = [(500, "Internal Server Error"), (502, "Bad Gateway"), (503, "Service Unavailable")];
        let (code, msg) = errors[rng.random_range(0..errors.len())];
        logs.push(LogEntry::new(
            format!("{}: {}", code, msg),
            "api-gateway",
            "error"
        ).with_trace_id(request_id)
         .with_field("user_id", json!(user_id))
         .with_field("latency_ms", json!(db_latency + 50))
         .with_field("status_code", json!(code))
         .with_field("endpoint", json!(endpoint))
         .with_field("error_code", json!(format!("ERR_{}", code))));
    } else {
        let codes = [200, 201, 301, 302];
        let code = codes[rng.random_range(0..codes.len())];
        logs.push(LogEntry::new(
            format!("Request completed GET {} - {}", endpoint, code),
            "api-gateway",
            "info"
        ).with_trace_id(request_id)
         .with_field("user_id", json!(user_id))
         .with_field("latency_ms", json!(db_latency + 30))
         .with_field("status_code", json!(code))
         .with_field("endpoint", json!(endpoint)));
    }

    logs
}

fn generate_payment_outage_flow(request_id: &str, user_id: &str, state: &SimulatorState, rng: &mut impl Rng) -> Vec<LogEntry> {
    let mut logs = Vec::new();

    match state.phase {
        0 => {
            // Normal payment flow
            logs.push(LogEntry::new(
                format!("Payment initiated for order ord_{}", rng.random_range(10000..99999)),
                "payment-service",
                "info"
            ).with_trace_id(request_id)
             .with_field("user_id", json!(user_id))
             .with_field("endpoint", json!("/api/payments")));
            
            logs.push(LogEntry::new(
                "Payment processed successfully",
                "payment-service",
                "info"
            ).with_trace_id(request_id)
             .with_field("user_id", json!(user_id))
             .with_field("latency_ms", json!(rng.random_range(100..300)))
             .with_field("status_code", json!(200)));
        }
        1 => {
            // DB slowing down
            logs.push(LogEntry::new(
                format!("Query slow: {}ms - SELECT * FROM transactions", state.base_latency),
                "database-service",
                "warn"
            ).with_trace_id(request_id)
             .with_field("latency_ms", json!(state.base_latency)));
            
            logs.push(LogEntry::new(
                "Payment processing delayed due to slow DB response",
                "payment-service",
                "warn"
            ).with_trace_id(request_id)
             .with_field("user_id", json!(user_id))
             .with_field("latency_ms", json!(state.base_latency + 200)));
        }
        2 => {
            // Payments starting to fail
            logs.push(LogEntry::new(
                "Database connection timeout after 5000ms",
                "database-service",
                "error"
            ).with_trace_id(request_id)
             .with_field("latency_ms", json!(5000))
             .with_field("error_code", json!("DB_TIMEOUT")));
            
            logs.push(LogEntry::new(
                format!("Payment failed: Unable to verify balance for {}", user_id),
                "payment-service",
                "error"
            ).with_trace_id(request_id)
             .with_field("user_id", json!(user_id))
             .with_field("latency_ms", json!(5200))
             .with_field("status_code", json!(500))
             .with_field("endpoint", json!("/api/payments"))
             .with_field("error_code", json!("PAYMENT_FAILED")));
            
            logs.push(LogEntry::new(
                "500 Internal Server Error - /api/checkout",
                "api-gateway",
                "error"
            ).with_trace_id(request_id)
             .with_field("user_id", json!(user_id))
             .with_field("latency_ms", json!(5500))
             .with_field("status_code", json!(500))
             .with_field("endpoint", json!("/api/checkout"))
             .with_field("error_code", json!("CHECKOUT_FAILED")));
        }
        _ => {
            // Full outage
            logs.push(LogEntry::new(
                "Connection pool exhausted - all connections busy",
                "database-service",
                "error"
            ).with_trace_id(request_id)
             .with_field("error_code", json!("POOL_EXHAUSTED")));
            
            logs.push(LogEntry::new(
                "CRITICAL: Payment service unavailable - circuit breaker OPEN",
                "payment-service",
                "error"
            ).with_trace_id(request_id)
             .with_field("status_code", json!(503))
             .with_field("error_code", json!("CIRCUIT_OPEN")));
            
            logs.push(LogEntry::new(
                "503 Service Unavailable - Payment system down",
                "api-gateway",
                "error"
            ).with_trace_id(request_id)
             .with_field("user_id", json!(user_id))
             .with_field("status_code", json!(503))
             .with_field("endpoint", json!("/api/checkout"))
             .with_field("error_code", json!("SERVICE_UNAVAILABLE")));
        }
    }

    logs
}

fn generate_db_slow_flow(request_id: &str, user_id: &str, state: &SimulatorState, rng: &mut impl Rng) -> Vec<LogEntry> {
    let mut logs = Vec::new();
    let latency = state.base_latency;

    let level = if latency > 2000 { "error" } else if latency > 500 { "warn" } else { "debug" };

    let mut entry = LogEntry::new(
        format!("Query execution time: {}ms - SELECT * FROM orders WHERE user_id = {}", latency, rng.random_range(1..1000)),
        "database-service",
        level
    ).with_trace_id(request_id)
     .with_field("latency_ms", json!(latency));
    
    if latency > 2000 {
        entry = entry.with_field("error_code", json!("SLOW_QUERY"));
    }
    logs.push(entry);

    if latency > 1000 {
        logs.push(LogEntry::new(
            format!("Request timeout warning - upstream latency {}ms", latency),
            "api-gateway",
            "warn"
        ).with_trace_id(request_id)
         .with_field("user_id", json!(user_id))
         .with_field("latency_ms", json!(latency + 50))
         .with_field("endpoint", json!("/api/orders")));
    }

    if latency > 3000 {
        logs.push(LogEntry::new(
            "Database connection pool running low: 2/10 connections available",
            "database-service",
            "warn"
        ).with_field("error_code", json!("LOW_POOL")));
    }

    if latency > 5000 {
        logs.push(LogEntry::new(
            "Query timeout: exceeded 5000ms limit",
            "database-service",
            "error"
        ).with_trace_id(request_id)
         .with_field("latency_ms", json!(5000))
         .with_field("error_code", json!("QUERY_TIMEOUT")));
    }

    logs
}

fn generate_auth_attack_flow(request_id: &str, state: &SimulatorState, rng: &mut impl Rng) -> Vec<LogEntry> {
    let mut logs = Vec::new();

    let attacker_ips = ["10.0.0.5", "192.168.1.100", "172.16.0.50", "10.10.10.10"];
    let ip = attacker_ips[rng.random_range(0..attacker_ips.len())];
    let usernames = ["admin", "root", "administrator", "user", "test"];
    let username = usernames[rng.random_range(0..usernames.len())];

    // Failed login attempts
    logs.push(LogEntry::new(
        format!("Failed password for invalid user {} from {} port 22", username, ip),
        "auth-service",
        "warn"
    ).with_trace_id(request_id)
     .with_field("latency_ms", json!(rng.random_range(50..200)))
     .with_field("status_code", json!(401))
     .with_field("endpoint", json!("/api/login"))
     .with_field("error_code", json!("AUTH_FAILED"))
     .with_field("source_ip", json!(ip)));

    // Rate limit warning
    if state.tick % 5 == 0 {
        logs.push(LogEntry::new(
            format!("Rate limit warning: {} attempts from {} in last minute", rng.random_range(10..30), ip),
            "auth-service",
            "warn"
        ).with_field("error_code", json!("RATE_LIMIT_WARN"))
         .with_field("source_ip", json!(ip)));
    }

    // Block after many attempts
    if state.tick % 10 == 0 {
        logs.push(LogEntry::new(
            format!("IP {} blocked after {} failed attempts", ip, rng.random_range(50..100)),
            "auth-service",
            "error"
        ).with_field("status_code", json!(403))
         .with_field("error_code", json!("IP_BLOCKED"))
         .with_field("source_ip", json!(ip)));
    }

    // Some legitimate traffic mixed in
    if rng.random_ratio(30, 100) {
        let legit_user = format!("user_{}", rng.random_range(1000..9999));
        logs.push(LogEntry::new(
            format!("User {} logged in successfully", legit_user),
            "auth-service",
            "info"
        ).with_trace_id(&Uuid::new_v4().to_string()[..8])
         .with_field("user_id", json!(legit_user))
         .with_field("latency_ms", json!(rng.random_range(20..100)))
         .with_field("status_code", json!(200))
         .with_field("endpoint", json!("/api/login")));
    }

    logs
}

async fn send_log(client: &reqwest::Client, log: &LogEntry) -> Result<(), Box<dyn std::error::Error>> {
    let url = format!("{}/api/logs", API_URL);
    client
        .post(&url)
        .json(log)
        .send()
        .await?
        .error_for_status()?;
    Ok(())
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() > max {
        format!("{}...", &s[..max - 3])
    } else {
        s.to_string()
    }
}
