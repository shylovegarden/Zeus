use clap::{Parser, Subcommand};
use shield_core::traits::{Scanner, Patcher, Verifier};
use shield_core::types::*;
use shield_patch::PatchEngine;
use shield_scanner::{NetworkScanner, DeviceScanner};
use shield_verify::ZeusVerifier;
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

    // Step 4: verify + apply
    println!("[4/4] Apply fixes");
    if auto {
        // Verify each patch before applying
        if verify {
            println!("  [VERIFY] Running Zeus verification on patches...");
            let zeus_path = which_zeus();
            let verifier = ZeusVerifier::new(zeus_path);
            for (_vuln, patch) in &mut patches {
                match verifier.verify(patch).await {
                    Ok(cert) => {
                        println!("  ✓ Verified: {} — properties: {}",
                            patch.description,
                            cert.properties_proven.join(", ")
                        );
                        patch.verified = true;
                        patch.certificate = Some(cert);
                    }
                    Err(e) => {
                        println!("  ✗ Verification failed for {}: {}", patch.description, e);
                        println!("    Skipping this patch.");
                        continue;
                    }
                }
            }
            println!();
        }

        println!("  Applying {} patches...", patches.len());
        let mut applied = 0;
        for (_vuln, patch) in &patches {
            if verify && !patch.verified {
                continue;
            }
            match apply_patch_to_system(patch).await {
                Ok(msg) => {
                    println!("  ✓ Applied: {}", msg);
                    applied += 1;
                }
                Err(e) => println!("  ✗ Failed to apply {}: {}", patch.description, e),
            }
        }
        println!();
        println!("  {} of {} patches applied.", applied, patches.len());
    } else {
        println!("  {} patches ready. Re-run with --auto to apply.", patches.len());
        println!("  Add --verify to run Zeus formal verification before applying.");
    }

    Ok(())
}

async fn cmd_verify(
    path: &PathBuf,
    properties: &[String],
) -> Result<(), Box<dyn std::error::Error>> {
    println!("╔══════════════════════════════════════════════════════╗");
    println!("║         ZEUS SHIELD — Formal Verifier               ║");
    println!("╚══════════════════════════════════════════════════════╝");
    println!();

    if !path.exists() {
        eprintln!("  ✗ File not found: {:?}", path);
        return Ok(());
    }

    let zeus_path = which_zeus();
    if !zeus_path.exists() {
        eprintln!("  ✗ Zeus binary not found. Set ZEUS_PATH or place zeus on PATH.");
        return Ok(());
    }

    println!("  Zeus binary: {:?}", zeus_path);
    println!("  File:        {:?}", path);
    println!();

    let verifier = ZeusVerifier::new(zeus_path);

    // Read file content as the patch diff
    let content = tokio::fs::read_to_string(path).await?;
    let patch = Patch {
        id: Uuid::new_v4(),
        vuln_id: Uuid::nil(),
        description: path.to_string_lossy().to_string(),
        diff: content,
        patch_type: PatchType::ZeusSource,
        confidence: 1.0,
        verified: false,
        certificate: None,
        created_at: chrono::Utc::now(),
    };

    // Check individual properties if requested
    if !properties.is_empty() {
        for prop in properties {
            match verifier.check_property(&patch, prop).await {
                Ok(true)  => println!("  ✓ Property '{prop}': PROVEN"),
                Ok(false) => println!("  ✗ Property '{prop}': NOT PROVEN"),
                Err(e)    => println!("  ! Property '{prop}': ERROR — {e}"),
            }
        }
        println!();
    }

    // Full verification
    println!("  Running zeus verify...");
    match verifier.verify(&patch).await {
        Ok(cert) => {
            println!("  ✓ VERIFICATION PASSED");
            println!("  Properties proven: {}", cert.properties_proven.join(", "));
            println!("  Certificate ID:    {}", cert.id);
            println!("  Signature (SHA256): {}", &cert.signature[..16]);
            println!("  Issued at:         {}", cert.issued_at);
        }
        Err(e) => {
            println!("  ✗ VERIFICATION FAILED");
            println!("  Reason: {}", e);
        }
    }
    Ok(())
}

