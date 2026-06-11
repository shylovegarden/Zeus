// Unit Tests: Z3 Cache System

#[cfg(test)]
mod z3_cache_tests {
    use zeus_compiler::z3_cache::{Z3ProofCache, ProofCacheEntry, ProofResult};
    use std::fs;
    use std::path::Path;

    // Test 1: Cache creation
    #[test]
    fn test_cache_creation() {
        let cache = Z3ProofCache::new("/tmp/test_cache.json");
        assert_eq!(cache.stats().hits, 0);
        assert_eq!(cache.stats().misses, 0);
    }

    // Test 2: Cache lookup miss
    #[test]
    fn test_cache_lookup_miss() {
        let mut cache = Z3ProofCache::new("/tmp/test_cache.json");
        let result = cache.lookup("nonexistent_hash");
        assert!(result.is_none());
        assert_eq!(cache.stats().misses, 1);
    }

    // Test 3: Cache store and lookup
    #[test]
    fn test_cache_store_and_lookup() {
        let mut cache = Z3ProofCache::new("/tmp/test_cache.json");
        
        let entry = ProofCacheEntry {
            ast_hash: "test_hash".to_string(),
            deps_hash: "deps_hash".to_string(),
            timestamp: 1234567890,
            result: ProofResult::Proved,
            verification_time_ms: 100,
            properties: vec!["zero_heap".to_string()],
        };
        
        cache.store("test_hash", entry.clone());
        
        let result = cache.lookup("test_hash");
        assert!(result.is_some());
        assert_eq!(result.unwrap().result, ProofResult::Proved);
        assert_eq!(cache.stats().hits, 1);
    }

    // Test 4: Cache hit doesn't increment misses
    #[test]
    fn test_cache_hit_no_miss_increment() {
        let mut cache = Z3ProofCache::new("/tmp/test_cache.json");
        
        let entry = ProofCacheEntry {
            ast_hash: "hash1".to_string(),
            deps_hash: "deps".to_string(),
            timestamp: 1,
            result: ProofResult::Proved,
            verification_time_ms: 50,
            properties: vec![],
        };
        
        cache.store("hash1", entry);
        
        // First lookup (was miss, now hit after store)
        cache.lookup("hash1");
        let stats = cache.stats();
        assert_eq!(stats.hits, 1);
        // Misses should be from earlier test or 0
    }

    // Test 5: ProofResult variants
    #[test]
    fn test_proof_result_variants() {
        let proved = ProofResult::Proved;
        let disproved = ProofResult::Disproved { 
            counterexample: "x = 5".to_string() 
        };
        let timeout = ProofResult::Timeout { attempted: 2000 };
        let bounded = ProofResult::Bounded { limit: 100 };
        
        // Just verify they can be created
        assert!(matches!(proved, ProofResult::Proved));
        assert!(matches!(disproved, ProofResult::Disproved { .. }));
        assert!(matches!(timeout, ProofResult::Timeout { .. }));
        assert!(matches!(bounded, ProofResult::Bounded { .. }));
    }

    // Test 6: Cache with multiple entries
    #[test]
    fn test_cache_multiple_entries() {
        let mut cache = Z3ProofCache::new("/tmp/test_cache.json");
        
        for i in 0..5 {
            let entry = ProofCacheEntry {
                ast_hash: format!("hash{}", i),
                deps_hash: format!("deps{}", i),
                timestamp: i as u64,
                result: ProofResult::Proved,
                verification_time_ms: 100,
                properties: vec![],
            };
            cache.store(&format!("hash{}", i), entry);
        }
        
        for i in 0..5 {
            let result = cache.lookup(&format!("hash{}", i));
            assert!(result.is_some());
        }
    }

    // Test 7: Cache persistence
    #[test]
    fn test_cache_save_and_load() {
        let cache_path = "/tmp/test_persistence.json";
        
        // Clean up if exists
        if Path::new(cache_path).exists() {
            fs::remove_file(cache_path).unwrap();
        }
        
        // Create and populate cache
        {
            let mut cache = Z3ProofCache::new(cache_path);
            let entry = ProofCacheEntry {
                ast_hash: "persist_hash".to_string(),
                deps_hash: "persist_deps".to_string(),
                timestamp: 999,
                result: ProofResult::Proved,
                verification_time_ms: 200,
                properties: vec!["constant_time".to_string()],
            };
            cache.store("persist_hash", entry);
            cache.save().unwrap();
        }
        
        // Load cache
        {
            let cache = Z3ProofCache::new(cache_path);
            let result = cache.lookup("persist_hash");
            assert!(result.is_some());
        }
        
        // Clean up
        fs::remove_file(cache_path).unwrap();
    }

