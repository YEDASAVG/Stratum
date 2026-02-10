// LogAI CLI - AI-Powered Log Analysis

use clap::{Parser, Subcommand};
use colored::Colorize;
use comfy_table::{Table, presets::UTF8_FULL};
use serde::{Deserialize, Serialize};
use std::io::{self, BufRead, Write};
use std::process::Command as ProcessCommand;
use uuid::Uuid;

const DEFAULT_API_URL: &str = "http://localhost:3000";

#[derive(Parser)]
#[command(name = "logai")]
#[command(author = "LogAI Team")]
#[command(version = "0.1.0")]
#[command(about = "AI-Powered Log Analysis CLI", long_about = None)]
struct Cli {
    /// API server URL
    #[arg(short, long, default_value = DEFAULT_API_URL)]
    api_url: String,

    /// API key for authentication (or set LOGAI_API_KEY env var)
    #[arg(short = 'k', long, env = "LOGAI_API_KEY")]
    api_key: Option<String>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Ask AI a question about your logs
    Ask {
        /// Your question in natural language
        question: String,
    },

    /// Semantic search for logs
    Search {
        /// Search query
        query: String,

        /// Maximum results to return
        #[arg(short, long, default_value = "10")]
        limit: usize,
    },

    /// Check system health status
    Status,

    /// Ingest logs from a file
    Ingest {
        /// Path to log file
        file: String,

        /// Log format (json, apache, nginx, syslog)
        #[arg(short, long, default_value = "json")]
        format: String,

        /// Service name for raw logs
        #[arg(short, long, default_value = "imported")]
        service: String,
    },

    /// Show recent logs
    Logs {
        /// Number of logs to show
        #[arg(short, long, default_value = "20")]
        limit: usize,

        /// Filter by level (error, warn, info, debug)
        #[arg(short = 'L', long)]
        level: Option<String>,
    },

    /// Show system statistics
    Stats,

    /// Start the API server
    Serve {
        /// Port to listen on
        #[arg(short, long, default_value = "3000")]
        port: u16,
    },

    /// List active alerts
    Alerts {
        /// Filter by status (firing, acknowledged, resolved)
        #[arg(short, long)]
        status: Option<String>,
    },

    /// Check for anomalies now
    Anomalies {
        /// Service to check (default: all)
        #[arg(short, long)]
        service: Option<String>,
    },

    /// Interactive chat mode for debugging
    Chat {
        /// Initial question (optional)
        question: Option<String>,
    },
}

// API Response types
#[derive(Deserialize)]
struct AskResponse {
    answer: String,
    sources_count: usize,
    response_time_ms: u128,
    provider: String,
    query_analysis: QueryAnalysis,
}

#[derive(Deserialize)]
struct QueryAnalysis {
    search_query: String,
    time_filter: Option<String>,
    service_filter: Option<String>,
}

#[derive(Deserialize)]
struct SearchResult {
    score: f32,
    log_id: String,
    service: String,
    level: String,
    message: String,
    timestamp: String,
}

#[derive(Serialize)]
struct LogEntry {
    message: String,
    service: String,
    level: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    
    // Build client with optional API key header
    let mut headers = reqwest::header::HeaderMap::new();
    if let Some(ref key) = cli.api_key {
        headers.insert("X-API-Key", reqwest::header::HeaderValue::from_str(key)?);
    }
    let client = reqwest::Client::builder()
        .default_headers(headers)
        .build()?;

    match cli.command {
        Commands::Ask { question } => {
            ask_ai(&client, &cli.api_url, &question).await?;
        }
        Commands::Search { query, limit } => {
            search_logs(&client, &cli.api_url, &query, limit).await?;
        }
        Commands::Status => {
            check_status(&client, &cli.api_url).await?;
        }
        Commands::Ingest { file, format, service } => {
            ingest_file(&client, &cli.api_url, &file, &format, &service).await?;
        }
        Commands::Logs { limit, level } => {
            show_logs(&client, &cli.api_url, limit, level).await?;
        }
        Commands::Stats => {
            show_stats(&client, &cli.api_url).await?;
        }
        Commands::Serve { port } => {
            start_server(port)?;
        }
        Commands::Alerts { status } => {
            show_alerts(&client, &cli.api_url, status).await?;
        }
        Commands::Anomalies { service } => {
            check_anomalies(&client, &cli.api_url, service).await?;
        }
        Commands::Chat { question } => {
            interactive_chat(&client, &cli.api_url, question).await?;
        }
    }