async fn cmd_agent(
    console_url: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let url = console_url.unwrap_or_else(|| "http://localhost:8443".to_string());
    println!("[*] Launching zeus-shield-agent daemon");
    println!("[*] Reporting to: {}", url);

    // Find agent binary alongside this binary
    let exe = std::env::current_exe()?;
    let agent_bin = exe.parent().unwrap().join("zeus-shield-agent");

    if !agent_bin.exists() {
        eprintln!("  ✗ zeus-shield-agent not found at {:?}", agent_bin);
        eprintln!("    Run it directly: zeus-shield-agent --console-url {}", url);
        return Ok(());
    }

    let status = tokio::process::Command::new(&agent_bin)
        .arg("--console-url")
        .arg(&url)
        .status()
        .await?;

    if !status.success() {
        eprintln!("  ✗ Agent exited with status: {}", status);
    }
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
    let zeus_path = which_zeus();
    // Zeus prints its banner to stdout on any invocation (no --version flag)
    let zeus_check = tokio::process::Command::new(&zeus_path)
        .output()
        .await;

    match zeus_check {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let version_line = stdout.lines()
                .find(|l| l.contains("Zeus") && l.contains("v"))
                .unwrap_or("")
                .trim()
                .to_string();
            println!("  Zeus compiler: ✓ {:?}", zeus_path);
            if !version_line.is_empty() {
                println!("                 {}", version_line);
            }
        }
        Err(_) => {
            println!("  Zeus compiler: ✗ not found at {:?} (verification disabled)", zeus_path);
            println!("                 Set ZEUS_PATH=/path/to/zeus to enable verification.");
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

/// Locate the zeus binary: $ZEUS_PATH env var, next to this exe, or 'zeus' on PATH
fn which_zeus() -> std::path::PathBuf {
    if let Ok(p) = std::env::var("ZEUS_PATH") {
        return std::path::PathBuf::from(p);
    }
    if let Ok(exe) = std::env::current_exe() {
        let sibling = exe.parent().unwrap_or(std::path::Path::new(".")).join("zeus");
        if sibling.exists() {
            return sibling;
        }
    }
    // Last resort: rely on PATH
    std::path::PathBuf::from("zeus")
}

/// Apply a patch to the real system.
/// FirewallRule patches: writes a shell script to /etc/zeus-shield/patches/ and executes it.
/// ConfigChange patches: same execution model.
/// Requires root for firewall operations; returns descriptive error if not.
async fn apply_patch_to_system(patch: &Patch) -> Result<String, String> {
    if patch.diff.is_empty() {
        return Err(format!("Patch {} has empty diff", patch.id));
    }

    // Write patch script to a known directory
    let patch_dir = std::path::Path::new("/etc/zeus-shield/patches");
    let local_dir = std::path::PathBuf::from(
        std::env::var("HOME").unwrap_or_default() + "/.zeus-shield/patches"
    );
    let dir = if patch_dir.exists() || std::fs::create_dir_all(patch_dir).is_ok() {
        patch_dir.to_path_buf()
    } else {
        std::fs::create_dir_all(&local_dir).ok();
        local_dir
    };

    let script_path = dir.join(format!("patch-{}.sh", patch.id));
    tokio::fs::write(&script_path, &patch.diff)
        .await
        .map_err(|e| format!("Write failed: {}", e))?;

    // Make executable
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = tokio::fs::metadata(&script_path).await
            .map_err(|e| e.to_string())?.permissions();
        perms.set_mode(0o750);
        tokio::fs::set_permissions(&script_path, perms).await.ok();
    }

    // Execute with sh
    let output = tokio::process::Command::new("sh")
        .arg(&script_path)
        .output()
        .await
        .map_err(|e| format!("Execution failed: {}", e))?;

    if output.status.success() {
        Ok(format!("{} (script: {:?})", patch.description, script_path))
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!("Script failed: {}", stderr.trim()))
    }
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
