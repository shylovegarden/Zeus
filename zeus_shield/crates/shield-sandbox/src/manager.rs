use shield_core::{
    config::SandboxConfig,
    error::ShieldResult,
    types::*,
    traits::Sandbox,
};
use crate::docker::DockerSandbox;
use std::collections::HashMap;
use uuid::Uuid;

/// Manages multiple sandbox instances
pub struct SandboxManager {
    config: SandboxConfig,
    active_sandboxes: HashMap<Uuid, Box<dyn Sandbox>>,
}

impl SandboxManager {
    pub fn new(config: SandboxConfig) -> Self {
        Self {
            config,
            active_sandboxes: HashMap::new(),
        }
    }

    pub async fn create_for_target(&mut self, target: &Target) -> ShieldResult<Uuid> {
        if self.active_sandboxes.len() >= self.config.max_concurrent {
            return Err(shield_core::error::ShieldError::Sandbox(
                format!("Max concurrent sandboxes ({}) reached", self.config.max_concurrent)
            ));
        }

        let mut sandbox = Box::new(DockerSandbox::new());
        sandbox.create(target).await?;

        let id = Uuid::new_v4();
        self.active_sandboxes.insert(id, sandbox);
        Ok(id)
    }

    pub async fn destroy(&mut self, id: &Uuid) -> ShieldResult<()> {
        if let Some(mut sandbox) = self.active_sandboxes.remove(id) {
            sandbox.destroy().await?;
        }
        Ok(())
    }

    pub async fn destroy_all(&mut self) -> ShieldResult<()> {
        let ids: Vec<Uuid> = self.active_sandboxes.keys().cloned().collect();
        for id in ids {
            self.destroy(&id).await?;
        }
        Ok(())
    }

    pub fn active_count(&self) -> usize {
        self.active_sandboxes.len()
    }
}