    Ok(())
}

async fn ask_ai(
    client: &reqwest::Client,
    api_url: &str,
    question: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n{}", "🤖 Asking AI...".cyan().bold());
    println!("{}", "─".repeat(50).dimmed());

    let url = format!("{}/api/ask?q={}", api_url, urlencoding::encode(question));
    let response = client
        .get(&url)
        .send()
        .await?;

    if !response.status().is_success() {
        let error = response.text().await?;
        println!("{} {}", "Error:".red().bold(), error);
        return Ok(());
    }

    let result: AskResponse = response.json().await?;

    // Print answer
    println!("\n{}", "Answer:".green().bold());
    println!("{}", result.answer);

    // Print metadata
    println!("\n{}", "─".repeat(50).dimmed());
    println!(
        "{} {} | {} {} | {} {}ms",
        "Sources:".dimmed(),
        result.sources_count.to_string().yellow(),
        "Provider:".dimmed(),
        result.provider.cyan(),
        "Time:".dimmed(),
        result.response_time_ms.to_string().yellow()
    );

    if let Some(service) = result.query_analysis.service_filter {
        println!("{} {}", "Service filter:".dimmed(), service.magenta());
    }
    if let Some(time) = result.query_analysis.time_filter {
        println!("{} {}", "Time filter:".dimmed(), time.magenta());
    }

    Ok(())
}

async fn search_logs(
    client: &reqwest::Client,
    api_url: &str,
    query: &str,
    limit: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n{} \"{}\"", "🔍 Searching:".cyan().bold(), query);
    println!("{}", "─".repeat(60).dimmed());

    let url = format!("{}/api/search?q={}&limit={}", api_url, urlencoding::encode(query), limit);
    let response = client
        .get(&url)
        .send()
        .await?;

    if !response.status().is_success() {
        let error = response.text().await?;
        println!("{} {}", "Error:".red().bold(), error);
        return Ok(());
    }

    let results: Vec<SearchResult> = response.json().await?;

    if results.is_empty() {
        println!("{}", "No results found.".yellow());
        return Ok(());
    }

    // Create table
    let mut table = Table::new();
    table.load_preset(UTF8_FULL);
    table.set_header(vec!["Score", "Level", "Service", "Message", "Time"]);

    for r in &results {
        let level_colored = match r.level.to_lowercase().as_str() {
            "error" => r.level.red().to_string(),
            "warn" => r.level.yellow().to_string(),
            "info" => r.level.green().to_string(),
            _ => r.level.clone(),
        };

        // Truncate message
        let msg = if r.message.len() > 40 {
            format!("{}...", &r.message[..37])
        } else {
            r.message.clone()
        };

        // Parse and format time
        let time = if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(&r.timestamp) {
            dt.format("%H:%M:%S").to_string()
        } else {
            r.timestamp.clone()
        };

        table.add_row(vec![
            format!("{:.2}", r.score),
            level_colored,
            r.service.clone(),
            msg,
            time,
        ]);
    }

    println!("{table}");
    println!("\n{} {}", "Found:".dimmed(), results.len().to_string().green());

    Ok(())
}

