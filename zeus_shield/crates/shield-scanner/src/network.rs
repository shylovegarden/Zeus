use async_trait::async_trait;
use shield_core::{
    error::ShieldResult,
    types::*,
    traits::Scanner,
};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream as TokioTcpStream;
use uuid::Uuid;
use chrono::Utc;

pub struct NetworkScanner {
    port_range: Vec<u16>,
    timeout: Duration,
    concurrent_probes: usize,
}

impl NetworkScanner {
    pub fn new() -> Self {
        Self {
            port_range: (1..=1024).collect(),
            timeout: Duration::from_secs(3),
            concurrent_probes: 100,
        }
    }

    pub fn with_port_range(mut self, start: u16, end: u16) -> Self {
        self.port_range = (start..=end).collect();
        self
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn with_concurrency(mut self, n: usize) -> Self {
        self.concurrent_probes = n;
        self
    }

    async fn scan_port(&self, host: &str, port: u16) -> Option<PortResult> {
        let addr = format!("{}:{}", host, port);
        match tokio::time::timeout(
            self.timeout,
            TokioTcpStream::connect(&addr),
        ).await {
            Ok(Ok(_stream)) => {
                let service = identify_service(port);
                Some(PortResult {
                    port,
                    state: PortState::Open,
                    service,
                    banner: None,
                })
            }
            _ => None,
        }
    }

    async fn grab_banner(&self, host: &str, port: u16) -> Option<String> {
        let addr = format!("{}:{}", host, port);
        let timeout_result = tokio::time::timeout(
            Duration::from_secs(2),
            TokioTcpStream::connect(&addr),
        ).await;

        if let Ok(Ok(mut stream)) = timeout_result {
            // Send protocol-appropriate probe
            let probe: Option<&[u8]> = match port {
                80 | 8080 => Some(b"HEAD / HTTP/1.0\r\nHost: target\r\n\r\n"),
                21 => None, // FTP sends banner immediately
                22 => None, // SSH sends banner immediately
                25 | 587 => Some(b"EHLO shield\r\n"),
                110 => None, // POP3 sends banner
                143 => None, // IMAP sends banner
                _ => None,
            };

            if let Some(p) = probe {
                let _ = stream.write_all(p).await;
            }

            let mut buf = vec![0u8; 512];
            if let Ok(Ok(n)) = tokio::time::timeout(
                Duration::from_secs(2),
                stream.read(&mut buf),
            ).await {
                if n > 0 {
                    let raw = String::from_utf8_lossy(&buf[..n]).to_string();
                    // Return first non-empty line only
                    let first_line = raw.lines().find(|l| !l.trim().is_empty())
                        .unwrap_or("").to_string();
                    return Some(first_line);
                }
            }
        }
        None
    }
}

#[async_trait]
impl Scanner for NetworkScanner {
    fn name(&self) -> &str {
        "network-scanner"
    }

    fn supported_targets(&self) -> Vec<TargetKind> {
        vec![TargetKind::Host, TargetKind::Network, TargetKind::Router, TargetKind::Firewall]
    }

    async fn discover(&self) -> ShieldResult<Vec<Target>> {
        let mut targets = Vec::new();
        let mut seen = std::collections::HashSet::new();

        // 1. Read the local ARP table — instantaneous, no network traffic
        let arp_hosts = read_arp_table().await;
        for host in arp_hosts {
            if seen.insert(host.clone()) {
                targets.push(Target {
                    id: Uuid::new_v4(),
                    name: host.clone(),
                    kind: TargetKind::Host,
                    address: host,
                    metadata: serde_json::json!({"source": "arp"}),
                    created_at: Utc::now(),
                });
            }
        }

        // 2. Ping-sweep the local /24 derived from our primary interface IP
        if let Some(local_ip) = get_local_ip() {
            let parts: Vec<&str> = local_ip.splitn(4, '.').collect();
            if parts.len() == 4 {
                let prefix = format!("{}.{}.{}", parts[0], parts[1], parts[2]);
                tracing::info!("Ping-sweeping {}.1-254", prefix);

                // Concurrently ping all 254 addresses
                let mut handles = Vec::new();
                for i in 1u8..=254 {
                    let addr = format!("{}.{}", prefix, i);
                    handles.push(tokio::spawn(async move {
                        let alive = ping_host(&addr).await;
                        (addr, alive)
                    }));
                }

                for handle in handles {
                    if let Ok((addr, true)) = handle.await {
                        if seen.insert(addr.clone()) {
                            targets.push(Target {
                                id: Uuid::new_v4(),
                                name: addr.clone(),
                                kind: TargetKind::Host,
                                address: addr,
                                metadata: serde_json::json!({"source": "ping_sweep"}),
                                created_at: Utc::now(),
                            });
                        }
                    }
                }
            }
        }

        tracing::info!("Discovered {} hosts", targets.len());
        Ok(targets)
    }

