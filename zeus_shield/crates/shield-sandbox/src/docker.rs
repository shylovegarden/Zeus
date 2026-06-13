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

        // Apply patch inside sandbox
        // TODO: Write diff to container, apply with patch command
        tracing::info!("Testing patch {} in sandbox", patch.id);

        // Run validation tests
        // TODO: Execute test suite after patch application

        Ok(true)
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