async fn check_status(
    client: &reqwest::Client,
    api_url: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n{}", "🔧 System Status".cyan().bold());
    println!("{}", "─".repeat(40).dimmed());

    // Check API
    print!("  API Server ({})... ", api_url);
    io::stdout().flush()?;

    match client.get(format!("{}/api/search?q=test", api_url)).send().await {
        Ok(resp) if resp.status().is_success() => {
            println!("{}", "✓ Running".green());
        }
        Ok(resp) => {
            println!("{} ({})", "✗ Error".red(), resp.status());
        }
        Err(e) => {
            println!("{} ({})", "✗ Down".red(), e);
        }
    }

    // Check Qdrant
    print!("  Qdrant (localhost:6333)... ");
    io::stdout().flush()?;

    match client.get("http://localhost:6333/collections").send().await {
        Ok(resp) if resp.status().is_success() => {
            println!("{}", "✓ Running".green());
        }
        _ => {
            println!("{}", "✗ Down".red());
        }
    }

    // Check ClickHouse
    print!("  ClickHouse (localhost:8123)... ");
    io::stdout().flush()?;

    match client.get("http://localhost:8123/ping").send().await {
        Ok(resp) if resp.status().is_success() => {
            println!("{}", "✓ Running".green());
        }
        _ => {
            println!("{}", "✗ Down".red());
        }
    }

    // Check NATS
    print!("  NATS (localhost:8222)... ");
    io::stdout().flush()?;

    match client.get("http://localhost:8222/healthz").send().await {
        Ok(resp) if resp.status().is_success() => {
            println!("{}", "✓ Running".green());
        }
        _ => {
            println!("{}", "✗ Down".red());
        }
    }

    println!();
    Ok(())
}

async fn ingest_file(
    client: &reqwest::Client,
    api_url: &str,
    file_path: &str,
    format: &str,
    service: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    use std::fs::File;
    use std::io::{BufRead, BufReader};

    println!("\n{} {}", "📥 Ingesting:".cyan().bold(), file_path);
    println!("{} {}", "Format:".dimmed(), format);
    println!("{} {}", "Service:".dimmed(), service);
    println!("{}", "─".repeat(40).dimmed());

    let file = File::open(file_path)?;
    let reader = BufReader::new(file);
    let lines: Vec<String> = reader
        .lines()
        .filter_map(|l| l.ok())
        .filter(|l| !l.trim().is_empty())
        .collect();

    let total = lines.len();
    println!("Found {} lines to process", total);

    if format == "json" {
        // JSON format: send each line individually
        let pb = indicatif::ProgressBar::new(total as u64);
        pb.set_style(
            indicatif::ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta})")?
                .progress_chars("#>-"),
        );

        let mut success = 0;
        let mut failed = 0;

        for line in &lines {
            let url = format!("{}/api/logs", api_url);
            match client
                .post(&url)
                .header("Content-Type", "application/json")
                .body(line.clone())
                .send()
                .await
            {
                Ok(resp) if resp.status().is_success() => success += 1,
                _ => failed += 1,
            }
            pb.inc(1);
        }

        pb.finish_with_message("Done!");
        println!("\n{}", "Results:".green().bold());
        println!("  {} {}", "Success:".dimmed(), success.to_string().green());
        println!("  {} {}", "Failed:".dimmed(), failed.to_string().red());
    } else {
        // Raw format (apache, nginx, syslog): send all lines in one batch
        println!("Sending {} lines as batch...", total);

        let url = format!("{}/api/logs/raw", api_url);
        let body = serde_json::json!({
            "format": format,
            "service": service,
            "lines": lines
        });

        match client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
        {
            Ok(resp) => {
                if resp.status().is_success() {
                    println!("\n{} Ingested {} logs successfully!", "✓".green().bold(), total);
                } else {
                    let status = resp.status();
                    let text = resp.text().await.unwrap_or_default();
                    println!("\n{} Failed: {} - {}", "✗".red().bold(), status, text);
                }
            }
            Err(e) => {
                println!("\n{} Error: {}", "✗".red().bold(), e);
            }
        }
    }

    Ok(())
}

