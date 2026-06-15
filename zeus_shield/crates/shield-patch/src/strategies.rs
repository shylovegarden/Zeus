use async_trait::async_trait;
use shield_core::{
    error::ShieldResult,
    types::*,
};
use uuid::Uuid;
use chrono::Utc;

/// Strategy trait for different patch generation approaches
#[async_trait]
pub trait PatchStrategy: Send + Sync {
    fn name(&self) -> &str;
    fn can_handle(&self, vuln: &Vulnerability) -> bool;
    async fn generate(&self, vuln: &Vulnerability) -> ShieldResult<Option<Patch>>;
}

// ─── Firewall Rule Strategy ─────────────────────────────────────────────────

pub struct FirewallRuleStrategy;

#[async_trait]
impl PatchStrategy for FirewallRuleStrategy {
    fn name(&self) -> &str { "firewall-rule" }

    fn can_handle(&self, vuln: &Vulnerability) -> bool {
        vuln.category == VulnCategory::NetworkExposure
    }

    async fn generate(&self, vuln: &Vulnerability) -> ShieldResult<Option<Patch>> {
        // Generate iptables/nftables/ufw rule to block exposed service
        let port = extract_port_from_evidence(&vuln.evidence.raw);
        
        let diff = if let Some(port) = port {
            format!(
                "# Block external access to port {}\n\
                 ufw deny from any to any port {}\n\
                 # Or using iptables:\n\
                 # iptables -A INPUT -p tcp --dport {} -j DROP\n",
                port, port, port
            )
        } else {
            return Ok(None);
        };

        Ok(Some(Patch {
            id: Uuid::new_v4(),
            vuln_id: vuln.id,
            description: format!("Block network access: {}", vuln.title),
            diff,
            patch_type: PatchType::FirewallRule,
            confidence: 0.9,
            verified: false,
            certificate: None,
            created_at: Utc::now(),
        }))
    }
}

// ─── Config Fix Strategy ────────────────────────────────────────────────────

pub struct ConfigFixStrategy;

#[async_trait]
impl PatchStrategy for ConfigFixStrategy {
    fn name(&self) -> &str { "config-fix" }

    fn can_handle(&self, vuln: &Vulnerability) -> bool {
        vuln.category == VulnCategory::Misconfiguration
    }

    async fn generate(&self, vuln: &Vulnerability) -> ShieldResult<Option<Patch>> {
        let diff = match vuln.evidence.affected_component.as_str() {
            path if path.contains("sshd_config") => {
                "# Harden SSH configuration\n\
                 PermitRootLogin no\n\
                 PasswordAuthentication no\n\
                 MaxAuthTries 3\n\
                 Protocol 2\n".to_string()
            }
            path if path.contains("shadow") || path.contains("id_") => {
                format!("chmod 600 {}\n", path)
            }
            _ => return Ok(None),
        };

        Ok(Some(Patch {
            id: Uuid::new_v4(),
            vuln_id: vuln.id,
            description: format!("Fix configuration: {}", vuln.title),
            diff,
            patch_type: PatchType::ConfigChange,
            confidence: 0.85,
            verified: false,
            certificate: None,
            created_at: Utc::now(),
        }))
    }
}

// ─── Dependency Update Strategy ─────────────────────────────────────────────

pub struct DependencyUpdateStrategy;

#[async_trait]
impl PatchStrategy for DependencyUpdateStrategy {
    fn name(&self) -> &str { "dependency-update" }

    fn can_handle(&self, vuln: &Vulnerability) -> bool {
        vuln.category == VulnCategory::OutdatedSoftware ||
        vuln.category == VulnCategory::SupplyChain
    }

    async fn generate(&self, vuln: &Vulnerability) -> ShieldResult<Option<Patch>> {
        let component = &vuln.evidence.affected_component;

        // Try to locate a manifest file containing this package
        let manifest = find_manifest_for(component);

        // Query OSV for the latest non-vulnerable version
        let fixed_version = query_osv_fixed_version(
            component,
            vuln.cve_id.as_deref(),
        ).await;

        let diff = build_dep_diff(component, &manifest, fixed_version.as_deref(), vuln);
        let confidence = if fixed_version.is_some() { 0.9 } else { 0.6 };

        Ok(Some(Patch {
            id: Uuid::new_v4(),
            vuln_id: vuln.id,
            description: format!("Update dependency: {}", component),
            diff,
            patch_type: PatchType::DependencyUpdate,
            confidence,
            verified: false,
            certificate: None,
            created_at: Utc::now(),
        }))
    }
}

