use async_trait::async_trait;
use shield_core::{
    error::ShieldResult,
    types::*,
    traits::Scanner,
};
use uuid::Uuid;
use chrono::Utc;

pub struct DeviceScanner;

impl DeviceScanner {
    pub fn new() -> Self {
        Self
    }

    /// Collect local system information
    async fn collect_system_info(&self) -> ShieldResult<SystemInfo> {
        let hostname = hostname::get()
            .map(|h| h.to_string_lossy().to_string())
            .unwrap_or_else(|_| "unknown".to_string());

        let os = std::env::consts::OS.to_string();
        let arch = std::env::consts::ARCH.to_string();

        Ok(SystemInfo {
            hostname,
            os,
            arch,
            kernel_version: get_kernel_version(),
        })
    }

    /// Check for available OS security updates
    async fn check_os_patches(&self) -> ShieldResult<Vec<Vulnerability>> {
        let mut vulns = Vec::new();

        #[cfg(target_os = "macos")]
        {
            // softwareupdate -l lists available updates; security ones contain "Security"
            let output = tokio::process::Command::new("softwareupdate")
                .arg("-l")
                .output()
                .await;

            if let Ok(out) = output {
                let text = String::from_utf8_lossy(&out.stdout).to_string()
                    + &String::from_utf8_lossy(&out.stderr).to_string();
                let security_updates: Vec<&str> = text
                    .lines()
                    .filter(|l| l.to_lowercase().contains("security") || l.contains("*"))
                    .collect();

                if !security_updates.is_empty() {
                    vulns.push(Vulnerability {
                        id: Uuid::new_v4(),
                        target_id: Uuid::nil(),
                        title: format!("{} macOS security updates available", security_updates.len()),
                        description: format!(
                            "Pending updates:\n{}",
                            security_updates.join("\n")
                        ),
                        severity: Severity::High,
                        category: VulnCategory::OutdatedSoftware,
                        cve_id: None,
                        cvss_score: None,
                        evidence: Evidence {
                            raw: text.lines().take(20).collect::<Vec<_>>().join("\n"),
                            reproduction_steps: vec!["softwareupdate -l".to_string()],
                            affected_component: "macOS".to_string(),
                            network_trace: None,
                        },
                        status: VulnStatus::Open,
                        discovered_at: Utc::now(),
                        fixed_at: None,
                    });
                }
            }
        }

        #[cfg(target_os = "linux")]
        {
            // Try apt first, then yum/dnf
            let apt = tokio::process::Command::new("apt")
                .args(["list", "--upgradable", "-qq"])
                .output()
                .await;

            if let Ok(out) = apt {
                let text = String::from_utf8_lossy(&out.stdout).to_string();
                let security: Vec<&str> = text.lines()
                    .filter(|l| l.contains("security"))
                    .collect();
                if !security.is_empty() {
                    vulns.push(Vulnerability {
                        id: Uuid::new_v4(),
                        target_id: Uuid::nil(),
                        title: format!("{} apt security updates available", security.len()),
                        description: security.join("\n"),
                        severity: Severity::High,
                        category: VulnCategory::OutdatedSoftware,
                        cve_id: None,
                        cvss_score: None,
                        evidence: Evidence {
                            raw: text.lines().take(30).collect::<Vec<_>>().join("\n"),
                            reproduction_steps: vec!["apt list --upgradable".to_string()],
                            affected_component: "apt packages".to_string(),
                            network_trace: None,
                        },
                        status: VulnStatus::Open,
                        discovered_at: Utc::now(),
                        fixed_at: None,
                    });
                }
            } else {
                // Fallback: yum/dnf
                let yum = tokio::process::Command::new("yum")
                    .args(["check-update", "--security", "-q"])
                    .output()
                    .await;
                if let Ok(out) = yum {
                    let text = String::from_utf8_lossy(&out.stdout).to_string();
                    let updates: Vec<&str> = text.lines()
                        .filter(|l| !l.trim().is_empty())
                        .collect();
                    if !updates.is_empty() {
                        vulns.push(Vulnerability {
                            id: Uuid::new_v4(),
                            target_id: Uuid::nil(),
                            title: format!("{} yum security updates available", updates.len()),
                            description: updates.join("\n"),
                            severity: Severity::High,
                            category: VulnCategory::OutdatedSoftware,
                            cve_id: None,
                            cvss_score: None,
                            evidence: Evidence {
                                raw: text,
                                reproduction_steps: vec!["yum check-update --security".to_string()],
                                affected_component: "yum packages".to_string(),
                                network_trace: None,
                            },
                            status: VulnStatus::Open,
                            discovered_at: Utc::now(),
                            fixed_at: None,
                        });
                    }
                }
            }
        }

        Ok(vulns)
    }

