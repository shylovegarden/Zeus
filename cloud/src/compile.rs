// Compilation module for Zeus Cloud

use std::process::Command;
use std::path::Path;
use tokio::fs;
use anyhow::{Result, Context};
use serde::{Serialize, Deserialize};
use sha2::{Sha256, Digest};

#[derive(Debug, Deserialize)]
pub struct CompileRequest {
    pub source: String,
    pub target: String,
    pub policy: Option<String>,
    pub verify: bool,
    pub generate_cert: bool,
}

#[derive(Debug, Serialize)]
pub struct CompilationResult {
    pub success: bool,
    pub binary: Option<Vec<u8>>,
    pub certificate: Option<String>,
    pub verification_report: Option<VerificationReport>,
    pub gas_estimate: Option<u64>,
    pub output: String,
    pub errors: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct VerificationReport {
    pub verified: bool,
    pub properties: Vec<String>,
    pub zir_summary: String,
}

/// Run Zeus compilation in isolated environment
pub async fn compile_request(req: &CompileRequest) -> Result<CompilationResult> {
    let job_id = generate_job_id(&req.source);
    let work_dir = format!("/tmp/zeus_cloud/{}", job_id);
    
    // Create work directory
    fs::create_dir_all(&work_dir).await
        .context("Failed to create work directory")?;
    
    // Write source file
    let source_path = format!("{}/main.zs", work_dir);
    fs::write(&source_path, &req.source).await
        .context("Failed to write source file")?;
    
    // Build compilation command
    let mut cmd = Command::new("zeus");
    cmd.arg("build")
        .arg(&source_path)
        .current_dir(&work_dir);
    
    // Add target
    if req.target != "x86_64" {
        cmd.arg(format!("--target={}", req.target));
    }
    
    // Add policy if specified
    if let Some(policy) = &req.policy {
        let policy_path = format!("{}/policy.txt", work_dir);
        fs::write(&policy_path, policy).await?;
        cmd.arg(format!("--policy={}", policy_path));
    }
    
    // Add verification
    if req.verify {
        cmd.arg("--verify");
    }
    
    // Add certificate generation
    if req.generate_cert {
        cmd.arg("--cert");
    }
    
    // Execute compilation
    let output = tokio::task::spawn_blocking(move || {
        cmd.output()
    }).await??;
    
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    
    // Parse result
    let success = output.status.success();
    
    // Read generated files
    let mut binary = None;
    let mut certificate = None;
    let mut verification_report = None;
    let mut gas_estimate = None;
    
    if success {
        // Read binary
        let binary_path = format!("{}/main", work_dir);
        if Path::new(&binary_path).exists() {
            binary = Some(fs::read(&binary_path).await?);
        }
        
        // Read certificate
        let cert_path = format!("{}/main.zcert", work_dir);
        if Path::new(&cert_path).exists() {
            certificate = Some(fs::read_to_string(&cert_path).await?);
        }
        
        // Parse verification report from stdout
        if req.verify {
            verification_report = Some(parse_verification_report(&stdout));
        }
        
        // Parse gas estimate for EVM target
        if req.target == "evm" {
            gas_estimate = parse_gas_estimate(&stdout);
        }
    }
    
    // Collect errors
    let errors = if !success {
        stderr.lines().map(|s| s.to_string()).collect()
    } else {
        vec![]
    };
    
    // Cleanup
    let _ = fs::remove_dir_all(&work_dir).await;
    
    Ok(CompilationResult {
        success,
        binary,
        certificate,
        verification_report,
        gas_estimate,
        output: stdout,
        errors,
    })
}

/// Generate unique job ID from source hash
fn generate_job_id(source: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(source.as_bytes());
    let result = hasher.finalize();
    format!("{:x}", &result[..8])
}

/// Parse verification report from Zeus output
fn parse_verification_report(output: &str) -> VerificationReport {
    let mut verified = false;
    let mut properties = vec![];
    
    for line in output.lines() {
        if line.contains("Verification passed") || line.contains("✓") {
            verified = true;
        }
        if line.contains("zero-heap") {
            properties.push("zero-heap".to_string());
        }
        if line.contains("constant-time") {
            properties.push("constant-time".to_string());
        }
        if line.contains("deterministic") {
            properties.push("deterministic".to_string());
        }
    }
    
    VerificationReport {
        verified,
        properties,
        zir_summary: "See full ZIR report".to_string(),
    }
}

/// Parse gas estimate from Zeus output
fn parse_gas_estimate(output: &str) -> Option<u64> {
    for line in output.lines() {
        if line.contains("Gas estimate:") {
            if let Some(num) = line.split(':').nth(1) {
                return num.trim().parse().ok();
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_job_id_generation() {
        let id1 = generate_job_id("test source");
        let id2 = generate_job_id("test source");
        let id3 = generate_job_id("different source");
        
        assert_eq!(id1, id2);
        assert_ne!(id1, id3);
    }
}
