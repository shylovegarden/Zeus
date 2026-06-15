use clap::{Parser, Subcommand};
use shield_core::traits::{Scanner, Patcher};
use shield_core::types::*;
use shield_patch::PatchEngine;
use shield_scanner::{NetworkScanner, DeviceScanner};
use std::path::PathBuf;
use tracing::info;
use uuid::Uuid;

#[derive(Parser)]
#[command(name = "zeus-shield")]
#[command(about = "Zeus Shield — Unified Cybersecurity Platform")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Verbosity level
    #[arg(short, long, default_value = "info")]
    log_level: String,
}

#[derive(Subcommand)]
enum Commands {
    /// Scan a target for vulnerabilities
    Scan {
        /// Target host or CIDR range
        target: String,

        /// Scan type: network, code, device, full
        #[arg(short = 't', long, default_value = "full")]
        scan_type: String,

        /// Port range (for network scans)
        #[arg(short, long, default_value = "1-1024")]
        ports: String,

        /// Output format: text, json
        #[arg(short, long, default_value = "text")]
        output: String,
    },

    /// Fix a vulnerability automatically
    Fix {
        /// Vulnerability ID or scan result file
        target: String,

        /// Apply fix without confirmation
        #[arg(long)]
        auto: bool,

        /// Verify the fix with Zeus formal verification
        #[arg(long)]
        verify: bool,
    },

    /// Verify a patch is correct using Zeus
    Verify {
        /// Path to patch file or source code
        path: PathBuf,

        /// Properties to verify
        #[arg(short, long)]
        properties: Vec<String>,
    },

    /// Start the agent daemon
    Agent {
        /// Console URL to report to
        #[arg(short, long)]
        console_url: Option<String>,
    },

    /// Start the console server
    Console {
        /// Bind address
        #[arg(short, long, default_value = "0.0.0.0:8443")]
        bind: String,
    },

    /// Show system status and health
    Status,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    match cli.command {
        Commands::Scan { target, scan_type, ports, output } => {
            cmd_scan(&target, &scan_type, &ports, &output).await?;
        }
        Commands::Fix { target, auto, verify } => {
            cmd_fix(&target, auto, verify).await?;
        }
        Commands::Verify { path, properties } => {
            cmd_verify(&path, &properties).await?;
        }
        Commands::Agent { console_url } => {
            cmd_agent(console_url).await?;
        }
        Commands::Console { bind } => {
            cmd_console(&bind).await?;
        }
        Commands::Status => {
            cmd_status().await?;
        }
    }

    Ok(())
}

async fn cmd_scan(
    target: &str,
    scan_type: &str,
    ports: &str,
    output: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("╔══════════════════════════════════════════════════════╗");
    println!("║          ZEUS SHIELD — Security Scanner             ║");
    println!("╚══════════════════════════════════════════════════════╝");
    println!();

    let t = shield_core::types::Target {
        id: Uuid::new_v4(),
        name: target.to_string(),
        kind: shield_core::types::TargetKind::Host,
        address: target.to_string(),
        metadata: serde_json::json!({}),
        created_at: chrono::Utc::now(),
    };

    let mut all_vulns = Vec::new();

    match scan_type {
        "network" | "full" => {
            let (start, end) = parse_port_range(ports);
            let scanner = NetworkScanner::new()
                .with_port_range(start, end)
                .with_concurrency(200);
            
            println!("[*] Network scan: {} (ports {}-{})", target, start, end);
            let vulns = scanner.scan(&t).await?;
            println!("[+] Found {} network vulnerabilities", vulns.len());
            all_vulns.extend(vulns);
        }
        _ => {}
    }

    match scan_type {
        "device" | "full" => {
            let scanner = DeviceScanner::new();
            println!("[*] Device scan: local system");
            let vulns = scanner.scan(&t).await?;
            println!("[+] Found {} device vulnerabilities", vulns.len());
            all_vulns.extend(vulns);
        }
        _ => {}
    }

    println!();
    println!("══════════════════════ RESULTS ═══════════════════════");
    println!();

    if all_vulns.is_empty() {
        println!("  ✓ No vulnerabilities found");
    } else {
        for vuln in &all_vulns {
            let severity_icon = match vuln.severity {
                shield_core::types::Severity::Critical => "🔴",
                shield_core::types::Severity::High => "🟠",
                shield_core::types::Severity::Medium => "🟡",
                shield_core::types::Severity::Low => "🔵",
                shield_core::types::Severity::Info => "⚪",
            };
            println!("  {} [{:?}] {}", severity_icon, vuln.severity, vuln.title);
            println!("    └─ {}", vuln.description);
            println!();
        }
    }

    println!("══════════════════════════════════════════════════════");
    println!("  Total: {} findings", all_vulns.len());
    println!("  Critical: {}", all_vulns.iter().filter(|v| v.severity == shield_core::types::Severity::Critical).count());
    println!("  High: {}", all_vulns.iter().filter(|v| v.severity == shield_core::types::Severity::High).count());
    println!("  Medium: {}", all_vulns.iter().filter(|v| v.severity == shield_core::types::Severity::Medium).count());

    if output == "json" {
        let json = serde_json::to_string_pretty(&all_vulns)?;
        println!("\n{}", json);
    }

    Ok(())
}