    /// Check running services for dangerous misconfigurations
    async fn check_services(&self) -> ShieldResult<Vec<Vulnerability>> {
        let mut vulns = Vec::new();

        // Dangerous services: (process_name, title, description, severity)
        let dangerous: &[(&str, &str, &str, Severity)] = &[
            ("telnetd",  "Telnet daemon running",
             "Telnet transmits credentials in plaintext. Disable and use SSH.",
             Severity::Critical),
            ("ftpd",     "FTP daemon running",
             "FTP transmits credentials in plaintext. Use SFTP instead.",
             Severity::High),
            ("rshd",     "RSH daemon running",
             "RSH provides unauthenticated remote shell access.",
             Severity::Critical),
            ("rlogind",  "rlogin daemon running",
             "rlogin is a legacy insecure remote login service.",
             Severity::High),
            ("tftpd",    "TFTP daemon running",
             "TFTP has no authentication. Restrict to local networks.",
             Severity::Medium),
            ("snmpd",    "SNMP daemon running",
             "Check SNMP community strings are not default (public/private).",
             Severity::Medium),
        ];

        // Get running process list
        let procs = get_running_processes().await;

        for (proc_name, title, description, severity) in dangerous {
            if procs.iter().any(|p| p.contains(proc_name)) {
                vulns.push(Vulnerability {
                    id: Uuid::new_v4(),
                    target_id: Uuid::nil(),
                    title: title.to_string(),
                    description: description.to_string(),
                    severity: severity.clone(),
                    category: VulnCategory::Misconfiguration,
                    cve_id: None,
                    cvss_score: None,
                    evidence: Evidence {
                        raw: format!("Process '{}' found in process list", proc_name),
                        reproduction_steps: vec![
                            "ps aux | grep -v grep".to_string(),
                        ],
                        affected_component: proc_name.to_string(),
                        network_trace: None,
                    },
                    status: VulnStatus::Open,
                    discovered_at: Utc::now(),
                    fixed_at: None,
                });
            }
        }

        // Check for world-writable directories in $PATH
        if let Ok(path_var) = std::env::var("PATH") {
            for dir in path_var.split(':') {
                if let Ok(meta) = std::fs::metadata(dir) {
                    #[cfg(unix)]
                    {
                        use std::os::unix::fs::PermissionsExt;
                        let mode = meta.permissions().mode();
                        if mode & 0o002 != 0 {
                            vulns.push(Vulnerability {
                                id: Uuid::new_v4(),
                                target_id: Uuid::nil(),
                                title: format!("World-writable PATH directory: {}", dir),
                                description: format!(
                                    "Directory {} in PATH is world-writable ({:o}). \
                                     Attacker can plant malicious binaries.",
                                    dir, mode & 0o777
                                ),
                                severity: Severity::Critical,
                                category: VulnCategory::Misconfiguration,
                                cve_id: None,
                                cvss_score: None,
                                evidence: Evidence {
                                    raw: format!("mode={:o}", mode & 0o777),
                                    reproduction_steps: vec![
                                        format!("ls -ld {}", dir),
                                    ],
                                    affected_component: dir.to_string(),
                                    network_trace: None,
                                },
                                status: VulnStatus::Open,
                                discovered_at: Utc::now(),
                                fixed_at: None,
                            });
                        }
                    }
                }
            }
        }

        Ok(vulns)
    }