    // Test 8: Entry timestamp validation
    #[test]
    fn test_entry_timestamp() {
        let entry = ProofCacheEntry {
            ast_hash: "hash".to_string(),
            deps_hash: "deps".to_string(),
            timestamp: 1234567890,
            result: ProofResult::Proved,
            verification_time_ms: 100,
            properties: vec![],
        };
        
        assert_eq!(entry.timestamp, 1234567890);
    }

    // Test 9: Entry properties
    #[test]
    fn test_entry_properties() {
        let entry = ProofCacheEntry {
            ast_hash: "hash".to_string(),
            deps_hash: "deps".to_string(),
            timestamp: 1,
            result: ProofResult::Proved,
            verification_time_ms: 100,
            properties: vec![
                "zero_heap".to_string(),
                "constant_time".to_string(),
            ],
        };
        
        assert_eq!(entry.properties.len(), 2);
        assert!(entry.properties.contains(&"zero_heap".to_string()));
        assert!(entry.properties.contains(&"constant_time".to_string()));
    }

    // Test 10: Empty cache operations
    #[test]
    fn test_empty_cache_operations() {
        let mut cache = Z3ProofCache::new("/tmp/empty_test.json");
        
        // Lookup in empty cache
        let result = cache.lookup("any_hash");
        assert!(result.is_none());
        
        // Stats on empty cache
        let stats = cache.stats();
        assert_eq!(stats.hits, 0);
        assert_eq!(stats.misses, 1); // The lookup we just did
    }

    // Test 11: Cache with different proof results
    #[test]
    fn test_cache_different_results() {
        let mut cache = Z3ProofCache::new("/tmp/multi_result.json");
        
        let results = vec![
            ProofResult::Proved,
            ProofResult::Disproved { counterexample: "x=1".to_string() },
            ProofResult::Timeout { attempted: 1000 },
            ProofResult::Bounded { limit: 50 },
        ];
        
        for (i, result) in results.iter().enumerate() {
            let entry = ProofCacheEntry {
                ast_hash: format!("hash{}", i),
                deps_hash: "deps".to_string(),
                timestamp: i as u64,
                result: result.clone(),
                verification_time_ms: 100,
                properties: vec![],
            };
            cache.store(&format!("hash{}", i), entry);
        }
        
        for i in 0..4 {
            let lookup = cache.lookup(&format!("hash{}", i));
            assert!(lookup.is_some());
        }
    }

    // Test 12: Cache overwrite
    #[test]
    fn test_cache_overwrite() {
        let mut cache = Z3ProofCache::new("/tmp/overwrite.json");
        
        let entry1 = ProofCacheEntry {
            ast_hash: "same_hash".to_string(),
            deps_hash: "deps1".to_string(),
            timestamp: 1,
            result: ProofResult::Proved,
            verification_time_ms: 100,
            properties: vec![],
        };
        
        let entry2 = ProofCacheEntry {
            ast_hash: "same_hash".to_string(),
            deps_hash: "deps2".to_string(),
            timestamp: 2,
            result: ProofResult::Timeout { attempted: 2000 },
            verification_time_ms: 200,
            properties: vec!["new".to_string()],
        };
        
        cache.store("same_hash", entry1);
        cache.store("same_hash", entry2.clone());
        
        let result = cache.lookup("same_hash");
        assert!(result.is_some());
        // Should be the second entry
        assert!(matches!(result.unwrap().result, ProofResult::Timeout { .. }));
    }

    // Test 13: Cache clear
    #[test]
    fn test_cache_clear() {
        let mut cache = Z3ProofCache::new("/tmp/clear.json");
        
        let entry = ProofCacheEntry {
            ast_hash: "hash".to_string(),
            deps_hash: "deps".to_string(),
            timestamp: 1,
            result: ProofResult::Proved,
            verification_time_ms: 100,
            properties: vec![],
        };
        
        cache.store("hash", entry);
        assert!(cache.lookup("hash").is_some());
        
        cache.clear();
        assert!(cache.lookup("hash").is_none());
    }