    async fn scan(&self, target: &Target) -> ShieldResult<Vec<Vulnerability>> {
        let host = &target.address;
        let mut vulns = Vec::new();

        tracing::info!("Scanning {} ({} ports)", host, self.port_range.len());

        // Scan ports in batches
        let mut open_ports = Vec::new();
        for chunk in self.port_range.chunks(self.concurrent_probes) {
            let mut handles = Vec::new();
            for &port in chunk {
                let host = host.clone();
                let timeout = self.timeout;
                handles.push(tokio::spawn(async move {
                    let addr = format!("{}:{}", host, port);
                    match tokio::time::timeout(
                        timeout,
                        TokioTcpStream::connect(&addr),
                    ).await {
                        Ok(Ok(_)) => Some(port),
                        _ => None,
                    }
                }));
            }
            for handle in handles {
                if let Ok(Some(port)) = handle.await {
                    open_ports.push(port);
                }
            }
        }

        tracing::info!("Found {} open ports on {}", open_ports.len(), host);

        // Grab banners and analyse open ports concurrently
        let mut banner_handles = Vec::new();
        for &port in &open_ports {
            let host = host.clone();
            let timeout = self.timeout;
            banner_handles.push(tokio::spawn(async move {
                let addr = format!("{}:{}", host, port);
                let result = tokio::time::timeout(
                    Duration::from_secs(2),
                    TokioTcpStream::connect(&addr),
                ).await;
                if let Ok(Ok(mut stream)) = result {
                    let probe: Option<&[u8]> = match port {
                        80 | 8080 => Some(b"HEAD / HTTP/1.0\r\nHost: target\r\n\r\n"),
                        25 | 587 => Some(b"EHLO shield\r\n"),
                        _ => None,
                    };
                    if let Some(p) = probe {
                        let _ = stream.write_all(p).await;
                    }
                    let mut buf = vec![0u8; 512];
                    if let Ok(Ok(n)) = tokio::time::timeout(
                        Duration::from_secs(2),
                        stream.read(&mut buf),
                    ).await {
                        if n > 0 {
                            let raw = String::from_utf8_lossy(&buf[..n]).to_string();
                            let banner = raw.lines().find(|l| !l.trim().is_empty())
                                .unwrap_or("").trim().to_string();
                            return (port, Some(banner));
                        }
                    }
                }
                (port, None::<String>)
            }));
        }

        let mut port_banners: std::collections::HashMap<u16, Option<String>> =
            std::collections::HashMap::new();
        for h in banner_handles {
            if let Ok((port, banner)) = h.await {
                port_banners.insert(port, banner);
            }
        }

        // Analyze open ports for vulnerabilities
        for port in &open_ports {
            let service = identify_service(*port);
            let banner = port_banners.get(port).and_then(|b| b.clone());

            if let Some(vuln) = check_dangerous_service(*port, &service) {
                let raw_evidence = match &banner {
                    Some(b) => format!("Port {} open: {} | Banner: {}", port, service, b),
                    None    => format!("Port {} open: {}", port, service),
                };
                vulns.push(Vulnerability {
                    id: Uuid::new_v4(),
                    target_id: target.id,
                    title: vuln.0,
                    description: vuln.1,
                    severity: vuln.2,
                    category: VulnCategory::NetworkExposure,
                    cve_id: None,
                    cvss_score: None,
                    evidence: Evidence {
                        raw: raw_evidence,
                        reproduction_steps: vec![
                            format!("Connect to {}:{}", host, port),
                        ],
                        affected_component: service.clone(),
                        network_trace: None,
                    },
                    status: VulnStatus::Open,
                    discovered_at: Utc::now(),
                    fixed_at: None,
                });
            }
        }

        Ok(vulns)
    }

