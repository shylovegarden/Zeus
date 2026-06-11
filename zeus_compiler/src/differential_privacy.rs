// Differential Privacy Module
// Implements epsilon-differential privacy with noise injection

use crate::ast::{Program, Statement, Expression, Type};
use std::collections::HashMap;

/// Privacy mechanism type
#[derive(Debug, Clone)]
pub enum PrivacyMechanism {
    Laplace { epsilon: f64, sensitivity: f64 },
    Gaussian { epsilon: f64, delta: f64, sensitivity: f64 },
    Exponential { epsilon: f64, utility_function: String },
}

/// Differential privacy configuration
#[derive(Debug)]
pub struct PrivacyConfig {
    pub epsilon: f64,
    pub delta: Option<f64>,
    pub mechanism: PrivacyMechanism,
    pub query_budget: f64,  // Total epsilon budget
    pub spent_budget: f64,  // Epsilon already spent
}

impl PrivacyConfig {
    pub fn new(epsilon: f64) -> Self {
        PrivacyConfig {
            epsilon,
            delta: None,
            mechanism: PrivacyMechanism::Laplace { epsilon, sensitivity: 1.0 },
            query_budget: epsilon,
            spent_budget: 0.0,
        }
    }
    
    pub fn with_delta(mut self, delta: f64) -> Self {
        self.delta = Some(delta);
        self.mechanism = PrivacyMechanism::Gaussian { 
            epsilon: self.epsilon, 
            delta, 
            sensitivity: 1.0 
        };
        self
    }
    
    /// Check if query can be executed within budget
    pub fn can_query(&self, query_epsilon: f64) -> bool {
        self.spent_budget + query_epsilon <= self.query_budget
    }
    
    /// Spend epsilon budget
    pub fn spend_budget(&mut self, epsilon: f64) -> Result<(), String> {
        if !self.can_query(epsilon) {
            return Err(format!(
                "Privacy budget exceeded: {:.3} + {:.3} > {:.3}",
                self.spent_budget, epsilon, self.query_budget
            ));
        }
        self.spent_budget += epsilon;
        Ok(())
    }
    
    /// Generate Laplace noise
    pub fn laplace_noise(&self, sensitivity: f64) -> f64 {
        let scale = sensitivity / self.epsilon;
        // Inverse transform sampling for Laplace
        let u: f64 = random_uniform();  // -0.5 to 0.5
        let sign = if u < 0.0 { -1.0 } else { 1.0 };
        -scale * sign * (1.0 - 2.0 * u.abs()).ln()
    }
    
    /// Generate Gaussian noise
    pub fn gaussian_noise(&self, sensitivity: f64) -> f64 {
        if let Some(delta) = self.delta {
            let sigma = sensitivity * (2.0 * (1.25 / delta).ln()).sqrt() / self.epsilon;
            // Box-Muller transform
            let u1: f64 = random_uniform();
            let u2: f64 = random_uniform();
            sigma * (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos()
        } else {
            self.laplace_noise(sensitivity)
        }
    }
}

/// Differential privacy analyzer
pub struct PrivacyAnalyzer {
    config: PrivacyConfig,
    sensitivities: HashMap<String, f64>,
    query_count: u32,
}

impl PrivacyAnalyzer {
    pub fn new(config: PrivacyConfig) -> Self {
        PrivacyAnalyzer {
            config,
            sensitivities: HashMap::new(),
            query_count: 0,
        }
    }
    
    /// Analyze program for privacy compliance
    pub fn analyze(&mut self, program: &Program) -> Result<PrivacyReport, String> {
        let mut report = PrivacyReport::new();
        
        for stmt in &program.statements {
            self.analyze_statement(stmt, &mut report)?;
        }
        
        report.final_epsilon = self.config.spent_budget;
        report.remaining_budget = self.config.query_budget - self.config.spent_budget;
        
        Ok(report)
    }
    