async fn show_logs(
    client: &reqwest::Client,
    api_url: &str,
    limit: usize,
    level: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let query = level.unwrap_or_else(|| "*".to_string());
    
    println!("\n{}", "📋 Recent Logs".cyan().bold());
    println!("{}", "─".repeat(80).dimmed());

    let url = format!("{}/api/search?q={}&limit={}", api_url, urlencoding::encode(&query), limit);
    let response = client
        .get(&url)
        .send()
        .await?;

    if !response.status().is_success() {
        let error = response.text().await?;
        println!("{} {}", "Error:".red().bold(), error);
        return Ok(());
    }

    let results: Vec<SearchResult> = response.json().await?;

    for r in results {
        let level_colored = match r.level.to_lowercase().as_str() {
            "error" => format!("[{}]", r.level).red().to_string(),
            "warn" => format!("[{}]", r.level).yellow().to_string(),
            "info" => format!("[{}]", r.level).green().to_string(),
            "debug" => format!("[{}]", r.level).blue().to_string(),
            _ => format!("[{}]", r.level),
        };

        let time = if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(&r.timestamp) {
            dt.format("%Y-%m-%d %H:%M:%S").to_string()
        } else {
            r.timestamp.clone()
        };

        println!(
            "{} {} {} {}",
            time.dimmed(),
            level_colored,
            r.service.cyan(),
            r.message
        );
    }

    Ok(())
}

// Response types for stats API
#[derive(Deserialize)]
struct StatsResponse {
    total_logs: u64,
    logs_24h: u64,
    error_count: u64,
    services_count: u64,
    embeddings_count: u64,
    storage_mb: f64,
}

async fn show_stats(
    client: &reqwest::Client,
    api_url: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n{}", "📊 System Statistics".cyan().bold());
    println!("{}", "─".repeat(50).dimmed());

    // Try API first, fallback to direct queries
    let url = format!("{}/api/stats", api_url);
    match client.get(&url).send().await {
        Ok(resp) if resp.status().is_success() => {
            let stats: StatsResponse = resp.json().await?;
            println!("  {} {}", "Total Logs:".dimmed(), stats.total_logs.to_string().green());
            println!("  {} {}", "Logs (24h):".dimmed(), stats.logs_24h.to_string().yellow());
            println!("  {} {}", "Errors:".dimmed(), stats.error_count.to_string().red());
            println!("  {} {}", "Services:".dimmed(), stats.services_count.to_string().cyan());
            println!("  {} {}", "Embeddings:".dimmed(), stats.embeddings_count.to_string().magenta());
            println!("  {} {:.2} MB", "Storage:".dimmed(), stats.storage_mb);
        }
        _ => {
            // Fallback: Query ClickHouse directly
            println!("  {} (querying directly...)", "API unavailable".yellow());
            
            // Get basic counts from ClickHouse
            let ch_url = "http://localhost:8123";
            
            // Total logs
            match client.get(format!("{}/?query=SELECT%20count(*)%20FROM%20logai.logs", ch_url)).send().await {
                Ok(resp) if resp.status().is_success() => {
                    let count = resp.text().await?.trim().to_string();
                    println!("  {} {}", "Total Logs:".dimmed(), count.green());
                }
                _ => println!("  {} {}", "Total Logs:".dimmed(), "N/A".red()),
            }
            
            // Logs last 24h
            match client.get(format!("{}/?query=SELECT%20count(*)%20FROM%20logai.logs%20WHERE%20timestamp%20%3E%20now()%20-%20INTERVAL%201%20DAY", ch_url)).send().await {
                Ok(resp) if resp.status().is_success() => {
                    let count = resp.text().await?.trim().to_string();
                    println!("  {} {}", "Logs (24h):".dimmed(), count.yellow());
                }
                _ => println!("  {} {}", "Logs (24h):".dimmed(), "N/A".red()),
            }
            
            // Error count
            match client.get(format!("{}/?query=SELECT%20count(*)%20FROM%20logai.logs%20WHERE%20level%20%3D%20%27Error%27", ch_url)).send().await {
                Ok(resp) if resp.status().is_success() => {
                    let count = resp.text().await?.trim().to_string();
                    println!("  {} {}", "Errors:".dimmed(), count.red());
                }
                _ => println!("  {} {}", "Errors:".dimmed(), "N/A".red()),
            }
            
            // Unique services
            match client.get(format!("{}/?query=SELECT%20count(DISTINCT%20service)%20FROM%20logai.logs", ch_url)).send().await {
                Ok(resp) if resp.status().is_success() => {
                    let count = resp.text().await?.trim().to_string();
                    println!("  {} {}", "Services:".dimmed(), count.cyan());
                }
                _ => println!("  {} {}", "Services:".dimmed(), "N/A".red()),
            }
            
            // Qdrant embeddings count
            match client.get("http://localhost:6333/collections/log_embeddings").send().await {
                Ok(resp) if resp.status().is_success() => {
                    let body: serde_json::Value = resp.json().await?;
                    if let Some(count) = body["result"]["points_count"].as_u64() {
                        println!("  {} {}", "Embeddings:".dimmed(), count.to_string().magenta());
                    }
                }
                _ => println!("  {} {}", "Embeddings:".dimmed(), "N/A".red()),
            }
        }
    }

    println!();
    Ok(())
}

