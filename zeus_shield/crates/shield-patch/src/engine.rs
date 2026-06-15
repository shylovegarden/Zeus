use async_trait::async_trait;
use shield_core::{
    error::ShieldResult,
    types::*,
    traits::Patcher,
};
use uuid::Uuid;
use chrono::Utc;
use crate::strategies::*;

/// Main patch generation engine
/// Combines multiple strategies: rule-based, AI-driven, template-based
pub struct PatchEngine {
    strategies: Vec<Box<dyn PatchStrategy>>,
}

impl PatchEngine {
    pub fn new() -> Self {
        Self {
            strategies: vec![
                Box::new(FirewallRuleStrategy),
                Box::new(ConfigFixStrategy),
                Box::new(DependencyUpdateStrategy),
            ],
        }
    }

    pub fn add_strategy(&mut self, strategy: Box<dyn PatchStrategy>) {
        self.strategies.push(strategy);
    }
}

#[async_trait]
impl Patcher for PatchEngine {
    fn can_fix(&self, vuln: &Vulnerability) -> bool {
        self.strategies.iter().any(|s| s.can_handle(vuln))
    }

    async fn generate_patch(&self, vuln: &Vulnerability) -> ShieldResult<Patch> {
        for strategy in &self.strategies {
            if strategy.can_handle(vuln) {
                if let Some(patch) = strategy.generate(vuln).await? {
                    return Ok(patch);
                }
            }
        }

        // Fallback: generate a manual remediation patch
        Ok(Patch {
            id: Uuid::new_v4(),
            vuln_id: vuln.id,
            description: format!("Manual remediation required for: {}", vuln.title),
            diff: String::new(),
            patch_type: PatchType::ConfigChange,
            confidence: 0.0,
            verified: false,
            certificate: None,
            created_at: Utc::now(),
        })
    }

    async fn refine_patch(&self, patch: &Patch, feedback: &str) -> ShieldResult<Patch> {
        let feedback_lower = feedback.to_lowercase();
        let mut refined = patch.clone();
        refined.id = Uuid::new_v4();
        refined.description = format!("{} [refined: {}]", patch.description, feedback);

        // Apply feedback directives to the diff content
        let mut new_diff = patch.diff.clone();

        // "dry run" / "no execute" — comment out execution lines
        if feedback_lower.contains("dry") || feedback_lower.contains("no exec") {
            new_diff = new_diff.lines().map(|line| {
                let t = line.trim();
                if !t.starts_with('#') && (
                    t.starts_with("ufw ") || t.starts_with("iptables ") ||
                    t.starts_with("systemctl ") || t.starts_with("apt ") ||
                    t.starts_with("pip ") || t.starts_with("npm ") ||
                    t.starts_with("cargo ")
                ) {
                    format!("# [DRY-RUN] {}", line)
                } else {
                    line.to_string()
                }
            }).collect::<Vec<_>>().join("\n");
            refined.confidence = (refined.confidence * 0.9).max(0.0);
        }

        // "rollback" / "undo" — prepend rollback instructions
        if feedback_lower.contains("rollback") || feedback_lower.contains("undo") {
            let rollback = generate_rollback(&patch.diff, patch.patch_type.clone());
            new_diff = format!("# ── ROLLBACK PROCEDURE ──\n{}\n\n# ── ORIGINAL PATCH ──\n{}", rollback, new_diff);
        }

        // "verify" / "test" — append verification steps
        if feedback_lower.contains("verify") || feedback_lower.contains("test") {
            new_diff = format!("{}\n\n# ── VERIFICATION ──\n{}", new_diff,
                generate_verification_steps(&patch.diff, patch.patch_type.clone()));
            refined.confidence = (refined.confidence + 0.05).min(1.0);
        }

        // "port <N>" — replace port numbers in firewall rules
        if let Some(port_str) = extract_port_from_feedback(feedback) {
            new_diff = new_diff.replace(
                &extract_port_from_diff(&patch.diff).unwrap_or_default(),
                &port_str
            );
        }

        refined.diff = new_diff;
        Ok(refined)
    }
}

// ─── Refine helpers ──────────────────────────────────────────────────────────

fn generate_rollback(diff: &str, patch_type: PatchType) -> String {
    match patch_type {
        PatchType::FirewallRule => {
            // Invert ufw deny → ufw allow / iptables DROP → DELETE
            diff.lines().map(|line| {
                let t = line.trim();
                if t.starts_with("ufw deny") {
                    format!("# ROLLBACK: {}", t.replace("deny", "allow"))
                } else if t.contains("-j DROP") {
                    format!("# ROLLBACK: {}", t.replace("-A", "-D").replace("-j DROP", "-j ACCEPT"))
                } else {
                    format!("# {}", line)
                }
            }).collect::<Vec<_>>().join("\n")
        }
        PatchType::DependencyUpdate => {
            // Suggest pinning to previous version
            format!("# To rollback: pin the previous known-good version in your manifest\n# and run your package manager's install/update command.")
        }
        _ => format!("# To rollback: restore the original configuration from backup\n# or version control: git checkout HEAD~1 -- <file>"),
    }
}

fn generate_verification_steps(diff: &str, patch_type: PatchType) -> String {
    match patch_type {
        PatchType::FirewallRule => {
            let port = extract_port_from_diff(diff).unwrap_or_else(|| "<port>".to_string());
            format!(
                "# Verify firewall rule applied:\n\
                 sudo ufw status | grep {}\n\
                 # Or with nmap from external host:\n\
                 nmap -p {} <target_ip>",
                port, port
            )
        }
        PatchType::DependencyUpdate => {
            "# Verify no vulnerable version remains:\n\
             cargo audit   # for Rust\n\
             npm audit     # for Node\n\
             pip-audit     # for Python".to_string()
        }
        _ => "# Verify the change took effect:\n# Check service status, config syntax, and run integration tests.".to_string(),
    }
}

fn extract_port_from_diff(diff: &str) -> Option<String> {
    for line in diff.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        for (i, part) in parts.iter().enumerate() {
            if *part == "port" || *part == "--dport" {
                if let Some(next) = parts.get(i + 1) {
                    return Some(next.to_string());
                }
            }
        }
    }
    None
}

fn extract_port_from_feedback(feedback: &str) -> Option<String> {
    let words: Vec<&str> = feedback.split_whitespace().collect();
    for (i, word) in words.iter().enumerate() {
        if word.to_lowercase() == "port" {
            if let Some(next) = words.get(i + 1) {
                if next.parse::<u16>().is_ok() {
                    return Some(next.to_string());
                }
            }
        }
    }
    None
}
