use async_trait::async_trait;
use crate::error::ShieldResult;
use crate::types::*;

// ─── Scanner Trait ──────────────────────────────────────────────────────────
// Any scanning module (network, code, cloud, device) implements this.

#[async_trait]
pub trait Scanner: Send + Sync {
    /// Human-readable name of this scanner
    fn name(&self) -> &str;

    /// What kinds of targets this scanner supports
    fn supported_targets(&self) -> Vec<TargetKind>;

    /// Discover targets automatically (e.g., network discovery)
    async fn discover(&self) -> ShieldResult<Vec<Target>>;

    /// Scan a specific target for vulnerabilities
    async fn scan(&self, target: &Target) -> ShieldResult<Vec<Vulnerability>>;

    /// Check if scanner is healthy and ready
    async fn health_check(&self) -> ShieldResult<()>;
}

// ─── Connector Trait ────────────────────────────────────────────────────────
// Connects to external systems (AWS, SSH, K8s, Docker, etc.)

#[async_trait]
pub trait Connector: Send + Sync {
    /// Human-readable name of this connector
    fn name(&self) -> &str;

    /// Connect to the target system
    async fn connect(&mut self) -> ShieldResult<()>;

    /// Disconnect from the target system
    async fn disconnect(&mut self) -> ShieldResult<()>;

    /// Check connection health
    async fn is_connected(&self) -> bool;

    /// Collect system telemetry (OS info, services, configs)
    async fn collect_telemetry(&self) -> ShieldResult<serde_json::Value>;

    /// Apply a patch to the connected system
    async fn apply_patch(&self, patch: &Patch) -> ShieldResult<()>;

    /// Rollback a previously applied patch
    async fn rollback_patch(&self, patch: &Patch) -> ShieldResult<()>;
}

// ─── Sandbox Trait ──────────────────────────────────────────────────────────
// Isolated environment for exploit reproduction and patch testing.

#[async_trait]
pub trait Sandbox: Send + Sync {
    /// Create an isolated environment matching the target
    async fn create(&mut self, target: &Target) -> ShieldResult<String>;

    /// Reproduce a vulnerability in the sandbox
    async fn reproduce(&self, vuln: &Vulnerability) -> ShieldResult<bool>;

    /// Apply and test a patch in the sandbox
    async fn test_patch(&self, patch: &Patch) -> ShieldResult<bool>;

    /// Destroy the sandbox environment
    async fn destroy(&mut self) -> ShieldResult<()>;

    /// Get sandbox status
    async fn status(&self) -> ShieldResult<serde_json::Value>;
}

// ─── Patcher Trait ──────────────────────────────────────────────────────────
// Generates fixes for vulnerabilities.

#[async_trait]
pub trait Patcher: Send + Sync {
    /// Check if this patcher can fix the given vulnerability
    fn can_fix(&self, vuln: &Vulnerability) -> bool;

    /// Generate a candidate patch
    async fn generate_patch(&self, vuln: &Vulnerability) -> ShieldResult<Patch>;

    /// Refine a patch based on test results
    async fn refine_patch(&self, patch: &Patch, feedback: &str) -> ShieldResult<Patch>;
}

// ─── Verifier Trait ─────────────────────────────────────────────────────────
// Formally verifies patches using Zeus.

#[async_trait]
pub trait Verifier: Send + Sync {
    /// Verify that a patch is correct and safe
    async fn verify(&self, patch: &Patch) -> ShieldResult<Certificate>;

    /// Check specific properties (constant-time, zero-heap, etc.)
    async fn check_property(&self, patch: &Patch, property: &str) -> ShieldResult<bool>;

    /// Get all provable properties for a patch
    async fn provable_properties(&self, patch: &Patch) -> ShieldResult<Vec<String>>;
}

// ─── Plugin Registry ────────────────────────────────────────────────────────
// Dynamic plugin loading and management.

pub trait Plugin: Send + Sync {
    fn name(&self) -> &str;
    fn version(&self) -> &str;
    fn description(&self) -> &str;
    fn plugin_type(&self) -> PluginType;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PluginType {
    Scanner,
    Connector,
    Sandbox,
    Patcher,
    Verifier,
    Reporter,
}