    /// Check file permissions for sensitive files
    async fn check_permissions(&self) -> ShieldResult<Vec<Vulnerability>> {
        let mut vulns = Vec::new();
        
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            
            let sensitive_files = vec![
                "/etc/shadow",
                "/etc/passwd",
                "/etc/ssh/sshd_config",
                "~/.ssh/id_rsa",
                "~/.ssh/id_ed25519",
            ];

            for file in sensitive_files {
                let expanded = shellexpand_path(file);
                if let Ok(metadata) = std::fs::metadata(&expanded) {
                    let mode = metadata.permissions().mode();
                    // Check if world-readable
                    if mode & 0o004 != 0 && file.contains("shadow") || file.contains("id_") {
                        vulns.push(Vulnerability {
                            id: Uuid::new_v4(),
                            target_id: Uuid::nil(),
                            title: format!("Sensitive file world-readable: {}", file),
                            description: format!(
                                "File {} has permissions {:o}, should not be world-readable",
                                file, mode & 0o777
                            ),
                            severity: Severity::High,
                            category: VulnCategory::Misconfiguration,
                            cve_id: None,
                            cvss_score: None,
                            evidence: Evidence {
                                raw: format!("mode={:o}", mode & 0o777),
                                reproduction_steps: vec![
                                    format!("ls -la {}", file),
                                ],
                                affected_component: file.to_string(),
                                network_trace: None,
                            },
                            status: VulnStatus::Open,
                            discovered_at: Utc::now(),
                            fixed_at: None,
                        });
                    }
                }
            }
        }

        Ok(vulns)
    }
}

#[async_trait]
impl Scanner for DeviceScanner {
    fn name(&self) -> &str {
        "device-scanner"
    }

    fn supported_targets(&self) -> Vec<TargetKind> {
        vec![TargetKind::Host, TargetKind::IoTDevice]
    }

    async fn discover(&self) -> ShieldResult<Vec<Target>> {
        let info = self.collect_system_info().await?;
        Ok(vec![Target {
            id: Uuid::new_v4(),
            name: info.hostname.clone(),
            kind: TargetKind::Host,
            address: "localhost".to_string(),
            metadata: serde_json::json!({
                "os": info.os,
                "arch": info.arch,
                "kernel": info.kernel_version,
            }),
            created_at: Utc::now(),
        }])
    }

    async fn scan(&self, target: &Target) -> ShieldResult<Vec<Vulnerability>> {
        let mut all_vulns = Vec::new();

        let patch_vulns = self.check_os_patches().await?;
        all_vulns.extend(patch_vulns);

        let service_vulns = self.check_services().await?;
        all_vulns.extend(service_vulns);

        let perm_vulns = self.check_permissions().await?;
        all_vulns.extend(perm_vulns);

        for vuln in &mut all_vulns {
            vuln.target_id = target.id;
        }

        Ok(all_vulns)
    }

    async fn health_check(&self) -> ShieldResult<()> {
        Ok(())
    }
}

// ─── Helpers ────────────────────────────────────────────────────────────────

struct SystemInfo {
    hostname: String,
    os: String,
    arch: String,
    kernel_version: String,
}

async fn get_running_processes() -> Vec<String> {
    #[cfg(unix)]
    {
        let out = tokio::process::Command::new("ps")
            .args(["aux"])
            .output()
            .await;
        match out {
            Ok(o) => String::from_utf8_lossy(&o.stdout)
                .lines()
                .map(|l| l.to_string())
                .collect(),
            Err(_) => vec![],
        }
    }
    #[cfg(not(unix))]
    {
        vec![]
    }
}

fn get_kernel_version() -> String {
    #[cfg(unix)]
    {
        std::process::Command::new("uname")
            .arg("-r")
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .unwrap_or_else(|_| "unknown".to_string())
    }
    #[cfg(not(unix))]
    {
        "unknown".to_string()
    }
}

fn shellexpand_path(path: &str) -> String {
    if path.starts_with("~/") {
        if let Some(home) = dirs::home_dir() {
            return path.replacen("~", &home.to_string_lossy(), 1);
        }
    }
    path.to_string()
}

mod dirs {
    pub fn home_dir() -> Option<std::path::PathBuf> {
        std::env::var("HOME").ok().map(std::path::PathBuf::from)
    }
}
mod hostname {
    pub fn get() -> std::io::Result<std::ffi::OsString> {
        #[cfg(unix)]
        {
            let output = std::process::Command::new("hostname").output()?;
            Ok(std::ffi::OsString::from(
                String::from_utf8_lossy(&output.stdout).trim().to_string()
            ))
        }
        #[cfg(not(unix))]
        {
            Ok(std::ffi::OsString::from("unknown"))
        }
    }
}