fn start_server(port: u16) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n{}", "🚀 Starting LogAI API Server...".cyan().bold());
    println!("{}", "─".repeat(40).dimmed());
    println!("  {} {}", "Port:".dimmed(), port.to_string().green());
    println!("  {} http://localhost:{}", "URL:".dimmed(), port);
    println!();
    println!("{}", "Press Ctrl+C to stop".dimmed());
    println!();

    // Find the logai-api binary
    let binary = std::env::current_exe()?
        .parent()
        .map(|p| p.join("logai-api"))
        .unwrap_or_else(|| std::path::PathBuf::from("./target/release/logai-api"));

    if !binary.exists() {
        println!("{} logai-api binary not found at {:?}", "Error:".red().bold(), binary);
        println!("Run: cargo build --release");
        return Ok(());
    }

    // Start the API server
    let status = ProcessCommand::new(&binary)
        .env("PORT", port.to_string())
        .status()?;

    if !status.success() {
        println!("{} Server exited with status: {}", "Error:".red().bold(), status);
    }

    Ok(())
}

#[derive(Deserialize)]
struct AlertResponse {
    alerts: Vec<AlertItem>,
}

#[derive(Deserialize)]
struct AlertItem {
    id: String,
    service: String,
    severity: String,
    message: String,
    status: String,
    fired_at: String,
}

async fn show_alerts(
    client: &reqwest::Client,
    api_url: &str,
    status_filter: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n{}", "🚨 Active Alerts".cyan().bold());
    println!("{}", "─".repeat(60).dimmed());

    // Try API first
    let url = match &status_filter {
        Some(s) => format!("{}/api/alerts?status={}", api_url, s),
        None => format!("{}/api/alerts", api_url),
    };

    match client.get(&url).send().await {
        Ok(resp) if resp.status().is_success() => {
            let data: AlertResponse = resp.json().await?;
            
            if data.alerts.is_empty() {
                println!("  {} No active alerts", "✓".green());
            } else {
                let mut table = Table::new();
                table.load_preset(UTF8_FULL);
                table.set_header(vec!["Status", "Severity", "Service", "Message", "Time"]);

                for alert in &data.alerts {
                    let status_colored = match alert.status.as_str() {
                        "firing" => "🔥 FIRING".red().to_string(),
                        "acknowledged" => "👁 ACK".yellow().to_string(),
                        _ => alert.status.clone(),
                    };

                    let severity_colored = match alert.severity.to_lowercase().as_str() {
                        "critical" => alert.severity.red().bold().to_string(),
                        "warning" => alert.severity.yellow().to_string(),
                        _ => alert.severity.clone(),
                    };

                    let msg = if alert.message.len() > 35 {
                        format!("{}...", &alert.message[..32])
                    } else {
                        alert.message.clone()
                    };

                    table.add_row(vec![
                        status_colored,
                        severity_colored,
                        alert.service.clone(),
                        msg,
                        alert.fired_at.clone(),
                    ]);
                }

                println!("{table}");
                println!("\n{} {} alerts", "Total:".dimmed(), data.alerts.len().to_string().yellow());
            }
        }
        _ => {
            // No API endpoint yet - show message
            println!("  {} Alert API not available", "⚠".yellow());
            println!();
            println!("  Run the anomaly runner to detect alerts:");
            println!("  {}", "RUST_LOG=info cargo run -p logai-anomaly --bin anomaly-runner".dimmed());
        }
    }

    println!();
    Ok(())
}

#[derive(Deserialize)]
struct AnomalyResponse {
    anomalies: Vec<AnomalyItem>,
    checked_at: String,
}