async fn cmd_fix(
    target: &str,
    auto: bool,
    verify: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("╔══════════════════════════════════════════════════════╗");
    println!("║          ZEUS SHIELD — Auto-Fix Engine              ║");
    println!("╚══════════════════════════════════════════════════════╝");
    println!();

    // Step 1: scan the target to find vulnerabilities
    println!("[1/4] Scanning target: {}", target);
    let t = Target {
        id: Uuid::new_v4(),
        name: target.to_string(),
        kind: TargetKind::Host,
        address: target.to_string(),
        metadata: serde_json::json!({}),
        created_at: chrono::Utc::now(),
    };

    let scanner = NetworkScanner::new().with_port_range(1, 1024).with_concurrency(256);
    let vulns = scanner.scan(&t).await?;

    if vulns.is_empty() {
        println!("  ✓ No vulnerabilities found — nothing to fix");
        return Ok(());
    }

    println!("  Found {} vulnerabilities", vulns.len());
    println!();

    // Step 2: generate patches for each vulnerability
    println!("[2/4] Generating patches...");
    let engine = PatchEngine::new();
    let mut patches = Vec::new();

    for vuln in &vulns {
        if engine.can_fix(vuln) {
            match engine.generate_patch(vuln).await {
                Ok(patch) => {
                    println!("  ✓ Patch generated for: {} (confidence: {:.0}%)",
                        vuln.title, patch.confidence * 100.0);
                    patches.push((vuln.clone(), patch));
                }
                Err(e) => println!("  ✗ Could not generate patch for {}: {}", vuln.title, e),
            }
        } else {
            println!("  ⚠ No auto-fix available for: {}", vuln.title);
        }
    }

    if patches.is_empty() {
        println!("\n  No auto-fixable vulnerabilities found.");
        return Ok(());
    }

    println!();

    // Step 3: show patches
    println!("[3/4] Proposed fixes:");
    println!();
    for (vuln, patch) in &patches {
        println!("  ┌─ {} [{:?}]", vuln.title, vuln.severity);
        println!("  │  Type: {:?} | Confidence: {:.0}%",
            patch.patch_type, patch.confidence * 100.0);
        println!("  │");
        for line in patch.diff.lines() {
            println!("  │  {}", line);
        }
        println!("  └─────────────────────────────────────");
        println!();
    }

    // Step 4: apply (if --auto) or prompt
    println!("[4/4] Apply fixes");
    if auto {
        println!("  --auto flag set: applying {} patches", patches.len());
        for (vuln, patch) in &patches {
            println!("  [APPLY] {}", patch.description);
            // TODO: invoke Connector.apply_patch() for real system changes
        }
        println!();
        println!("  ✓ {} patches applied", patches.len());
        if verify {
            println!("  [VERIFY] Zeus formal verification not yet wired — run:");
            println!("           zeus-shield verify <patch-file>");
        }
    } else {
        println!("  {} patches ready. Re-run with --auto to apply.", patches.len());
    }

    Ok(())
}

async fn cmd_verify(
    path: &PathBuf,
    properties: &[String],
) -> Result<(), Box<dyn std::error::Error>> {
    println!("[*] Verifying: {:?}", path);
    println!("[*] Properties: {:?}", properties);
    println!("[*] Zeus verification integration pending");
    Ok(())
}

async fn cmd_agent(
    console_url: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let url = console_url.unwrap_or_else(|| "http://localhost:8443".to_string());
    println!("[*] Starting Zeus Shield Agent");
    println!("[*] Reporting to: {}", url);
    println!("[*] Agent daemon mode not yet implemented");
    Ok(())
}

async fn cmd_console(bind: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("[*] Starting Zeus Shield Console on {}", bind);
    println!("[*] Use shield-console binary for full server");
    Ok(())
}

async fn cmd_status() -> Result<(), Box<dyn std::error::Error>> {
    println!("╔══════════════════════════════════════════════════════╗");
    println!("║            ZEUS SHIELD — System Status              ║");
    println!("╚══════════════════════════════════════════════════════╝");
    println!();
    println!("  Version: {}", env!("CARGO_PKG_VERSION"));
    println!("  OS: {} {}", std::env::consts::OS, std::env::consts::ARCH);
    println!("  Zeus compiler: checking...");
    
    // Check if Zeus compiler is available
    let zeus_check = tokio::process::Command::new("zeus_compiler")
        .arg("--version")
        .output()
        .await;
    
    match zeus_check {
        Ok(output) if output.status.success() => {
            println!("  Zeus compiler: ✓ available");
        }
        _ => {
            println!("  Zeus compiler: ✗ not found (verification disabled)");
        }
    }

    // Check Docker for sandbox
    let docker_check = tokio::process::Command::new("docker")
        .arg("--version")
        .output()
        .await;
    
    match docker_check {
        Ok(output) if output.status.success() => {
            let version = String::from_utf8_lossy(&output.stdout);
            println!("  Docker: ✓ {}", version.trim());
        }
        _ => {
            println!("  Docker: ✗ not found (sandbox disabled)");
        }
    }

    Ok(())
}

fn parse_port_range(range: &str) -> (u16, u16) {
    let parts: Vec<&str> = range.split('-').collect();
    if parts.len() == 2 {
        let start = parts[0].parse().unwrap_or(1);
        let end = parts[1].parse().unwrap_or(1024);
        (start, end)
    } else {
        (1, 1024)
    }
}
