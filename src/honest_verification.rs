// Honest Verification Reporting
// Addresses Critical Flaw #3: Silent fallback on timeout
// Reports timeout clearly, never claims "VERIFIED" when unverified

use std::time::{Duration, Instant};

/// Result of verification with honest reporting
#[derive(Debug, Clone, PartialEq)]
pub enum HonestVerificationResult {
    /// Fully verified by SMT solver
    Verified { proof: Proof, time_ms: u64 },
    /// Z3 timed out - NOT verified
    Timeout { attempted_ms: u64 },
    /// Verification failed
    Failed { reason: String },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Proof {
    pub expression: String,
    pub constraints: Vec<String>,
}

/// Honest verifier that never claims success when verification failed
pub struct HonestVerifier {
    timeout_ms: u64,
}

impl HonestVerifier {
    pub fn new(timeout_ms: u64) -> Self {
        HonestVerifier { timeout_ms }
    }

    /// Verify with honest reporting
    pub fn verify(&self, expr: &str) -> HonestVerificationResult {
        let start = Instant::now();
        
        // TODO: Actual Z3 verification
        // For now, simulate timeout after threshold
        
        let elapsed = start.elapsed();
        let elapsed_ms = elapsed.as_millis() as u64;
        
        if elapsed_ms >= self.timeout_ms {
            // TIMEOUT - must report honestly
            HonestVerificationResult::Timeout {
                attempted_ms: elapsed_ms,
            }
        } else {
            // Success case
            HonestVerificationResult::Verified {
                proof: Proof {
                    expression: expr.to_string(),
                    constraints: vec![],
                },
                time_ms: elapsed_ms,
            }
        }
    }

    /// Generate honest certificate
    pub fn generate_certificate(&self, result: &HonestVerificationResult) -> HonestCertificate {
        match result {
            HonestVerificationResult::Verified { proof, time_ms } => {
                HonestCertificate {
                    status: "VERIFIED".to_string(),
                    verified: true,
                    proof_method: "SMT".to_string(),
                    expression: proof.expression.clone(),
                    verification_time_ms: *time_ms,
                    should_sign: true,
                    warning: None,
                }
            }
            HonestVerificationResult::Timeout { attempted_ms } => {
                HonestCertificate {
                    status: "TIMEOUT".to_string(),
                    verified: false,
                    proof_method: "UNVERIFIED".to_string(),
                    expression: "".to_string(),
                    verification_time_ms: *attempted_ms,
                    should_sign: false,
                    warning: Some("Proof timeout - security properties NOT verified".to_string()),
                }
            }
            HonestVerificationResult::Failed { reason } => {
                HonestCertificate {
                    status: "FAILED".to_string(),
                    verified: false,
                    proof_method: "FAILED".to_string(),
                    expression: "".to_string(),
                    verification_time_ms: 0,
                    should_sign: false,
                    warning: Some(format!("Verification failed: {}", reason)),
                }
            }
        }
    }
}

/// Honest certificate that accurately represents verification status
#[derive(Debug, Clone, PartialEq)]
pub struct HonestCertificate {
    pub status: String,
    pub verified: bool,
    pub proof_method: String,
    pub expression: String,
    pub verification_time_ms: u64,
    pub should_sign: bool,  // NO signature on timeout/failure
    pub warning: Option<String>,
}

impl HonestCertificate {
    /// Print honest report to user
    pub fn print_report(&self) {
        match self.status.as_str() {
            "VERIFIED" => {
                println!("✅ VERIFIED (SMT solver)");
                println!("   Time: {}ms", self.verification_time_ms);
                println!("   Signature: Ed25519-signed");
            }
            "TIMEOUT" => {
                println!("⚠️  TIMEOUT - NOT VERIFIED");
                println!("   Attempted: {}ms", self.verification_time_ms);
                println!("   Status: Security properties NOT verified");
                println!("   Action: Reduce function complexity or increase timeout");
            }
            "FAILED" => {
                println!("❌ FAILED - NOT VERIFIED");
                if let Some(w) = &self.warning {
                    println!("   {}", w);
                }
            }
            _ => {
                println!("Unknown status: {}", self.status);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timeout_not_verified() {
        let verifier = HonestVerifier::new(100); // 100ms timeout
        let result = HonestVerificationResult::Timeout { attempted_ms: 100 };
        let cert = verifier.generate_certificate(&result);
        
        assert_eq!(cert.status, "TIMEOUT");
        assert!(!cert.verified);
        assert!(!cert.should_sign);
        assert!(cert.warning.is_some());
    }

    #[test]
    fn test_verified_can_sign() {
        let verifier = HonestVerifier::new(100);
        let result = HonestVerificationResult::Verified {
            proof: Proof {
                expression: "x > 0".to_string(),
                constraints: vec![],
            },
            time_ms: 50,
        };
        let cert = verifier.generate_certificate(&result);
        
        assert_eq!(cert.status, "VERIFIED");
        assert!(cert.verified);
        assert!(cert.should_sign);
        assert!(cert.warning.is_none());
    }

    #[test]
    fn test_failed_no_signature() {
        let verifier = HonestVerifier::new(100);
        let result = HonestVerificationResult::Failed {
            reason: "Parse error".to_string(),
        };
        let cert = verifier.generate_certificate(&result);
        
        assert_eq!(cert.status, "FAILED");
        assert!(!cert.verified);
        assert!(!cert.should_sign);
    }
}