#[derive(Deserialize)]
struct AnomalyItem {
    service: String,
    rule: String,
    severity: String,
    message: String,
    current_value: f64,
    expected_value: f64,
}

async fn check_anomalies(
    client: &reqwest::Client,
    api_url: &str,
    service_filter: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n{}", "🔎 Anomaly Detection".cyan().bold());
    println!("{}", "─".repeat(60).dimmed());

    // Try API
    let url = match &service_filter {
        Some(s) => format!("{}/api/anomalies?service={}", api_url, s),
        None => format!("{}/api/anomalies", api_url),
    };

    match client.get(&url).send().await {
        Ok(resp) if resp.status().is_success() => {
            let data: AnomalyResponse = resp.json().await?;
            
            println!("  {} {}", "Checked at:".dimmed(), data.checked_at);
            println!();

            if data.anomalies.is_empty() {
                println!("  {} No anomalies detected", "✓".green());
            } else {
                for anomaly in &data.anomalies {
                    let severity_icon = match anomaly.severity.to_lowercase().as_str() {
                        "critical" => "🔴",
                        "warning" => "🟡",
                        _ => "🔵",
                    };

                    println!(
                        "  {} {} {} {}",
                        severity_icon,
                        format!("[{}]", anomaly.severity).red(),
                        anomaly.service.cyan(),
                        anomaly.rule.dimmed()
                    );
                    println!("     {}", anomaly.message);
                    println!(
                        "     {} current={:.1} expected={:.1}",
                        "→".dimmed(),
                        anomaly.current_value,
                        anomaly.expected_value
                    );
                    println!();
                }
                println!("{} {} anomalies found", "Total:".dimmed(), data.anomalies.len().to_string().red());
            }
        }
        _ => {
            // No API - show how to run detection
            println!("  {} Anomaly API not available", "⚠".yellow());
            println!();
            println!("  To check anomalies, ensure the API has /api/anomalies endpoint");
            println!("  or run: {}", "cargo test -p logai-anomaly".dimmed());
        }
    }

    println!();
    Ok(())
}

// Chat types
#[derive(Serialize)]
struct ChatRequest {
    session_id: String,
    message: String,
    history: Vec<ChatHistoryMessage>,
}

