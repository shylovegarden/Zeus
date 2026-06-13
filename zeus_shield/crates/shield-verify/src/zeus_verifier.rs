use async_trait::async_trait;
use shield_core::{
    error::{ShieldError, ShieldResult},
    types::*,
    traits::Verifier,
};
use uuid::Uuid;
use chrono::Utc;
use sha2::{Sha256, Digest};
use std::path::PathBuf;

/// Zeus-powered formal verification engine
/// Delegates to the Zeus compiler for mathematical proofs
pub struct ZeusVerifier {
    zeus_binary: PathBuf,
}

impl ZeusVerifier {
    pub fn new(zeus_binary: PathBuf) -> Self {
        Self { zeus_binary }
    }

    /// Run Zeus verify command on a source file
    async fn run_zeus_verify(&self, source_path: &str) -> ShieldResult<VerifyOutput> {
        let output = tokio::process::Command::new(&self.zeus_binary)
            .args(["verify", source_path])
            .output()
            .await
            .map_err(|e| ShieldError::Verification(format!("Zeus verify failed: {}", e)))?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let success = output.status.success();

        Ok(VerifyOutput { success, stdout })
    }

    /// Run Zeus audit for detailed property checking
    async fn run_zeus_audit(&self, source_path: &str) -> ShieldResult<serde_json::Value> {
        let output = tokio::process::Command::new(&self.zeus_binary)
            .args(["audit", source_path, "--json"])
            .output()
            .await
            .map_err(|e| ShieldError::Verification(format!("Zeus audit failed: {}", e)))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        serde_json::from_str(&stdout)
            .map_err(|e| ShieldError::Verification(format!("Failed to parse audit output: {}", e)))
    }

    /// Generate a SHA-256 hash of the patch content
    fn hash_patch(&self, patch: &Patch) -> String {
        let mut hasher = Sha256::new();
        hasher.update(patch.diff.as_bytes());
        hasher.update(patch.description.as_bytes());
        hex::encode(hasher.finalize())
    }
}

#[async_trait]
impl Verifier for ZeusVerifier {
    async fn verify(&self, patch: &Patch) -> ShieldResult<Certificate> {
        // Write patch to temp file for verification
        let temp_dir = std::env::temp_dir();
        let patch_file = temp_dir.join(format!("zeus_patch_{}.zs", patch.id));
        
        // For code patches, write the diff content
        if !patch.diff.is_empty() {
            tokio::fs::write(&patch_file, &patch.diff)
                .await
                .map_err(|e| ShieldError::Verification(format!("Failed to write patch: {}", e)))?;
        }

        let patch_path = patch_file.to_string_lossy().to_string();
        let verify_result = self.run_zeus_verify(&patch_path).await?;

        // Clean up temp file
        let _ = tokio::fs::remove_file(&patch_file).await;

        if verify_result.success {
            let hash = self.hash_patch(patch);
            Ok(Certificate {
                id: Uuid::new_v4(),
                patch_id: patch.id,
                properties_proven: vec![
                    "memory_safe".to_string(),
                    "no_undefined_behavior".to_string(),
                    "bounds_verified".to_string(),
                ],
                verifier_version: "zeus-0.1.0".to_string(),
                signature: hash,
                issued_at: Utc::now(),
            })
        } else {
            Err(ShieldError::Verification(format!(
                "Verification failed: {}",
                verify_result.stdout
            )))
        }
    }

    async fn check_property(&self, patch: &Patch, property: &str) -> ShieldResult<bool> {
        // Map property names to Zeus verification flags
        let flag = match property {
            "constant_time" => "--require=constant-time",
            "zero_heap" => "--require=zero-heap",
            "deterministic" => "--require=deterministic",
            "bounded_wcet" => "--require=bounded",
            _ => return Ok(false),
        };

        let temp_dir = std::env::temp_dir();
        let patch_file = temp_dir.join(format!("zeus_prop_{}.zs", patch.id));
        tokio::fs::write(&patch_file, &patch.diff)
            .await
            .map_err(|e| ShieldError::Verification(format!("Write failed: {}", e)))?;

        let output = tokio::process::Command::new(&self.zeus_binary)
            .args(["verify", &patch_file.to_string_lossy(), flag])
            .output()
            .await
            .map_err(|e| ShieldError::Verification(format!("Zeus verify failed: {}", e)))?;

        let _ = tokio::fs::remove_file(&patch_file).await;
        Ok(output.status.success())
    }

    async fn provable_properties(&self, _patch: &Patch) -> ShieldResult<Vec<String>> {
        Ok(vec![
            "memory_safe".to_string(),
            "no_undefined_behavior".to_string(),
            "bounds_verified".to_string(),
            "constant_time".to_string(),
            "zero_heap".to_string(),
            "deterministic".to_string(),
            "bounded_wcet".to_string(),
        ])
    }
}

struct VerifyOutput {
    success: bool,
    stdout: String,
}

mod hex {
    pub fn encode(bytes: impl AsRef<[u8]>) -> String {
        bytes.as_ref().iter().map(|b| format!("{:02x}", b)).collect()
    }
}
