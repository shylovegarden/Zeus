use async_trait::async_trait;
use shield_core::{
    error::ShieldResult,
    types::*,
    traits::Patcher,
};
use uuid::Uuid;
use chrono::Utc;
use crate::strategies::*;

/// Main patch generation engine
/// Combines multiple strategies: rule-based, AI-driven, template-based
pub struct PatchEngine {
    strategies: Vec<Box<dyn PatchStrategy>>,
}

impl PatchEngine {
    pub fn new() -> Self {
        Self {
            strategies: vec![
                Box::new(FirewallRuleStrategy),
                Box::new(ConfigFixStrategy),
                Box::new(DependencyUpdateStrategy),
            ],
        }
    }

    pub fn add_strategy(&mut self, strategy: Box<dyn PatchStrategy>) {
        self.strategies.push(strategy);
    }
}

#[async_trait]
impl Patcher for PatchEngine {
    fn can_fix(&self, vuln: &Vulnerability) -> bool {
        self.strategies.iter().any(|s| s.can_handle(vuln))
    }

    async fn generate_patch(&self, vuln: &Vulnerability) -> ShieldResult<Patch> {
        for strategy in &self.strategies {
            if strategy.can_handle(vuln) {
                if let Some(patch) = strategy.generate(vuln).await? {
                    return Ok(patch);
                }
            }
        }

        // Fallback: generate a manual remediation patch
        Ok(Patch {
            id: Uuid::new_v4(),
            vuln_id: vuln.id,
            description: format!("Manual remediation required for: {}", vuln.title),
            diff: String::new(),
            patch_type: PatchType::ConfigChange,
            confidence: 0.0,
            verified: false,
            certificate: None,
            created_at: Utc::now(),
        })
    }

    async fn refine_patch(&self, patch: &Patch, feedback: &str) -> ShieldResult<Patch> {
        // TODO: Use AI feedback loop to refine patch
        let mut refined = patch.clone();
        refined.id = Uuid::new_v4();
        refined.description = format!("{} (refined: {})", patch.description, feedback);
        Ok(refined)
    }
}
