// License System: Enterprise-grade license management
// Implements on-prem licensing for Fatal Vector 3 hardening

use ed25519_dalek::{PublicKey, Signature, Verifier};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc, Duration};

/// License tier determines features and limits
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum LicenseTier {
    Free,       // Open source, limited verifications
    Pro,        // Paid, 10K verifications/month
    Enterprise, // Unlimited, SLA, custom policies
}

impl LicenseTier {
    pub fn max_verifications_per_month(&self) -> Option<u64> {
        match self {
            LicenseTier::Free => Some(100),
            LicenseTier::Pro => Some(10_000),
            LicenseTier::Enterprise => None, // Unlimited
        }
    }
    
    pub fn price_per_year(&self) -> u64 {
        match self {
            LicenseTier::Free => 0,
            LicenseTier::Pro => 999,      // $999/year
            LicenseTier::Enterprise => 50_000, // $50K/year
        }
    }
    
    pub fn features(&self) -> Vec<String> {
        match self {
            LicenseTier::Free => vec![
                "basic_verification".to_string(),
                "community_support".to_string(),
            ],
            LicenseTier::Pro => vec![
                "advanced_verification".to_string(),
                "email_support".to_string(),
                "ci_cd_integration".to_string(),
                "team_dashboard".to_string(),
            ],
            LicenseTier::Enterprise => vec![
                "unlimited_verification".to_string(),
                "sla_guarantee".to_string(),
                "sso_integration".to_string(),
                "audit_logging".to_string(),
                "custom_policies".to_string(),
                "dedicated_support".to_string(),
                "air_gapped_deployment".to_string(),
            ],
        }
    }
}

/// License data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct License {
    pub tier: LicenseTier,
    pub organization: String,
    pub issued_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub features: Vec<String>,
    pub max_verifications: Option<u64>,
    pub signature: String, // Ed25519 signature (hex)
}

/// License verification result
#[derive(Debug, Clone)]
pub enum LicenseStatus {
    Valid { license: License },
    Expired { expired_at: DateTime<Utc> },
    InvalidSignature,
    InvalidFormat(String),
    Revoked,
    FeatureNotAllowed { feature: String, tier: LicenseTier },
}

/// License manager handles verification and feature gating
pub struct LicenseManager {
    /// Zeus public key for signature verification
    public_key: PublicKey,
    /// Verified license cache
    cache: HashMap<String, License>,
    /// Current month's verification count
    verification_count: HashMap<String, u64>,
}

impl LicenseManager {
    /// Create license manager with embedded public key
    pub fn new() -> Result<Self, String> {
        // Embedded Zeus public key (production would load from secure storage)
        let public_key_bytes = include_bytes!("../../licenses/zeus_public.key");
        let public_key = PublicKey::from_bytes(public_key_bytes)
            .map_err(|e| format!("Invalid public key: {}", e))?;
        
        Ok(LicenseManager {
            public_key,
            cache: HashMap::new(),
            verification_count: HashMap::new(),
        })
    }
    
    /// Verify a license key
    pub fn verify_license(&mut self, license_key: &str) -> LicenseStatus {
        // 1. Parse license
        let license = match self.parse_license(license_key) {
            Ok(l) => l,
            Err(e) => return LicenseStatus::InvalidFormat(e),
        };
        
        // 2. Check expiration
        if Utc::now() > license.expires_at {
            return LicenseStatus::Expired { 
                expired_at: license.expires_at 
            };
        }
        
        // 3. Verify signature
        if !self.verify_signature(&license) {
            return LicenseStatus::InvalidSignature;
        }
        
        // 4. Check if revoked (would check against revocation list)
        if self.is_revoked(&license) {
            return LicenseStatus::Revoked;
        }
        
        // 5. Cache valid license
        self.cache.insert(license_key.to_string(), license.clone());
        
        LicenseStatus::Valid { license }
    }
    
    /// Parse license from base64-encoded JSON
    fn parse_license(&self, license_key: &str) -> Result<License, String> {
        // Decode base64
        let decoded = base64::decode(license_key)
            .map_err(|e| format!("Invalid base64: {}", e))?;
        
        // Parse JSON
        let license: License = serde_json::from_slice(&decoded)
            .map_err(|e| format!("Invalid JSON: {}", e))?;
        
        Ok(license)
    }
    
    /// Verify Ed25519 signature
    fn verify_signature(&self, license: &License) -> bool {
        // Create message to verify (license without signature)
        let mut license_copy = license.clone();
        license_copy.signature = String::new();
        
        let message = match serde_json::to_vec(&license_copy) {
            Ok(m) => m,
            Err(_) => return false,
        };
        
        // Decode signature
        let sig_bytes = match hex::decode(&license.signature) {
            Ok(s) => s,
            Err(_) => return false,
        };
        
        let signature = match Signature::from_bytes(&sig_bytes) {
            Ok(s) => s,
            Err(_) => return false,
        };
        
        // Verify
        self.public_key.verify(&message, &signature).is_ok()
    }
    
    /// Check if license is revoked
    fn is_revoked(&self, _license: &License) -> bool {
        // Would check against online revocation list for Enterprise
        // For on-prem, this is typically done during support calls
        false
    }
    