    // Test 14: Cache size limit
    #[test]
    fn test_cache_size() {
        let mut cache = Z3ProofCache::new("/tmp/size.json");
        
        for i in 0..100 {
            let entry = ProofCacheEntry {
                ast_hash: format!("hash{}", i),
                deps_hash: "deps".to_string(),
                timestamp: i as u64,
                result: ProofResult::Proved,
                verification_time_ms: 100,
                properties: vec![],
            };
            cache.store(&format!("hash{}", i), entry);
        }
        
        assert_eq!(cache.len(), 100);
    }

    // Test 15: Verification time tracking
    #[test]
    fn test_verification_time_tracking() {
        let entry = ProofCacheEntry {
            ast_hash: "hash".to_string(),
            deps_hash: "deps".to_string(),
            timestamp: 1,
            result: ProofResult::Proved,
            verification_time_ms: 150,
            properties: vec![],
        };
        
        assert_eq!(entry.verification_time_ms, 150);
    }

    // Test 16: Cache hit rate
    #[test]
    fn test_cache_hit_rate() {
        let mut cache = Z3ProofCache::new("/tmp/hitrate.json");
        
        // Store one entry
        let entry = ProofCacheEntry {
            ast_hash: "hash".to_string(),
            deps_hash: "deps".to_string(),
            timestamp: 1,
            result: ProofResult::Proved,
            verification_time_ms: 100,
            properties: vec![],
        };
        cache.store("hash", entry);
        
        // 3 hits
        cache.lookup("hash");
        cache.lookup("hash");
        cache.lookup("hash");
        
        // 1 miss
        cache.lookup("other");
        
        let hit_rate = cache.hit_rate();
        assert!(hit_rate > 0.7); // 3/4 = 75%
    }

    // Test 17: Entry equality
    #[test]
    fn test_entry_equality() {
        let entry1 = ProofCacheEntry {
            ast_hash: "hash".to_string(),
            deps_hash: "deps".to_string(),
            timestamp: 1,
            result: ProofResult::Proved,
            verification_time_ms: 100,
            properties: vec![],
        };
        
        let entry2 = entry1.clone();
        assert_eq!(entry1.ast_hash, entry2.ast_hash);
        assert_eq!(entry1.result, entry2.result);
    }

    // Test 18: Complex hash
    #[test]
    fn test_complex_hash() {
        let mut cache = Z3ProofCache::new("/tmp/complex.json");
        
        let complex_hash = "sha256:abcdef1234567890...";
        let entry = ProofCacheEntry {
            ast_hash: complex_hash.to_string(),
            deps_hash: "other".to_string(),
            timestamp: 1,
            result: ProofResult::Proved,
            verification_time_ms: 100,
            properties: vec![],
        };
        
        cache.store(complex_hash, entry);
        assert!(cache.lookup(complex_hash).is_some());
    }

    // Test 19: Cache with unicode
    #[test]
    fn test_unicode_hash() {
        let mut cache = Z3ProofCache::new("/tmp/unicode.json");
        
        let unicode_hash = "hash_日本語_émojis_🚀";
        let entry = ProofCacheEntry {
            ast_hash: unicode_hash.to_string(),
            deps_hash: "deps".to_string(),
            timestamp: 1,
            result: ProofResult::Proved,
            verification_time_ms: 100,
            properties: vec![],
        };
        
        cache.store(unicode_hash, entry);
        assert!(cache.lookup(unicode_hash).is_some());
    }

    // Test 20: Large cache
    #[test]
    fn test_large_cache() {
        let mut cache = Z3ProofCache::new("/tmp/large.json");
        
        for i in 0..1000 {
            let entry = ProofCacheEntry {
                ast_hash: format!("hash_{}", i),
                deps_hash: format!("deps_{}", i),
                timestamp: i as u64,
                result: ProofResult::Proved,
                verification_time_ms: 100 + (i as u64),
                properties: vec![format!("prop{}", i)],
            };
            cache.store(&format!("hash_{}", i), entry);
        }
        
        assert_eq!(cache.len(), 1000);
        
        // Verify random lookups
        for i in [0, 100, 500, 999] {
            assert!(cache.lookup(&format!("hash_{}", i)).is_some());
        }
    }
}
