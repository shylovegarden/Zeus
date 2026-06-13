use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShieldEvent {
    pub id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub source: String,
    pub event_type: EventType,
    pub payload: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum EventType {
    // Agent lifecycle
    AgentConnected,
    AgentDisconnected,
    AgentHeartbeat,

    // Scanning
    ScanStarted,
    ScanCompleted,
    VulnerabilityFound,
    VulnerabilityConfirmed,

    // Sandbox
    SandboxCreated,
    SandboxDestroyed,
    ExploitReproduced,
    ExploitFailed,

    // Patching
    PatchGenerated,
    PatchApplied,
    PatchRolledBack,
    PatchFailed,

    // Verification
    VerificationStarted,
    VerificationPassed,
    VerificationFailed,
    CertificateIssued,

    // System
    ConfigChanged,
    PluginLoaded,
    PluginUnloaded,
    Error,
}

impl ShieldEvent {
    pub fn new(source: &str, event_type: EventType, payload: serde_json::Value) -> Self {
        Self {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            source: source.to_string(),
            event_type,
            payload,
        }
    }
}
