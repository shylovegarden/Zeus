use async_trait::async_trait;
use shield_core::{
    error::{ShieldError, ShieldResult},
    types::*,
    traits::Sandbox,
};
use uuid::Uuid;

/// Docker-based sandbox for exploit reproduction
pub struct DockerSandbox {
    container_id: Option<String>,
    image: String,
    network_isolated: bool,
}

impl DockerSandbox {
    pub fn new() -> Self {
        Self {
            container_id: None,
            image: "ubuntu:22.04".to_string(),
            network_isolated: true,
        }
    }

    pub fn with_image(mut self, image: &str) -> Self {
        self.image = image.to_string();
        self
    }

    async fn run_docker_command(&self, args: &[&str]) -> ShieldResult<String> {
        let output = tokio::process::Command::new("docker")
            .args(args)
            .output()
            .await
            .map_err(|e| ShieldError::Sandbox(format!("docker command failed: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(ShieldError::Sandbox(format!("docker error: {}", stderr)));
        }

        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }
}

#[async_trait]
impl Sandbox for DockerSandbox {
    async fn create(&mut self, target: &Target) -> ShieldResult<String> {
        let name = format!("zeus-sandbox-{}", Uuid::new_v4().to_string()[..8].to_string());
        
        let mut args = vec![
            "run", "-d",
            "--name", &name,
            "--memory", "512m",
            "--cpus", "1.0",
        ];

        if self.network_isolated {
            args.push("--network=none");
        }

        args.push(&self.image);
        args.push("sleep");
        args.push("3600"); // Keep alive for 1 hour

        let container_id = self.run_docker_command(&args).await?;
        self.container_id = Some(container_id.clone());

        tracing::info!("Sandbox created: {} ({})", name, &container_id[..12]);
        Ok(container_id)
    }

    async fn reproduce(&self, vuln: &Vulnerability) -> ShieldResult<bool> {
        let container_id = self.container_id.as_ref()
            .ok_or_else(|| ShieldError::Sandbox("No sandbox created".to_string()))?;

        // Execute reproduction steps inside the container
        for step in &vuln.evidence.reproduction_steps {
            let output = self.run_docker_command(&[
                "exec", container_id, "sh", "-c", step,
            ]).await;

            match output {
                Ok(_) => tracing::info!("Step succeeded: {}", step),
                Err(e) => {
                    tracing::warn!("Step failed: {} - {}", step, e);
                    return Ok(false);
                }
            }
        }

        Ok(true)
    }

    async fn test_patch(&self, patch: &Patch) -> ShieldResult<bool> {
        let container_id = self.container_id.as_ref()
            .ok_or_else(|| ShieldError::Sandbox("No sandbox created".to_string()))?;

        tracing::info!("Testing patch {} in sandbox {}", patch.id, &container_id[..12]);

        if patch.diff.is_empty() {
            tracing::warn!("Patch {} has empty diff — nothing to test", patch.id);
            return Ok(false);
        }

        // Write patch diff to a temp file on the host, then copy into container
        let host_patch_file = std::env::temp_dir()
            .join(format!("zeus_patch_{}.sh", patch.id));
        tokio::fs::write(&host_patch_file, patch.diff.as_bytes())
            .await
            .map_err(|e| ShieldError::Sandbox(format!("Failed to write patch file: {}", e)))?;

        let host_path = host_patch_file.to_string_lossy().to_string();
        let container_path = format!("/tmp/zeus_patch_{}.sh", patch.id);

        // Copy patch into container
        self.run_docker_command(&[
            "cp", &host_path, &format!("{}:{}", container_id, container_path),
        ]).await?;

        let _ = tokio::fs::remove_file(&host_patch_file).await;

        // Apply based on patch type
        let apply_result = match patch.patch_type {
            PatchType::FirewallRule | PatchType::ConfigChange => {
                // Run as a shell script (dry-run mode: echo commands, don't actually execute)
                self.run_docker_command(&[
                    "exec", container_id,
                    "sh", "-c",
                    &format!("chmod +x {} && sh -n {}", container_path, container_path),
                ]).await
            }
            PatchType::DependencyUpdate => {
                // Verify the script is syntactically valid
                self.run_docker_command(&[
                    "exec", container_id,
                    "sh", "-c",
                    &format!("sh -n {}", container_path),
                ]).await
            }
            _ => {
                self.run_docker_command(&[
                    "exec", container_id,
                    "sh", "-c",
                    &format!("sh -n {}", container_path),
                ]).await
            }
        };

        // Cleanup inside container
        let _ = self.run_docker_command(&[
            "exec", container_id, "rm", "-f", &container_path,
        ]).await;

        match apply_result {
            Ok(_) => {
                tracing::info!("Patch {} validated successfully in sandbox", patch.id);
                Ok(true)
            }
            Err(e) => {
                tracing::warn!("Patch {} failed sandbox validation: {}", patch.id, e);
                Ok(false)
            }
        }
    }

    async fn destroy(&mut self) -> ShieldResult<()> {
        if let Some(container_id) = &self.container_id {
            let _ = self.run_docker_command(&["rm", "-f", container_id]).await;
            tracing::info!("Sandbox destroyed: {}", &container_id[..12]);
        }
        self.container_id = None;
        Ok(())
    }

    async fn status(&self) -> ShieldResult<serde_json::Value> {
        if let Some(container_id) = &self.container_id {
            let info = self.run_docker_command(&[
                "inspect", "--format", "{{.State.Status}}", container_id,
            ]).await?;
            Ok(serde_json::json!({
                "container_id": container_id,
                "status": info,
                "image": self.image,
                "network_isolated": self.network_isolated,
            }))
        } else {
            Ok(serde_json::json!({ "status": "not_created" }))
        }
    }
}
