use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShieldConfig {
    pub agent: AgentConfig,
    pub console: ConsoleConfig,
    pub scanner: ScannerConfig,
    pub sandbox: SandboxConfig,
    pub plugins: Vec<PluginConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub id: Option<String>,
    pub console_url: String,
    pub heartbeat_interval_secs: u64,
    pub log_level: String,
    pub data_dir: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsoleConfig {
    pub bind_address: String,
    pub port: u16,
    pub tls_cert: Option<PathBuf>,
    pub tls_key: Option<PathBuf>,
    pub database_url: String,
    pub jwt_secret: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScannerConfig {
    pub concurrent_scans: usize,
    pub timeout_secs: u64,
    pub port_range: String,
    pub excluded_hosts: Vec<String>,
    pub scan_interval_secs: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxConfig {
    pub runtime: SandboxRuntime,
    pub max_concurrent: usize,
    pub timeout_secs: u64,
    pub network_isolated: bool,
    pub resource_limits: ResourceLimits,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SandboxRuntime {
    Docker,
    Firecracker,
    Nsjail,
    Bubblewrap,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    pub max_memory_mb: u64,
    pub max_cpu_percent: u64,
    pub max_disk_mb: u64,
    pub max_network_bandwidth_mbps: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginConfig {
    pub name: String,
    pub enabled: bool,
    pub path: Option<PathBuf>,
    pub config: serde_json::Value,
}

impl Default for ShieldConfig {
    fn default() -> Self {
        Self {
            agent: AgentConfig {
                id: None,
                console_url: "https://localhost:8443".to_string(),
                heartbeat_interval_secs: 30,
                log_level: "info".to_string(),
                data_dir: PathBuf::from("/var/lib/zeus-shield"),
            },
            console: ConsoleConfig {
                bind_address: "0.0.0.0".to_string(),
                port: 8443,
                tls_cert: None,
                tls_key: None,
                database_url: "sqlite:///var/lib/zeus-shield/shield.db".to_string(),
                jwt_secret: "CHANGE_ME_IN_PRODUCTION".to_string(),
            },
            scanner: ScannerConfig {
                concurrent_scans: 10,
                timeout_secs: 300,
                port_range: "1-65535".to_string(),
                excluded_hosts: vec![],
                scan_interval_secs: 3600,
            },
            sandbox: SandboxConfig {
                runtime: SandboxRuntime::Docker,
                max_concurrent: 5,
                timeout_secs: 600,
                network_isolated: true,
                resource_limits: ResourceLimits {
                    max_memory_mb: 512,
                    max_cpu_percent: 50,
                    max_disk_mb: 1024,
                    max_network_bandwidth_mbps: 10,
                },
            },
            plugins: vec![],
        }
    }
}