    async fn health_check(&self) -> ShieldResult<()> {
        Ok(())
    }
}

// ─── Helpers ────────────────────────────────────────────────────────────────

#[derive(Debug)]
struct PortResult {
    port: u16,
    state: PortState,
    service: String,
    banner: Option<String>,
}

#[derive(Debug)]
enum PortState {
    Open,
    Closed,
    Filtered,
}

fn identify_service(port: u16) -> String {
    match port {
        21 => "FTP".to_string(),
        22 => "SSH".to_string(),
        23 => "Telnet".to_string(),
        25 => "SMTP".to_string(),
        53 => "DNS".to_string(),
        80 => "HTTP".to_string(),
        110 => "POP3".to_string(),
        143 => "IMAP".to_string(),
        443 => "HTTPS".to_string(),
        445 => "SMB".to_string(),
        993 => "IMAPS".to_string(),
        995 => "POP3S".to_string(),
        1433 => "MSSQL".to_string(),
        3306 => "MySQL".to_string(),
        3389 => "RDP".to_string(),
        5432 => "PostgreSQL".to_string(),
        5900 => "VNC".to_string(),
        6379 => "Redis".to_string(),
        8080 => "HTTP-Proxy".to_string(),
        8443 => "HTTPS-Alt".to_string(),
        9200 => "Elasticsearch".to_string(),
        27017 => "MongoDB".to_string(),
        _ => format!("Unknown({})", port),
    }
}

fn check_dangerous_service(port: u16, service: &str) -> Option<(String, String, Severity)> {
    match port {
        23 => Some((
            "Telnet service exposed".to_string(),
            "Telnet transmits credentials in plaintext. Replace with SSH.".to_string(),
            Severity::Critical,
        )),
        21 => Some((
            "FTP service exposed".to_string(),
            "FTP transmits credentials in plaintext. Replace with SFTP.".to_string(),
            Severity::High,
        )),
        3389 => Some((
            "RDP service exposed to network".to_string(),
            "RDP is frequently targeted for brute-force attacks. Restrict access via VPN.".to_string(),
            Severity::High,
        )),
        6379 => Some((
            "Redis exposed without authentication".to_string(),
            "Redis default configuration has no authentication. Bind to localhost or add AUTH.".to_string(),
            Severity::Critical,
        )),
        27017 => Some((
            "MongoDB exposed without authentication".to_string(),
            "MongoDB default configuration allows unauthenticated access. Enable auth.".to_string(),
            Severity::Critical,
        )),
        9200 => Some((
            "Elasticsearch exposed".to_string(),
            "Elasticsearch without X-Pack security allows unrestricted data access.".to_string(),
            Severity::High,
        )),
        5900 => Some((
            "VNC service exposed".to_string(),
            "VNC may have weak authentication. Restrict to VPN or use SSH tunnel.".to_string(),
            Severity::Medium,
        )),
        _ => None,
    }
}

/// Read the OS ARP table and return all known IP addresses
async fn read_arp_table() -> Vec<String> {
    #[cfg(unix)]
    {
        // `arp -a` on macOS/Linux prints lines like: host (192.168.1.1) at ...
        let out = tokio::process::Command::new("arp")
            .arg("-a")
            .output()
            .await;
        let mut hosts = Vec::new();
        if let Ok(o) = out {
            for line in String::from_utf8_lossy(&o.stdout).lines() {
                // Extract IP from parentheses: "? (192.168.1.1) at ..."
                if let Some(start) = line.find('(') {
                    if let Some(end) = line[start..].find(')') {
                        let ip = &line[start + 1..start + end];
                        // Skip incomplete entries (all-zero MACs)
                        if !line.contains("(incomplete)") && !ip.is_empty() {
                            hosts.push(ip.to_string());
                        }
                    }
                }
            }
        }
        hosts
    }
    #[cfg(not(unix))]
    {
        vec![]
    }
}

/// Ping a single host; returns true if it responds within 300ms
async fn ping_host(addr: &str) -> bool {
    // Use the system `ping` with a 1-packet, 300ms timeout
    #[cfg(target_os = "macos")]
    let args = ["-c", "1", "-W", "300", addr];
    #[cfg(target_os = "linux")]
    let args = ["-c", "1", "-W", "1", addr]; // Linux -W is seconds
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    let args = ["-c", "1", addr];

    tokio::process::Command::new("ping")
        .args(&args)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .await
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Return the primary non-loopback IPv4 address of this machine
fn get_local_ip() -> Option<String> {
    // Parse `ip route` (Linux) or `route get default` (macOS) to find local IP
    let output = std::process::Command::new("hostname")
        .arg("-I")  // Linux
        .output();
    if let Ok(o) = output {
        let text = String::from_utf8_lossy(&o.stdout);
        if let Some(ip) = text.split_whitespace().next() {
            let ip = ip.to_string();
            if ip.starts_with("192.") || ip.starts_with("10.") || ip.starts_with("172.") {
                return Some(ip);
            }
        }
    }

    // macOS fallback: `ipconfig getifaddr en0`
    for iface in &["en0", "en1", "eth0", "wlan0"] {
        let out = std::process::Command::new("ipconfig")
            .args(["getifaddr", iface])
            .output();
        if let Ok(o) = out {
            let ip = String::from_utf8_lossy(&o.stdout).trim().to_string();
            if !ip.is_empty() && !ip.starts_with("127.") {
                return Some(ip);
            }
        }
    }
    None
}