    /// Check if feature is allowed
    pub fn check_feature(&self, license_key: &str, feature: &str) -> bool {
        let license = match self.cache.get(license_key) {
            Some(l) => l,
            None => {
                // Try to verify
                match self.verify_license(license_key) {
                    LicenseStatus::Valid { license } => {
                        license
                    }
                    _ => return false,
                }
            }
        };
        
        license.features.contains(&feature.to_string())
    }
    
    /// Check verification limit
    pub fn can_verify(&mut self, license_key: &str) -> bool {
        let license = match self.cache.get(license_key) {
            Some(l) => l.clone(),
            None => return false,
        };
        
        // Check if there's a limit
        let limit = match license.max_verifications {
            Some(l) => l,
            None => return true, // Unlimited
        };
        
        // Check current count
        let count = self.verification_count.entry(license_key.to_string()).or_insert(0);
        if *count >= limit {
            return false;
        }
        
        *count += 1;
        true
    }
    
    /// Get remaining verifications
    pub fn remaining_verifications(&self, license_key: &str) -> Option<u64> {
        let license = self.cache.get(license_key)?;
        let limit = license.max_verifications?;
        let count = self.verification_count.get(license_key).copied().unwrap_or(0);
        
        Some(limit.saturating_sub(count))
    }
    
    /// Reset monthly verification count
    pub fn reset_monthly_count(&mut self) {
        self.verification_count.clear();
    }
}

/// License generator (Zeus internal use only)
#[cfg(feature = "license-generator")]
pub mod license_generator {
    use super::*;
    use ed25519_dalek::{Keypair, Signer};
    
    pub struct LicenseGenerator {
        keypair: Keypair,
    }
    
    impl LicenseGenerator {
        pub fn new(secret_key_path: &str) -> Result<Self, String> {
            let secret_key_bytes = fs::read(secret_key_path)
                .map_err(|e| format!("Cannot read secret key: {}", e))?;
            
            let keypair = Keypair::from_bytes(&secret_key_bytes)
                .map_err(|e| format!("Invalid keypair: {}", e))?;
            
            Ok(LicenseGenerator { keypair })
        }
        
        pub fn generate_license(
            &self,
            tier: LicenseTier,
            organization: &str,
            validity_days: i64,
        ) -> Result<String, String> {
            let now = Utc::now();
            let expires = now + Duration::days(validity_days);
            
            let license = License {
                tier: tier.clone(),
                organization: organization.to_string(),
                issued_at: now,
                expires_at: expires,
                features: tier.features(),
                max_verifications: tier.max_verifications_per_month(),
                signature: String::new(), // Will be filled after signing
            };
            
            // Sign license
            let mut license_json = serde_json::to_vec(&license)
                .map_err(|e| format!("JSON error: {}", e))?;
            
            let signature = self.keypair.sign(&license_json);
            
            // Create final license with signature
            let license_with_sig = License {
                signature: hex::encode(signature.to_bytes()),
                ..license
            };
            
            // Encode to base64
            let json = serde_json::to_vec(&license_with_sig)
                .map_err(|e| format!("JSON error: {}", e))?;
            
            Ok(base64::encode(&json))
        }
    }
}

/// CLI command integration
pub fn cmd_license_check(license_key: &str) {
    let mut manager = LicenseManager::new().expect("Initialize license manager");
    
    match manager.verify_license(license_key) {
        LicenseStatus::Valid { license } => {
            println!("✅ License Valid");
            println!("  Tier: {:?}", license.tier);
            println!("  Organization: {}", license.organization);
            println!("  Expires: {}", license.expires_at);
            println!("  Features: {:?}", license.features);
            if let Some(max) = license.max_verifications {
                println!("  Monthly limit: {}", max);
            } else {
                println!("  Monthly limit: Unlimited");
            }
        }
        LicenseStatus::Expired { expired_at } => {
            println!("❌ License Expired on {}", expired_at);
        }
        LicenseStatus::InvalidSignature => {
            println!("❌ Invalid License Signature");
        }
        LicenseStatus::InvalidFormat(e) => {
            println!("❌ Invalid License Format: {}", e);
        }
        LicenseStatus::Revoked => {
            println!("❌ License Revoked");
        }
        LicenseStatus::FeatureNotAllowed { feature, tier } => {
            println!("❌ Feature '{}' not available in {:?} tier", feature, tier);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_license_tier_limits() {
        assert_eq!(LicenseTier::Free.max_verifications_per_month(), Some(100));
        assert_eq!(LicenseTier::Pro.max_verifications_per_month(), Some(10_000));
        assert_eq!(LicenseTier::Enterprise.max_verifications_per_month(), None);
    }
    
    #[test]
    fn test_tier_pricing() {
        assert_eq!(LicenseTier::Free.price_per_year(), 0);
        assert_eq!(LicenseTier::Pro.price_per_year(), 999);
        assert_eq!(LicenseTier::Enterprise.price_per_year(), 50_000);
    }
    
    #[test]
    fn test_enterprise_features() {
        let features = LicenseTier::Enterprise.features();
        assert!(features.contains(&"sla_guarantee".to_string()));
        assert!(features.contains(&"air_gapped_deployment".to_string()));
    }
}