#[derive(Serialize)]
struct ChatHistoryMessage {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct ChatResponse {
    answer: String,
    sources_count: usize,
    response_time_ms: u128,
    provider: String,
    context_logs: usize,
    conversation_turn: usize,
    #[serde(default)]
    source_logs: Vec<String>,
}

/// Interactive chat mode - the core debugging experience
async fn interactive_chat(
    client: &reqwest::Client,
    api_url: &str,
    initial_question: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    // Generate unique session ID
    let session_id = uuid::Uuid::new_v4().to_string();
    
    // Print banner
    println!();
    println!("{}", "╔════════════════════════════════════════════════════════════════╗".cyan());
    println!("{}", "║           🤖 LogAI Interactive Debugging Chat                  ║".cyan().bold());
    println!("{}", "╠════════════════════════════════════════════════════════════════╣".cyan());
    println!("{}", "║  Ask questions about your logs in natural language.            ║".cyan());
    println!("{}", "║  The AI remembers conversation context for follow-up queries.  ║".cyan());
    println!("{}", "║                                                                ║".cyan());
    println!("{}", "║  Commands:                                                     ║".cyan());
    println!("{}", "║    /help     - Show available commands                         ║".cyan());
    println!("{}", "║    /clear    - Clear conversation history                      ║".cyan());
    println!("{}", "║    /logs     - Show source logs from last query                ║".cyan());
    println!("{}", "║    /tips     - Show debugging tips                             ║".cyan());
    println!("{}", "║    /status   - Show system status                              ║".cyan());
    println!("{}", "║    /exit     - Exit chat                                       ║".cyan());
    println!("{}", "╚════════════════════════════════════════════════════════════════╝".cyan());
    println!();
    println!("{} {}", "Session:".dimmed(), session_id[..8].yellow());
    println!();

    // Track conversation
    let mut conversation_history: Vec<(String, String)> = Vec::new();
    let mut last_sources = 0usize;
    let mut last_source_logs: Vec<String> = Vec::new();

    // Handle initial question if provided
    if let Some(ref q) = initial_question {
        process_chat_message(client, api_url, &session_id, q, &mut conversation_history, &mut last_sources, &mut last_source_logs).await?;
    }

    // REPL loop
    let stdin = io::stdin();
    loop {
        // Print prompt
        print!("{} ", "You:".green().bold());
        io::stdout().flush()?;

        // Read input
        let mut input = String::new();
        stdin.lock().read_line(&mut input)?;
        let input = input.trim();

        // Skip empty
        if input.is_empty() {
            continue;
        }

        // Handle commands
        if input.starts_with('/') {
            match input.to_lowercase().as_str() {
                "/exit" | "/quit" | "/q" => {
                    println!("\n{}", "Goodbye! Happy debugging! 👋".cyan());
                    break;
                }
                "/help" | "/h" => {
                    println!();
                    println!("{}", "Available Commands:".yellow().bold());
                    println!("  {}  - Show this help", "/help".cyan());
                    println!("  {} - Start fresh conversation", "/clear".cyan());
                    println!("  {}  - Show example questions", "/tips".cyan());
                    println!("  {}  - Exit chat", "/exit".cyan());
                    println!();
                    println!("{}", "Example Questions:".yellow().bold());
                    println!("  • {}", "What errors happened in the last hour?".dimmed());
                    println!("  • {}", "Are there any timeout issues in payment-service?".dimmed());
                    println!("  • {}", "What's causing the database connection failures?".dimmed());
                    println!("  • {}", "Show me the error pattern for nginx".dimmed());
                    println!("  • {}", "Why is auth-service failing?".dimmed());
                    println!();
                }
                "/clear" | "/new" => {
                    conversation_history.clear();
                    println!("\n{} Starting fresh conversation.\n", "✓".green());
                }
                "/tips" => {
                    println!();
                    println!("{}", "🎯 Debugging Tips:".yellow().bold());
                    println!();
                    println!("  {}", "1. Start broad, then narrow down:".cyan());
                    println!("     \"What are the main errors today?\"");
                    println!("     → \"Tell me more about the timeout errors\"");
                    println!("     → \"What service is causing most timeouts?\"");
                    println!();
                    println!("  {}", "2. Ask for root causes:".cyan());
                    println!("     \"What's the root cause of these connection failures?\"");
                    println!();
                    println!("  {}", "3. Request correlations:".cyan());
                    println!("     \"Are payment errors related to database issues?\"");
                    println!();
                    println!("  {}", "4. Ask for fixes:".cyan());
                    println!("     \"How can I fix these timeout errors?\"");
                    println!();
                }
                "/status" => {
                    check_status(client, api_url).await?;
                }
                "/logs" => {
                    if last_source_logs.is_empty() {
                        println!("\n{} No logs from last query.\n", "⚠".yellow());
                    } else {
                        println!();
                        println!("{}", "📋 Source Logs:".yellow().bold());
                        println!("{}", "─".repeat(60).dimmed());
                        for (i, log) in last_source_logs.iter().enumerate() {
                            println!("{} {}", format!("[{}]", i + 1).dimmed(), log);
                        }
                        println!("{}", "─".repeat(60).dimmed());
                        println!();
                    }
                }
                _ => {
                    println!("{} Unknown command. Type {} for help.", "⚠".yellow(), "/help".cyan());
                }
            }
            continue;
        }

        // Process as chat message
        process_chat_message(client, api_url, &session_id, input, &mut conversation_history, &mut last_sources, &mut last_source_logs).await?;
    }

    Ok(())
}

async fn process_chat_message(
    client: &reqwest::Client,
    api_url: &str,
    session_id: &str,
    message: &str,
    history: &mut Vec<(String, String)>,
    last_sources: &mut usize,
    last_source_logs: &mut Vec<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    println!();
    println!("{}", "Thinking...".dimmed());

    let url = format!("{}/api/chat", api_url);
    
    // Build history messages
    let mut history_messages = Vec::new();
    for (q, a) in history.iter() {
        history_messages.push(ChatHistoryMessage {
            role: "user".to_string(),
            content: q.clone(),
        });
        history_messages.push(ChatHistoryMessage {
            role: "assistant".to_string(),
            content: a.clone(),
        });
    }
    
    let request_body = ChatRequest {
        session_id: session_id.to_string(),
        message: message.to_string(),
        history: history_messages,
    };

    let start = std::time::Instant::now();
    
    let response = client
        .post(&url)
        .header("Content-Type", "application/json")
        .json(&request_body)
        .send()
        .await;

    match response {
        Ok(resp) if resp.status().is_success() => {
            let result: ChatResponse = resp.json().await?;
            let elapsed = start.elapsed().as_millis();

            // Clear "Thinking..." and print answer
            print!("\x1B[1A\x1B[2K"); // Move up and clear line
            
            println!("{}", "─".repeat(60).dimmed());
            println!("{}", "AI:".cyan().bold());
            println!();
            
            // Print answer with word wrapping
            print_wrapped(&result.answer, 70);
            
            println!();
            println!("{}", "─".repeat(60).dimmed());
            println!(
                "{} {} | {} {} | {} {}ms | {} {}",
                "Sources:".dimmed(),
                result.sources_count.to_string().yellow(),
                "Turn:".dimmed(),
                result.conversation_turn.to_string().cyan(),
                "Time:".dimmed(),
                elapsed.to_string().yellow(),
                "Provider:".dimmed(),
                result.provider.magenta()
            );
            println!();

            // Store in history
            history.push((message.to_string(), result.answer.clone()));
            *last_sources = result.sources_count;
            *last_source_logs = result.source_logs.clone();

            // Keep history manageable (last 10 turns)
            if history.len() > 10 {
                history.remove(0);
            }
        }
        Ok(resp) => {
            print!("\x1B[1A\x1B[2K");
            let status = resp.status();
            let error = resp.text().await.unwrap_or_default();
            
            if status.as_u16() == 404 {
                // Fallback to /api/ask if /api/chat not available
                println!("{} Chat API not available, using single-query mode.", "⚠".yellow());
                println!("{}", "─".repeat(60).dimmed());
                
                // Use ask endpoint as fallback
                let ask_url = format!("{}/api/ask?q={}", api_url, urlencoding::encode(message));
                match client.get(&ask_url).send().await {
                    Ok(ask_resp) if ask_resp.status().is_success() => {
                        let result: AskResponse = ask_resp.json().await?;
                        println!("{}", "AI:".cyan().bold());
                        println!();
                        print_wrapped(&result.answer, 70);
                        println!();
                        println!("{}", "─".repeat(60).dimmed());
                        println!(
                            "{} {} | {} {}ms",
                            "Sources:".dimmed(),
                            result.sources_count.to_string().yellow(),
                            "Time:".dimmed(),
                            result.response_time_ms.to_string().yellow()
                        );
                        println!();
                        
                        history.push((message.to_string(), result.answer.clone()));
                    }
                    _ => {
                        println!("{} Could not get response.", "Error:".red().bold());
                    }
                }
            } else {
                println!("{} {} - {}", "Error:".red().bold(), status, error);
            }
        }
        Err(e) => {
            print!("\x1B[1A\x1B[2K");
            println!("{} Connection failed: {}", "Error:".red().bold(), e);
            println!("{}", "Make sure the API server is running (logai serve)".dimmed());
        }
    }

    Ok(())
}

/// Print text with word wrapping
fn print_wrapped(text: &str, width: usize) {
    for line in text.lines() {
        if line.len() <= width {
            println!("  {}", line);
        } else {
            // Word wrap
            let mut current_line = String::new();
            for word in line.split_whitespace() {
                if current_line.len() + word.len() + 1 > width {
                    if !current_line.is_empty() {
                        println!("  {}", current_line);
                        current_line.clear();
                    }
                }
                if !current_line.is_empty() {
                    current_line.push(' ');
                }
                current_line.push_str(word);
            }
            if !current_line.is_empty() {
                println!("  {}", current_line);
            }
        }
    }
}
