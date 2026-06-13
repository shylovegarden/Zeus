use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ─── Target ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Target {
    pub id: Uuid,
    pub name: String,
    pub kind: TargetKind,
    pub address: String,
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TargetKind {
    Host,
    Network,
    Container,
    CloudInstance,
    KubernetesCluster,
    Repository,
    Application,
    IoTDevice,
    Router,
    Firewall,
}

// ─── Vulnerability ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vulnerability {
    pub id: Uuid,
    pub target_id: Uuid,
    pub title: String,
    pub description: String,
    pub severity: Severity,
    pub category: VulnCategory,
    pub cve_id: Option<String>,
    pub cvss_score: Option<f64>,
    pub evidence: Evidence,
    pub status: VulnStatus,
    pub discovered_at: DateTime<Utc>,
    pub fixed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    Critical,
    High,
    Medium,
    Low,
    Info,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum VulnCategory {
    NetworkExposure,
    CodeVulnerability,
    Misconfiguration,
    OutdatedSoftware,
    WeakCredentials,
    DataExposure,
    InjectionFlaw,
    BufferOverflow,
    TimingLeak,
    PrivilegeEscalation,
    DenialOfService,
    SupplyChain,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Evidence {
    pub raw: String,
    pub reproduction_steps: Vec<String>,
    pub affected_component: String,
    pub network_trace: Option<Vec<u8>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum VulnStatus {
    Open,
    Confirmed,
    Reproducing,
    Patching,
    Verifying,
    Fixed,
    Accepted,
    FalsePositive,
}

// ─── Patch ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Patch {
    pub id: Uuid,
    pub vuln_id: Uuid,
    pub description: String,
    pub diff: String,
    pub patch_type: PatchType,
    pub confidence: f64,
    pub verified: bool,
    pub certificate: Option<Certificate>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PatchType {
    CodeFix,
    ConfigChange,
    DependencyUpdate,
    FirewallRule,
    AccessControl,
    NetworkPolicy,
}

// ─── Certificate (Zeus Verification) ────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Certificate {
    pub id: Uuid,
    pub patch_id: Uuid,
    pub properties_proven: Vec<String>,
    pub verifier_version: String,
    pub signature: String,
    pub issued_at: DateTime<Utc>,
}

// ─── Scan Job ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanJob {
    pub id: Uuid,
    pub targets: Vec<Target>,
    pub scan_type: ScanType,
    pub status: JobStatus,
    pub findings: Vec<Vulnerability>,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ScanType {
    NetworkScan,
    PortScan,
    CodeAudit,
    DependencyCheck,
    ConfigAudit,
    FullSuite,
    Continuous,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum JobStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

// ─── Agent Info ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInfo {
    pub id: Uuid,
    pub hostname: String,
    pub os: String,
    pub arch: String,
    pub version: String,
    pub capabilities: Vec<String>,
    pub last_heartbeat: DateTime<Utc>,
    pub status: AgentStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AgentStatus {
    Online,
    Offline,
    Scanning,
    Patching,
    Error,
}
