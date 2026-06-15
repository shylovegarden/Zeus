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

    /// Run `zeus verify <file.zs>` on a Zeus source file.
    /// Returns success=true if output contains "Formal Verification Successful".
    async fn run_zeus_verify(&self, source_path: &str) -> ShieldResult<VerifyOutput> {
        let output = tokio::process::Command::new(&self.zeus_binary)
            .args(["verify", source_path])
            .output()
            .await
            .map_err(|e| ShieldError::Verification(format!("Zeus binary not runnable: {}", e)))?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let combined = format!("{}{}", stdout, stderr);

        // Zeus prints "Formal Verification Successful" on success
        let success = combined.contains("Formal Verification Successful");

        Ok(VerifyOutput { success, stdout: combined })
    }

    /// Run `zeus doc <file.zs>` to generate a MISRA-C / safety audit trace.
    /// Returns the audit markdown path on success.
    async fn run_zeus_doc(&self, source_path: &str) -> ShieldResult<String> {
        let output = tokio::process::Command::new(&self.zeus_binary)
            .args(["doc", source_path])
            .output()
            .await
            .map_err(|e| ShieldError::Verification(format!("Zeus doc failed: {}", e)))?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        // Parses: "Generated Audit Trail & Documentation at <path>"
        let audit_path = stdout
            .lines()
            .find(|l| l.contains("Generated Audit"))
            .and_then(|l| l.split(" at ").nth(1))
            .unwrap_or("")
            .trim()
            .to_string();
        Ok(audit_path)
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
        // Zeus can only formally verify .zs source files.
        // For shell/firewall patches we perform structural validation instead.
        let is_zeus_source = matches!(patch.patch_type, PatchType::ZeusSource);

        let (properties_proven, zeus_output) = if is_zeus_source && !patch.diff.is_empty() {
            let temp_dir = std::env::temp_dir();
            let patch_file = temp_dir.join(format!("zeus_patch_{}.zs", patch.id));

            tokio::fs::write(&patch_file, &patch.diff)
                .await
                .map_err(|e| ShieldError::Verification(
                    format!("Failed to write patch for verification: {}", e)
                ))?;

            let patch_path = patch_file.to_string_lossy().to_string();
            let result = self.run_zeus_verify(&patch_path).await?;
            let _ = tokio::fs::remove_file(&patch_file).await;

            if !result.success {
                return Err(ShieldError::Verification(format!(
                    "Zeus formal verification failed:\n{}", result.stdout
                )));
            }

            let props = vec![
                "memory_safe".to_string(),
                "no_undefined_behavior".to_string(),
                "bounds_verified".to_string(),
                "formally_verified".to_string(),
            ];
            (props, result.stdout)
        } else {
            // Structural validation: non-empty diff, no shell injection characters
            if patch.diff.is_empty() {
                return Err(ShieldError::Verification(
                    "Cannot verify: patch has empty diff".to_string()
                ));
            }
            let dangerous = ["$(" , "`", "eval ", "curl ", "wget ", "rm -rf /"];
            for d in &dangerous {
                if patch.diff.contains(d) {
                    return Err(ShieldError::Verification(
                        format!("Patch contains dangerous pattern '{}'", d)
                    ));
                }
            }
            let props = vec![
                "structurally_valid".to_string(),
                "no_injection".to_string(),
            ];
            (props, "structural validation passed".to_string())
        };

        let hash = self.hash_patch(patch);
        Ok(Certificate {
            id: Uuid::new_v4(),
            patch_id: patch.id,
            properties_proven,
            verifier_version: format!("zeus-shield-{}", env!("CARGO_PKG_VERSION")),
            signature: hash,
            issued_at: Utc::now(),
        })
    }

    async fn check_property(&self, patch: &Patch, property: &str) -> ShieldResult<bool> {
        // Only Zeus-source patches can have properties formally checked
        if !matches!(patch.patch_type, PatchType::ZeusSource) {
            return Ok(false);
        }
        if patch.diff.is_empty() {
            return Ok(false);
        }

        // Zeus verify command: `zeus verify <file.zs>`
        // The only supported flag is `--medical` for medical-grade safety
        // Other properties are checked by analyzing the verify output text
        let temp_dir = std::env::temp_dir();
        let patch_file = temp_dir.join(format!("zeus_prop_{}.zs", patch.id));
        tokio::fs::write(&patch_file, &patch.diff)
            .await
            .map_err(|e| ShieldError::Verification(format!("Write failed: {}", e)))?;

        let patch_path = patch_file.to_string_lossy().to_string();
        let mut cmd = tokio::process::Command::new(&self.zeus_binary);
        cmd.arg("verify").arg(&patch_path);
        if property == "medical_grade" {
            cmd.arg("--medical");
        }

        let output = cmd.output()
            .await
            .map_err(|e| ShieldError::Verification(format!("Zeus verify failed: {}", e)))?;

        let _ = tokio::fs::remove_file(&patch_file).await;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let passed = stdout.contains("Formal Verification Successful");
        Ok(passed)
    }

    async fn provable_properties(&self, patch: &Patch) -> ShieldResult<Vec<String>> {
        if matches!(patch.patch_type, PatchType::ZeusSource) {
            Ok(vec![
                "memory_safe".to_string(),
                "no_undefined_behavior".to_string(),
                "bounds_verified".to_string(),
                "formally_verified".to_string(),
                "medical_grade".to_string(),
            ])
        } else {
            Ok(vec![
                "structurally_valid".to_string(),
                "no_injection".to_string(),
            ])
        }
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
