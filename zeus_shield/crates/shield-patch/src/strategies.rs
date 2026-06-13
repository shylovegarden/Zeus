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
        // TODO: Parse affected package and suggest update
        let diff = format!(
            "# Update vulnerable dependency\n\
             # Affected: {}\n\
             # CVE: {}\n",
            vuln.evidence.affected_component,
            vuln.cve_id.as_deref().unwrap_or("N/A"),
        );

        Ok(Some(Patch {
            id: Uuid::new_v4(),
            vuln_id: vuln.id,
            description: format!("Update dependency: {}", vuln.title),
            diff,
            patch_type: PatchType::DependencyUpdate,
            confidence: 0.7,
            verified: false,
            certificate: None,
            created_at: Utc::now(),
        }))
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