    fn analyze_statement(&mut self, stmt: &Statement, report: &mut PrivacyReport) -> Result<(), String> {
        match stmt {
            Statement::FunctionDeclaration { name, attributes, body, .. } => {
                // Check for @differential_privacy attribute
                for attr in attributes {
                    if let crate::ast::FunctionAttribute::Custom(s) = attr {
                        if s.starts_with("differential_privacy") {
                            let epsilon = parse_epsilon(s)?;
                            self.config.spend_budget(epsilon)?;
                            report.add_query(name.clone(), epsilon);
                        }
                    }
                }
                
                // Analyze function body
                for s in body {
                    self.analyze_statement(s, report)?;
                }
            }
            _ => {}
        }
        Ok(())
    }
    
    /// Calculate sensitivity of a query
    pub fn calculate_sensitivity(&self, query: &str) -> f64 {
        // Sensitivity = max |f(D) - f(D')| for neighboring D, D'
        // In practice, often estimated or bounded
        *self.sensitivities.get(query).unwrap_or(&1.0)
    }
}

/// Privacy analysis report
#[derive(Debug)]
pub struct PrivacyReport {
    pub queries: Vec<QueryRecord>,
    pub final_epsilon: f64,
    pub remaining_budget: f64,
    pub compliance_status: ComplianceStatus,
}

#[derive(Debug)]
pub struct QueryRecord {
    pub function_name: String,
    pub epsilon_spent: f64,
    pub mechanism: String,
}

#[derive(Debug)]
pub enum ComplianceStatus {
    Compliant,
    BudgetExceeded,
    MechanismViolation,
}

impl PrivacyReport {
    pub fn new() -> Self {
        PrivacyReport {
            queries: Vec::new(),
            final_epsilon: 0.0,
            remaining_budget: 0.0,
            compliance_status: ComplianceStatus::Compliant,
        }
    }
    
    pub fn add_query(&mut self, name: String, epsilon: f64) {
        self.queries.push(QueryRecord {
            function_name: name,
            epsilon_spent: epsilon,
            mechanism: "Laplace".to_string(),
        });
    }
    
    pub fn generate_report(&self) -> String {
        let mut report = String::new();
        report.push_str("DIFFERENTIAL PRIVACY REPORT\n");
        report.push_str("============================\n\n");
        
        report.push_str(&format!("Total Budget: ε = {:.3}\n", self.queries.iter().map(|q| q.epsilon_spent).sum::<f64>() + self.remaining_budget));
        report.push_str(&format!("Spent: ε = {:.3}\n", self.final_epsilon));
        report.push_str(&format!("Remaining: ε = {:.3}\n\n", self.remaining_budget));
        
        report.push_str("Queries:\n");
        for query in &self.queries {
            report.push_str(&format!(
                "  - {}: ε = {:.3} ({} mechanism)\n",
                query.function_name, query.epsilon_spent, query.mechanism
            ));
        }
        
        match self.compliance_status {
            ComplianceStatus::Compliant => {
                report.push_str("\n✅ Compliant with differential privacy guarantees\n");
            }
            ComplianceStatus::BudgetExceeded => {
                report.push_str("\n❌ Privacy budget exceeded\n");
            }
            _ => {
                report.push_str("\n⚠️ Compliance issue detected\n");
            }
        }
        
        report
    }
}

fn parse_epsilon(attr: &str) -> Result<f64, String> {
    // Parse @differential_privacy(epsilon=0.1)
    if let Some(start) = attr.find("epsilon=") {
        let rest = &attr[start + 8..];
        if let Some(end) = rest.find(|c| c == ')' || c == ',') {
            rest[..end].parse().map_err(|e| format!("Invalid epsilon: {}", e))
        } else {
            Err("Malformed epsilon attribute".to_string())
        }
    } else {
        Ok(1.0)  // Default epsilon
    }
}

fn random_uniform() -> f64 {
    // In real implementation, use cryptographically secure RNG
    // This is a placeholder
    0.5
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_privacy_budget() {
        let mut config = PrivacyConfig::new(1.0);
        assert!(config.can_query(0.5));
        config.spend_budget(0.5).unwrap();
        assert!(config.can_query(0.3));
        assert!(!config.can_query(0.6));
    }
    
    #[test]
    fn test_laplace_noise() {
        let config = PrivacyConfig::new(1.0);
        let noise = config.laplace_noise(1.0);
        assert!(noise.is_finite());
    }
}
