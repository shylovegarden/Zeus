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

    /// Check for outdated OS packages
    async fn check_os_patches(&self) -> ShieldResult<Vec<Vulnerability>> {
        // TODO: Implement OS patch checking
        // Linux: apt list --upgradable / yum check-update
        // macOS: softwareupdate -l
        // Windows: wmic qfe list
        Ok(vec![])
    }

    /// Check running services for known issues
    async fn check_services(&self) -> ShieldResult<Vec<Vulnerability>> {
        // TODO: Check running services for misconfigurations
        // systemctl list-units --type=service
        // launchctl list
        Ok(vec![])
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
