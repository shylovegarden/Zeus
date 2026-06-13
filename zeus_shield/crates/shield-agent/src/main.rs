use clap::Parser;
use shield_core::config::ShieldConfig;
use shield_core::traits::Scanner;
use shield_scanner::{NetworkScanner, DeviceScanner};
use std::path::PathBuf;
use tracing::{info, warn};
use uuid::Uuid;

#[derive(Parser)]
#[command(name = "zeus-shield-agent")]
#[command(about = "Zeus Shield Security Agent")]
struct Cli {
    /// Configuration file path
    #[arg(short, long, default_value = "/etc/zeus-shield/agent.toml")]
    config: PathBuf,

    /// Console URL to report to
    #[arg(short = 'u', long)]
    console_url: Option<String>,

    /// Run a single scan and exit (no daemon mode)
    #[arg(long)]
    once: bool,

    /// Scan target (host or CIDR)
    #[arg(short, long)]
    target: Option<String>,

    /// Verbosity level
    #[arg(short, long, default_value = "info")]
    log_level: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    info!("Zeus Shield Agent v{}", env!("CARGO_PKG_VERSION"));
    info!("Agent ID: {}", Uuid::new_v4());

    if cli.once {
        run_single_scan(&cli).await?;
    } else {
        run_daemon(&cli).await?;
    }

    Ok(())
}

async fn run_single_scan(cli: &Cli) -> Result<(), Box<dyn std::error::Error>> {
    info!("Running single scan...");

    // Device scan (local system)
    let device_scanner = DeviceScanner::new();
    let targets = device_scanner.discover().await?;
    
    for target in &targets {
        info!("Scanning device: {} ({})", target.name, target.address);
        let vulns = device_scanner.scan(target).await?;
        for vuln in &vulns {
            warn!(
                "[{:?}] {} - {}",
                vuln.severity, vuln.title, vuln.description
            );
        }
        info!("Device scan complete: {} findings", vulns.len());
    }

    // Network scan (if target specified)
    if let Some(target_addr) = &cli.target {
        let network_scanner = NetworkScanner::new()
            .with_port_range(1, 1024)
            .with_concurrency(200);

        let target = shield_core::types::Target {
            id: Uuid::new_v4(),
            name: target_addr.clone(),
            kind: shield_core::types::TargetKind::Host,
            address: target_addr.clone(),
            metadata: serde_json::json!({}),
            created_at: chrono::Utc::now(),
        };

        info!("Network scanning: {}", target_addr);
        let vulns = network_scanner.scan(&target).await?;
        for vuln in &vulns {
            warn!(
                "[{:?}] {} - {}",
                vuln.severity, vuln.title, vuln.description
            );
        }
        info!("Network scan complete: {} findings", vulns.len());
    }

    Ok(())
}

async fn run_daemon(cli: &Cli) -> Result<(), Box<dyn std::error::Error>> {
    info!("Starting daemon mode...");
    info!("Heartbeat interval: 30s");
    
    let console_url = cli.console_url.clone()
        .unwrap_or_else(|| "http://localhost:8443".to_string());
    
    info!("Reporting to console: {}", console_url);

    loop {
        // Heartbeat
        info!("Sending heartbeat...");
        
        // TODO: Send heartbeat to console
        // TODO: Check for pending scan jobs
        // TODO: Execute assigned scans
        // TODO: Report results
        
        tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
    }
}
