use async_trait::async_trait;
use shield_core::{
    error::ShieldResult,
    types::*,
    traits::Scanner,
};
use uuid::Uuid;
use chrono::Utc;
use std::path::PathBuf;

pub struct CodeScanner {
    zeus_binary: PathBuf,
}

impl CodeScanner {
    pub fn new(zeus_binary: PathBuf) -> Self {
        Self { zeus_binary }
    }

    /// Run Zeus audit on a source file
    async fn zeus_audit(&self, path: &str) -> ShieldResult<Vec<Vulnerability>> {
        let output = tokio::process::Command::new(&self.zeus_binary)
            .args(["audit", path, "--json"])
            .output()
            .await
            .map_err(|e| shield_core::error::ShieldError::Scan(
                format!("Failed to run zeus audit: {}", e)
            ))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        
        // Parse Zeus audit JSON output for findings
        let mut vulns = Vec::new();
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&stdout) {
            if let Some(findings) = json.get("findings").and_then(|f| f.as_array()) {
                for finding in findings {
                    let title = finding.get("title")
                        .and_then(|t| t.as_str())
                        .unwrap_or("Unknown finding")
                        .to_string();
                    let description = finding.get("description")
                        .and_then(|d| d.as_str())
                        .unwrap_or("")
                        .to_string();
                    let severity = match finding.get("severity")
                        .and_then(|s| s.as_str())
                        .unwrap_or("medium") 
                    {
                        "critical" => Severity::Critical,
                        "high" => Severity::High,
                        "medium" => Severity::Medium,
                        "low" => Severity::Low,
                        _ => Severity::Info,
                    };

                    vulns.push(Vulnerability {
                        id: Uuid::new_v4(),
                        target_id: Uuid::nil(),
                        title,
                        description,
                        severity,
                        category: VulnCategory::CodeVulnerability,
                        cve_id: None,
                        cvss_score: None,
                        evidence: Evidence {
                            raw: finding.to_string(),
                            reproduction_steps: vec![],
                            affected_component: path.to_string(),
                            network_trace: None,
                        },
                        status: VulnStatus::Open,
                        discovered_at: Utc::now(),
                        fixed_at: None,
                    });
                }
            }
        }

        Ok(vulns)
    }

    /// Check dependencies for known CVEs
    async fn check_dependencies(&self, manifest_path: &str) -> ShieldResult<Vec<Vulnerability>> {
        // TODO: Implement dependency CVE checking
        // Parse Cargo.toml/package.json/requirements.txt
        // Cross-reference with NVD/OSV database
        Ok(vec![])
    }

    /// Detect hardcoded secrets in source
    async fn detect_secrets(&self, path: &str) -> ShieldResult<Vec<Vulnerability>> {
        // TODO: Implement secret detection
        // Regex patterns for API keys, tokens, passwords
        Ok(vec![])
    }
}

#[async_trait]
impl Scanner for CodeScanner {
    fn name(&self) -> &str {
        "code-scanner"
    }

    fn supported_targets(&self) -> Vec<TargetKind> {
        vec![TargetKind::Repository, TargetKind::Application]
    }

    async fn discover(&self) -> ShieldResult<Vec<Target>> {
        Ok(vec![])
    }

    async fn scan(&self, target: &Target) -> ShieldResult<Vec<Vulnerability>> {
        let path = &target.address;
        let mut all_vulns = Vec::new();

        // Run Zeus audit
        let audit_vulns = self.zeus_audit(path).await?;
        all_vulns.extend(audit_vulns);

        // Check dependencies
        let dep_vulns = self.check_dependencies(path).await?;
        all_vulns.extend(dep_vulns);

        // Detect secrets
        let secret_vulns = self.detect_secrets(path).await?;
        all_vulns.extend(secret_vulns);

        // Set target_id on all vulns
        for vuln in &mut all_vulns {
            vuln.target_id = target.id;
        }

        Ok(all_vulns)
    }

    async fn health_check(&self) -> ShieldResult<()> {
        // Check if zeus binary exists and is executable
        if !self.zeus_binary.exists() {
            return Err(shield_core::error::ShieldError::Config(
                format!("Zeus binary not found at {:?}", self.zeus_binary)
            ));
        }
        Ok(())
    }
}