/// Scan CWD and parent dirs for Cargo.toml or package.json containing `component`
fn find_manifest_for(component: &str) -> Option<(String, String)> {
    let search_dirs = [
        std::env::current_dir().ok().map(|p| p.to_string_lossy().to_string()),
        std::env::var("HOME").ok(),
    ];

    for maybe_dir in search_dirs.iter().flatten() {
        // Cargo.toml
        let cargo = format!("{}/Cargo.toml", maybe_dir);
        if let Ok(content) = std::fs::read_to_string(&cargo) {
            if content.contains(component) {
                return Some(("cargo".to_string(), cargo));
            }
        }
        // package.json
        let pkg = format!("{}/package.json", maybe_dir);
        if let Ok(content) = std::fs::read_to_string(&pkg) {
            if content.contains(component) {
                return Some(("npm".to_string(), pkg));
            }
        }
        // requirements.txt
        let req = format!("{}/requirements.txt", maybe_dir);
        if let Ok(content) = std::fs::read_to_string(&req) {
            if content.to_lowercase().contains(&component.to_lowercase()) {
                return Some(("pip".to_string(), req));
            }
        }
    }
    None
}

/// Query OSV API for the fixed version of a package/CVE pair
async fn query_osv_fixed_version(package: &str, cve_id: Option<&str>) -> Option<String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(8))
        .build()
        .ok()?;

    // Build query: prefer CVE ID if available, fall back to package name
    let query = if let Some(cve) = cve_id {
        serde_json::json!({"query": {"package": {"name": package}}, "version": ""})
    } else {
        serde_json::json!({"query": {"package": {"name": package}}})
    };
    let _ = cve_id; // already used above

    let resp = client
        .post("https://api.osv.dev/v1/query")
        .json(&query)
        .send()
        .await
        .ok()?;

    let json: serde_json::Value = resp.json().await.ok()?;

    // Extract the latest "fixed" version from the first matching advisory
    let vulns = json.get("vulns")?.as_array()?;
    for vuln in vulns {
        if let Some(affected) = vuln.get("affected").and_then(|a| a.as_array()) {
            for aff in affected {
                if let Some(ranges) = aff.get("ranges").and_then(|r| r.as_array()) {
                    for range in ranges {
                        if let Some(events) = range.get("events").and_then(|e| e.as_array()) {
                            for event in events {
                                if let Some(fixed) = event.get("fixed").and_then(|f| f.as_str()) {
                                    return Some(fixed.to_string());
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    None
}

/// Build the actual diff/script for updating the dependency
fn build_dep_diff(
    component: &str,
    manifest: &Option<(String, String)>,
    fixed_version: Option<&str>,
    vuln: &Vulnerability,
) -> String {
    let cve = vuln.cve_id.as_deref().unwrap_or("N/A");
    let ver_note = match fixed_version {
        Some(v) => format!("# Fixed in version: {}\n", v),
        None    => format!("# No fixed version found in OSV — check upstream manually\n"),
    };

    match manifest {
        Some((kind, path)) if kind == "cargo" => {
            let update_cmd = match fixed_version {
                Some(v) => format!("cargo update -p {}@{}", component, v),
                None    => format!("cargo update -p {}", component),
            };
            format!(
                "# CVE: {}\n\
                 {}# Manifest: {}\n\
                 \n\
                 # Update the dependency:\n\
                 {}\n\
                 \n\
                 # Or pin in Cargo.toml:\n\
                 # {} = \"={}\"\n",
                cve, ver_note, path, update_cmd,
                component, fixed_version.unwrap_or("<latest>")
            )
        }
        Some((kind, path)) if kind == "npm" => {
            let install_cmd = match fixed_version {
                Some(v) => format!("npm install {}@{}", component, v),
                None    => format!("npm update {}", component),
            };
            format!(
                "# CVE: {}\n\
                 {}# Manifest: {}\n\
                 \n\
                 {}",
                cve, ver_note, path, install_cmd
            )
        }
        Some((kind, path)) if kind == "pip" => {
            let install_cmd = match fixed_version {
                Some(v) => format!("pip install \"{}>={}\" --upgrade", component, v),
                None    => format!("pip install {} --upgrade", component),
            };
            format!(
                "# CVE: {}\n\
                 {}# Manifest: {}\n\
                 \n\
                 {}",
                cve, ver_note, path, install_cmd
            )
        }
        _ => {
            // No manifest found; generic guidance
            format!(
                "# CVE: {}\n\
                 {}# Package: {}\n\
                 \n\
                 # No manifest found automatically.\n\
                 # Manually update {} to the fixed version in your dependency file.",
                cve, ver_note, component, component
            )
        }
    }
}

// ─── Helpers ────────────────────────────────────────────────────────────────

fn extract_port_from_evidence(raw: &str) -> Option<u16> {
    // Try to extract port number from evidence string like "Port 23 open: Telnet"
    let parts: Vec<&str> = raw.split_whitespace().collect();
    for (i, part) in parts.iter().enumerate() {
        if *part == "Port" || *part == "port" {
            if let Some(next) = parts.get(i + 1) {
                return next.parse().ok();
            }
        }
    }
    None
}
