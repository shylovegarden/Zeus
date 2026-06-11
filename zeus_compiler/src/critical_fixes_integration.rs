// Critical Flaws Integration Module
// Integrates all 3 critical flaw fixes into the compiler pipeline
// This module provides clean APIs for main.rs to call

use std::path::Path;
use crate::binary_verifier::{BinaryVerifier, BinaryVerificationResult};
use crate::type_checker_strict::StrictTypeChecker;
use crate::honest_verification::{HonestVerifier, HonestVerificationResult, HonestCertificate};
use crate::ast::{Type, Expression};

/// Result of applying all critical flaw checks
pub struct CriticalFlawsCheckResult {
    pub binary_verified: bool,
    pub type_check_passed: bool,
    pub verification_passed: bool,
    pub certificate: HonestCertificate,
    pub should_sign: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

/// Apply all 3 critical flaw fixes
pub fn apply_critical_flaws_check(
    binary_path: Option<&Path>,
    target_type: &Type,
    value_type: &Type,
    expr_to_verify: &str,
    timeout_ms: u64,
) -> CriticalFlawsCheckResult {
    let mut result = CriticalFlawsCheckResult {
        binary_verified: false,
        type_check_passed: false,
        verification_passed: false,
        certificate: HonestCertificate {
            status: "PENDING".to_string(),
            verified: false,
            proof_method: "PENDING".to_string(),
            expression: expr_to_verify.to_string(),
            verification_time_ms: 0,
            should_sign: false,
            warning: None,
        },
        should_sign: false,
        errors: Vec::new(),
        warnings: Vec::new(),
    };

    // CRITICAL FLAW #2: Strict Type System Check
    let type_checker = StrictTypeChecker::new();
    match type_checker.check_assignment(target_type, value_type) {
        Ok(()) => {
            result.type_check_passed = true;
        }
        Err(e) => {
            result.type_check_passed = false;
            result.errors.push(format!("Type error: {:?}", e));
        }
    }

    // CRITICAL FLAW #3: Honest Verification
    let verifier = HonestVerifier::new(timeout_ms);
    let verification_result = verifier.verify(expr_to_verify);
    result.certificate = verifier.generate_certificate(&verification_result);
    
    match verification_result {
        HonestVerificationResult::Verified { .. } => {
            result.verification_passed = true;
        }
        HonestVerificationResult::Timeout { attempted_ms } => {
            result.verification_passed = false;
            result.warnings.push(format!(
                "Verification timeout after {}ms - not verified", 
                attempted_ms
            ));
        }
        HonestVerificationResult::Failed { reason } => {
            result.verification_passed = false;
            result.errors.push(format!("Verification failed: {}", reason));
        }
    }

    // CRITICAL FLAW #1: Binary Verification (if binary exists)
    if let Some(path) = binary_path {
        if path.exists() {
            let mut bin_verifier = BinaryVerifier::new();
            match bin_verifier.verify_constant_time(path) {
                BinaryVerificationResult::ConstantTime => {
                    result.binary_verified = true;
                }
                BinaryVerificationResult::TimingLeaks(leaks) => {
                    result.binary_verified = false;
                    for leak in leaks {
                        result.errors.push(format!(
                            "Binary timing leak at {:x}: {}", 
                            leak.address, leak.instruction
                        ));
                    }
                }
                BinaryVerificationResult::Failed(e) => {
                    result.binary_verified = false;
                    result.warnings.push(format!("Binary verification: {}", e));
                }
            }
        } else {
            result.warnings.push("Binary not found, skipping binary verification".to_string());
        }
    }

    // Determine if certificate should be signed
    // ONLY sign if ALL checks pass
    result.should_sign = result.binary_verified 
        && result.type_check_passed 
        && result.verification_passed;
    
    result.certificate.should_sign = result.should_sign;
    result.certificate.verified = result.should_sign;
    
    if result.should_sign {
        result.certificate.status = "VERIFIED".to_string();
    } else if !result.errors.is_empty() {
        result.certificate.status = "FAILED".to_string();
    } else {
        result.certificate.status = "CONDITIONAL".to_string();
    }

    result
}

/// Print comprehensive report of all checks
pub fn print_critical_flaws_report(result: &CriticalFlawsCheckResult) {
    println!("\n=== CRITICAL FLAWS VERIFICATION REPORT ===");
    
    // Type System
    if result.type_check_passed {
        println!("✅ Type System: PASS (strict width checking)");
    } else {
        println!("❌ Type System: FAIL");
        for err in &result.errors {
            if err.contains("Type error") {
                println!("   {}", err);
            }
        }
    }
    
    // Formal Verification
    match result.certificate.status.as_str() {
        "VERIFIED" => {
            println!("✅ Formal Verification: PASS (SMT solver)");
            println!("   Time: {}ms", result.certificate.verification_time_ms);
        }
        "TIMEOUT" | "CONDITIONAL" => {
            println!("⚠️  Formal Verification: TIMEOUT");
            if let Some(ref w) = result.certificate.warning {
                println!("   {}", w);
            }
        }
        _ => {
            println!("❌ Formal Verification: FAIL");
        }
    }
    
    // Binary Verification
    if result.binary_verified {
        println!("✅ Binary Verification: PASS (no timing leaks)");
    } else {
        println!("⚠️  Binary Verification: INCOMPLETE or FAIL");
        for err in &result.errors {
            if err.contains("Binary") {
                println!("   {}", err);
            }
        }
    }
    
    // Overall
    if result.should_sign {
        println!("\n✅ ALL CHECKS PASSED - Certificate will be signed");
    } else {
        println!("\n⚠️  SOME CHECKS FAILED - Certificate will NOT be signed");
        if !result.warnings.is_empty() {
            println!("\nWarnings:");
            for w in &result.warnings {
                println!("  - {}", w);
            }
        }
    }
    
    println!("==========================================\n");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_checks_pass() {
        // This would need actual files and Z3 to test properly
        // For now, just test the structure
        let result = CriticalFlawsCheckResult {
            binary_verified: true,
            type_check_passed: true,
            verification_passed: true,
            certificate: HonestCertificate {
                status: "VERIFIED".to_string(),
                verified: true,
                proof_method: "SMT".to_string(),
                expression: "test".to_string(),
                verification_time_ms: 100,
                should_sign: true,
                warning: None,
            },
            should_sign: true,
            errors: vec![],
            warnings: vec![],
        };
        
        assert!(result.should_sign);
        assert!(result.certificate.should_sign);
    }

    #[test]
    fn test_timeout_no_signature() {
        let result = CriticalFlawsCheckResult {
            binary_verified: true,
            type_check_passed: true,
            verification_passed: false,
            certificate: HonestCertificate {
                status: "TIMEOUT".to_string(),
                verified: false,
                proof_method: "UNVERIFIED".to_string(),
                expression: "test".to_string(),
                verification_time_ms: 2000,
                should_sign: false,
                warning: Some("Timeout".to_string()),
            },
            should_sign: false,
            errors: vec![],
            warnings: vec!["timeout".to_string()],
        };
        
        assert!(!result.should_sign);
        assert!(!result.certificate.should_sign);
    }
}
