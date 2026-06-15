mod reporter;

use clap::Parser;
use reporter::ConsoleReporter;
use shield_core::traits::Scanner;
use shield_core::types::{AgentStatus, Target, TargetKind};
use shield_scanner::{NetworkScanner, DeviceScanner};
use std::path::PathBuf;
use chrono;
use tracing::{info, warn};
use uuid::Uuid;

#[derive(Parser)]
#[command(name = "zeus-shield-agent")]
#[command(about = "Zeus Shield Security Agent — collects findings and reports to console")]
struct Cli {
    /// Configuration file path
    #[arg(short, long, default_value = "/etc/zeus-shield/agent.toml")]
    config: PathBuf,

    /// Console URL to report findings to
    #[arg(short = 'u', long, default_value = "http://localhost:8443")]
    console_url: String,

    /// Run a single scan and exit (no daemon mode)
    #[arg(long)]
    once: bool,

    /// Scan target host/IP (for network scan)
    #[arg(short, long)]
    target: Option<String>,

    /// Port range for network scan (e.g. 1-1024)
    #[arg(short, long, default_value = "1-1024")]
    ports: String,

    /// Heartbeat interval in seconds
    #[arg(long, default_value = "30")]
    interval: u64,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    let agent_id = Uuid::new_v4();
    info!("Zeus Shield Agent v{}", env!("CARGO_PKG_VERSION"));
    info!("Agent ID: {}", agent_id);
    info!("Console: {}", cli.console_url);

    let reporter = ConsoleReporter::new(&cli.console_url, agent_id);

    if cli.once {
        run_single_scan(&cli, &reporter).await?;
    } else {
        run_daemon(&cli, reporter).await?;
    }

    Ok(())
}

async fn run_single_scan(
    cli: &Cli,
    reporter: &ConsoleReporter,
) -> Result<(), Box<dyn std::error::Error>> {
    info!("Running single scan...");
    reporter.send_heartbeat(AgentStatus::Scanning).await;

    let mut all_vulns = Vec::new();

    // Device scan (always runs on local system)
    let device_scanner = DeviceScanner::new();
    let local_targets = device_scanner.discover().await?;
    for target in &local_targets {
        info!("Device scan: {} ({})", target.name, target.address);
        match device_scanner.scan(target).await {
            Ok(vulns) => {
                info!("Device scan: {} findings", vulns.len());
                for v in &vulns {
                    warn!("[{:?}] {}", v.severity, v.title);
                }
                all_vulns.extend(vulns);
            }
            Err(e) => warn!("Device scan error: {}", e),
        }
    }

    // Network scan (if --target specified)
    if let Some(target_addr) = &cli.target {
        let (start, end) = parse_port_range(&cli.ports);
        let network_scanner = NetworkScanner::new()
            .with_port_range(start, end)
            .with_concurrency(256);

        let target = shield_core::types::Target {
            id: Uuid::new_v4(),
            name: target_addr.clone(),
            kind: TargetKind::Host,
            address: target_addr.clone(),
            metadata: serde_json::json!({}),
            created_at: chrono::Utc::now(),
        };

        info!("Network scan: {} ports {}-{}", target_addr, start, end);
        match network_scanner.scan(&target).await {
            Ok(vulns) => {
                info!("Network scan: {} findings", vulns.len());
                for v in &vulns {
                    warn!("[{:?}] {}", v.severity, v.title);
                }
                all_vulns.extend(vulns);
            }
            Err(e) => warn!("Network scan error: {}", e),
        }
    }

    // Report all findings back to console
    reporter.report_vulnerabilities(&all_vulns).await;
    reporter.send_heartbeat(AgentStatus::Online).await;

    info!("Scan complete. Total findings: {}", all_vulns.len());
    Ok(())
}

async fn run_daemon(
    cli: &Cli,
    reporter: ConsoleReporter,
) -> Result<(), Box<dyn std::error::Error>> {
    info!("Daemon mode — heartbeat every {}s", cli.interval);

    let mut tick = tokio::time::interval(
        tokio::time::Duration::from_secs(cli.interval)
    );

    // Initial scan on startup
    run_single_scan(cli, &reporter).await?;

    loop {
        tick.tick().await;

        // Send heartbeat
        reporter.send_heartbeat(AgentStatus::Online).await;

        // Check for console-assigned jobs
        let jobs = reporter.fetch_pending_jobs().await;
        if !jobs.is_empty() {
            info!("{} pending jobs from console", jobs.len());
            for job in jobs {
                execute_job(&reporter, &cli, &job).await;
            }
        }
    }
}

/// Execute a single console-assigned scan job
async fn execute_job(reporter: &ConsoleReporter, cli: &Cli, job: &serde_json::Value) {
    let job_id = job.get("id").and_then(|v| v.as_str()).unwrap_or("unknown");
    let job_type = job.get("scan_type").and_then(|v| v.as_str()).unwrap_or("network");
    let target_addr = job.get("target")
        .and_then(|v| v.as_str())
        .unwrap_or("localhost");

    info!("Executing job {} type={} target={}", job_id, job_type, target_addr);

    let target = Target {
        id: Uuid::new_v4(),
        name: target_addr.to_string(),
        kind: match job_type {
            "device" => TargetKind::Host,
            "network" => TargetKind::Network,
            _ => TargetKind::Host,
        },
        address: target_addr.to_string(),
        metadata: serde_json::json!({"job_id": job_id}),
        created_at: chrono::Utc::now(),
    };

    let vulns = match job_type {
        "device" => {
            let scanner = DeviceScanner::new();
            match scanner.scan(&target).await {
                Ok(v) => v,
                Err(e) => {
                    warn!("Device scan failed for job {}: {}", job_id, e);
                    return;
                }
            }
        }
        _ => {
            // Default: network scan
            let scanner = NetworkScanner::new();
            match scanner.scan(&target).await {
                Ok(v) => v,
                Err(e) => {
                    warn!("Network scan failed for job {}: {}", job_id, e);
                    return;
                }
            }
        }
    };

    info!("Job {} complete: {} findings", job_id, vulns.len());
    reporter.report_vulnerabilities(&vulns).await;
}

fn parse_port_range(s: &str) -> (u16, u16) {
    let parts: Vec<&str> = s.split('-').collect();
    match parts.as_slice() {
        [start, end] => (
            start.parse().unwrap_or(1),
            end.parse().unwrap_or(1024),
        ),
        _ => (1, 1024),
    }
}
