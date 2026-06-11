// Unit Tests: LLVM Hardening (Jasmin Defense)

#[cfg(test)]
mod llvm_hardening_tests {
    use zeus_compiler::llvm_hardening::{LLVMHardeningPass, SafeLLVMConfig};

    // Test 1: Safe config creation
    #[test]
    fn test_safe_config_creation() {
        let config = SafeLLVMConfig::default();
        assert!(config.optnone_secret_functions);
        assert!(config.insert_speculation_barriers);
        assert!(config.mark_secret_memory_volatile);
    }

    // Test 2: Config with disabled passes
    #[test]
    fn test_config_disabled_passes() {
        let config = SafeLLVMConfig::default();
        let disabled = config.disabled_passes();
        
        assert!(disabled.contains(&"loop-unroll"));
        assert!(disabled.contains(&"loop-vectorize"));
        assert!(disabled.contains(&"simplifycfg"));
        assert!(disabled.contains(&"jump-threading"));
    }

    // Test 3: Config with allowed passes
    #[test]
    fn test_config_allowed_passes() {
        let config = SafeLLVMConfig::default();
        let allowed = config.allowed_passes();
        
        assert!(allowed.contains(&"mem2reg"));
        assert!(allowed.contains(&"instcombine"));
    }

    // Test 4: Hardening pass creation
    #[test]
    fn test_hardening_pass_creation() {
        let config = SafeLLVMConfig::default();
        let pass = LLVMHardeningPass::new(config);
        assert!(pass.is_enabled());
    }

    // Test 5: Optnone attribute check
    #[test]
    fn test_optnone_attribute_required() {
        let config = SafeLLVMConfig::default();
        assert!(config.requires_optnone("secret_encrypt"));
        assert!(config.requires_optnone("decrypt_data"));
        assert!(!config.requires_optnone("public_helper"));
    }

    // Test 6: Secret function detection
    #[test]
    fn test_secret_function_detection() {
        let config = SafeLLVMConfig::default();
        
        // Functions with "secret" prefix
        assert!(config.is_secret_function("secret_key_derive"));
        assert!(config.is_secret_function("secret_sbox"));
        
        // Functions with crypto names
        assert!(config.is_secret_function("aes_encrypt"));
        assert!(config.is_secret_function("sha256_hash"));
        
        // Regular functions
        assert!(!config.is_secret_function("print_hello"));
        assert!(!config.is_secret_function("main"));
    }

    // Test 7: Memory barrier insertion points
    #[test]
    fn test_barrier_insertion_points() {
        let config = SafeLLVMConfig::default();
        let points = config.barrier_insertion_points();
        
        assert!(points.contains(&"after_load"));
        assert!(points.contains(&"before_branch"));
        assert!(points.contains(&"after_store"));
    }

    // Test 8: Volatile marking for secret data
    #[test]
    fn test_volatile_marking() {
        let config = SafeLLVMConfig::default();
        
        // Secret variables should be volatile
        assert!(config.should_mark_volatile("secret_key"));
        assert!(config.should_mark_volatile("private_key"));
        
        // Public variables should not
        assert!(!config.should_mark_volatile("public_data"));
        assert!(!config.should_mark_volatile("buffer"));
    }

    // Test 9: Assembly verification patterns
    #[test]
    fn test_assembly_verification_patterns() {
        let config = SafeLLVMConfig::default();
        let patterns = config.assembly_check_patterns();
        
        assert!(patterns.contains(&"conditional_jump_on_secret"));
        assert!(patterns.contains(&"variable_timing_instruction"));
        assert!(patterns.contains(&"secret_data_in_register"));
    }

    // Test 10: Config serialization
    #[test]
    fn test_config_serialization() {
        let config = SafeLLVMConfig::default();
        let json = serde_json::to_string(&config).expect("Serialize");
        let decoded: SafeLLVMConfig = serde_json::from_str(&json).expect("Deserialize");
        
        assert_eq!(config.optnone_secret_functions, decoded.optnone_secret_functions);
    }

    // Test 11: Hardening pass with disabled config
    #[test]
    fn test_disabled_hardening() {
        let mut config = SafeLLVMConfig::default();
        config.enabled = false;
        
        let pass = LLVMHardeningPass::new(config);
        assert!(!pass.is_enabled());
    }

    // Test 12: Custom function patterns
    #[test]
    fn test_custom_secret_patterns() {
        let mut config = SafeLLVMConfig::default();
        config.add_secret_pattern("custom_crypto");
        config.add_secret_pattern("secure_rng");
        
        assert!(config.is_secret_function("custom_crypto_init"));
        assert!(config.is_secret_function("secure_rng_seed"));
    }

    // Test 13: Speculation barrier types
    #[test]
    fn test_speculation_barrier_types() {
        let config = SafeLLVMConfig::default();
        let barriers = config.speculation_barriers();
        
        assert!(barriers.contains(&"lfence"));
        assert!(barriers.contains(&"sfence"));
        assert!(barriers.contains(&"mfence"));
    }

    // Test 14: Optimization level override
    #[test]
    fn test_optimization_override() {
        let config = SafeLLVMConfig::default();
        
        // Secret functions should use -O0 (none)
        assert_eq!(config.optimization_level("secret_func"), 0);
        
        // Non-secret can use higher levels
        assert!(config.optimization_level("public_func") > 0);
    }

    // Test 15: Inline assembly checks
    #[test]
    fn test_inline_assembly_checks() {
        let config = SafeLLVMConfig::default();
        
        // Should check inline asm for secrets
        assert!(config.check_inline_asm);
        
        // Forbidden patterns
        let forbidden = config.forbidden_asm_patterns();
        assert!(forbidden.contains(&"cpuid"));
        assert!(forbidden.contains(&"rdtsc"));
    }
}
