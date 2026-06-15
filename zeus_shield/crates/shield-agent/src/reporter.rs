use reqwest::Client;
use shield_core::types::{AgentInfo, AgentStatus, Vulnerability};
use tracing::{error, info, warn};
use uuid::Uuid;

#[derive(Clone)]
pub struct ConsoleReporter {
    client: Client,
    pub console_url: String,
    agent_id: Uuid,
    hostname: String,
}

impl ConsoleReporter {
    pub fn new(console_url: &str, agent_id: Uuid) -> Self {
        let hostname = std::process::Command::new("hostname")
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .unwrap_or_else(|_| "unknown".to_string());

        Self {
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .expect("failed to build HTTP client"),
            console_url: console_url.trim_end_matches('/').to_string(),
            agent_id,
            hostname,
        }
    }

    pub async fn send_heartbeat(&self, status: AgentStatus) {
        let info = AgentInfo {
            id: self.agent_id,
            hostname: self.hostname.clone(),
            os: std::env::consts::OS.to_string(),
            arch: std::env::consts::ARCH.to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            capabilities: vec![
                "network-scan".to_string(),
                "device-scan".to_string(),
            ],
            last_heartbeat: chrono::Utc::now(),
            status,
        };

        let url = format!("{}/api/v1/agents/heartbeat", self.console_url);
        match self.client.post(&url).json(&info).send().await {
            Ok(resp) if resp.status().is_success() => {
                info!("Heartbeat sent to console");
            }
            Ok(resp) => {
                warn!("Console returned HTTP {}", resp.status());
            }
            Err(e) => {
                warn!("Failed to reach console: {} — working offline", e);
            }
        }
    }

    pub async fn report_vulnerabilities(&self, vulns: &[Vulnerability]) {
        if vulns.is_empty() {
            return;
        }

        info!("Reporting {} findings to console", vulns.len());

        for vuln in vulns {
            let url = format!("{}/api/v1/vulnerabilities", self.console_url);
            match self.client.post(&url).json(vuln).send().await {
                Ok(resp) if resp.status().is_success() => {}
                Ok(resp) => {
                    warn!("Console rejected finding: HTTP {}", resp.status());
                }
                Err(e) => {
                    warn!("Failed to report finding: {}", e);
                }
            }
        }

        info!("All findings reported");
    }

    pub async fn fetch_pending_jobs(&self) -> Vec<serde_json::Value> {
        let url = format!("{}/api/v1/scans?agent_id={}&status=pending", self.console_url, self.agent_id);
        match self.client.get(&url).send().await {
            Ok(resp) if resp.status().is_success() => {
                resp.json::<Vec<serde_json::Value>>().await.unwrap_or_default()
            }
            _ => vec![],
        }
    }
}
