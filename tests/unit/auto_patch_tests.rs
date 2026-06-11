// Unit Tests: Auto-Patch API (Fatal Vector 1 Hardening)

use zeus_compiler::auto_patch::{AutoPatcher, DegradationLevel, Diagnostic, DiagnosticKind};
use zeus_compiler::ast::{Program, Statement, Expression};

fn create_simple_program() -> Program {
    Program { statements: vec![] }
}

#[test]
fn test_strict_mode_fails_on_unbounded_loop() {
    let mut patcher = AutoPatcher::new(DegradationLevel::Strict);
    
    let diagnostics = vec![
        Diagnostic {
            kind: DiagnosticKind::UnboundedLoop { 
                line: 10, 
                function: "main".to_string() 
            },
            line: 10,
            function: "main".to_string(),
            message: "Unbounded while loop detected".to_string(),
        }
    ];
    
    let mut program = create_simple_program();
    let result = patcher.auto_patch(&mut program, &diagnostics);
    
    assert!(!result.success, "Strict mode should fail on undecidable");
    assert!(result.warnings.iter().any(|w| w.contains("cannot auto-patch")));
}

#[test]
fn test_adaptive_mode_injects_watchdog() {
    let mut patcher = AutoPatcher::new(DegradationLevel::Adaptive);
    
    let diagnostics = vec![
        Diagnostic {
            kind: DiagnosticKind::UnboundedLoop { 
                line: 10, 
                function: "process".to_string() 
            },
            line: 10,
            function: "process".to_string(),
            message: "Unbounded while loop".to_string(),
        }
    ];
    
    let mut program = create_simple_program();
    let result = patcher.auto_patch(&mut program, &diagnostics);
    
    assert!(result.success, "Adaptive mode should succeed");
    assert!(result.patches_applied.iter().any(|p| p.contains("watchdog")));
}

#[test]
fn test_adaptive_mode_converts_malloc_to_arena() {
    let mut patcher = AutoPatcher::new(DegradationLevel::Adaptive);
    
    let diagnostics = vec![
        Diagnostic {
            kind: DiagnosticKind::DynamicPointer { 
                line: 5, 
                function: "alloc".to_string() 
            },
            line: 5,
            function: "alloc".to_string(),
            message: "Dynamic memory allocation".to_string(),
        }
    ];
    
    let mut program = create_simple_program();
    let result = patcher.auto_patch(&mut program, &diagnostics);
    
    assert!(result.success, "Should succeed with arena conversion");
    assert!(result.patches_applied.iter().any(|p| p.contains("arena")));
}

#[test]
fn test_permissive_mode_uses_bmc() {
    let mut patcher = AutoPatcher::new(DegradationLevel::Permissive);
    
    let diagnostics = vec![
        Diagnostic {
            kind: DiagnosticKind::UnboundedLoop { 
                line: 10, 
                function: "search".to_string() 
            },
            line: 10,
            function: "search".to_string(),
            message: "Potentially unbounded".to_string(),
        }
    ];
    
    let mut program = create_simple_program();
    let result = patcher.auto_patch(&mut program, &diagnostics);
    
    assert!(result.success, "Permissive mode should always succeed");
    assert!(result.patches_applied.iter().any(|p| p.contains("bounded model checking")));
}

#[test]
fn test_external_lib_sandboxing() {
    let mut patcher = AutoPatcher::new(DegradationLevel::Strict);
    
    let diagnostics = vec![
        Diagnostic {
            kind: DiagnosticKind::ExternalLibrary { 
                library: "unsafe_lib".to_string() 
            },
            line: 15,
            function: "call_external".to_string(),
            message: "External library call".to_string(),
        }
    ];
    
    let mut program = create_simple_program();
    let result = patcher.auto_patch(&mut program, &diagnostics);
    
    // External libs should be sandboxed regardless of mode
    assert!(result.patches_applied.iter().any(|p| p.contains("sandbox")));
}

#[test]
fn test_multiple_diagnostics_handled() {
    let mut patcher = AutoPatcher::new(DegradationLevel::Adaptive);
    
    let diagnostics = vec![
        Diagnostic {
            kind: DiagnosticKind::UnboundedLoop { 
                line: 10, 
                function: "main".to_string() 
            },
            line: 10,
            function: "main".to_string(),
            message: "Loop 1".to_string(),
        },
        Diagnostic {
            kind: DiagnosticKind::DynamicPointer { 
                line: 20, 
                function: "main".to_string() 
            },
            line: 20,
            function: "main".to_string(),
            message: "Pointer 1".to_string(),
        },
        Diagnostic {
            kind: DiagnosticKind::ExternalLibrary { 
                library: "libX".to_string() 
            },
            line: 30,
            function: "main".to_string(),
            message: "External".to_string(),
        },
    ];
    
    let mut program = create_simple_program();
    let result = patcher.auto_patch(&mut program, &diagnostics);
    
    assert_eq!(result.patches_applied.len(), 3, "Should apply 3 patches");
    assert!(result.success);
}

#[test]
fn test_generate_patch_report() {
    let mut patcher = AutoPatcher::new(DegradationLevel::Adaptive);
    
    let diagnostics = vec![
        Diagnostic {
            kind: DiagnosticKind::UnboundedLoop { 
                line: 10, 
                function: "process".to_string() 
            },
            line: 10,
            function: "process".to_string(),
            message: "Unbounded".to_string(),
        }
    ];
    
    let mut program = create_simple_program();
    patcher.auto_patch(&mut program, &diagnostics);
    
    let report = patcher.generate_patch_report();
    
    assert!(report.contains("ZEUS AUTO-PATCH REPORT"));
    assert!(report.contains("ADAPTIVE"));
    assert!(report.contains("watchdog"));
}

#[test]
fn test_no_patches_needed() {
    let mut patcher = AutoPatcher::new(DegradationLevel::Strict);
    
    // Empty diagnostics
    let diagnostics: Vec<Diagnostic> = vec![];
    
    let mut program = create_simple_program();
    let result = patcher.auto_patch(&mut program, &diagnostics);
    
    assert!(result.success);
    assert!(result.patches_applied.is_empty());
    
    let report = patcher.generate_patch_report();
    assert!(report.contains("No patches applied"));
}

#[test]
fn test_nested_loop_bound_injection() {
    let mut patcher = AutoPatcher::new(DegradationLevel::Permissive);
    
    let diagnostics = vec![
        Diagnostic {
            kind: DiagnosticKind::UnboundedLoop { 
                line: 5, 
                function: "nested".to_string() 
            },
            line: 5,
            function: "nested".to_string(),
            message: "Nested loop".to_string(),
        }
    ];
    
    let mut program = create_simple_program();
    let result = patcher.auto_patch(&mut program, &diagnostics);
    
    // Should inject counter and bound check
    assert!(result.patches_applied.iter().any(|p| p.contains("loop_counter")));
    assert!(result.patches_applied.iter().any(|p| p.contains("100")));
}

#[test]
fn test_watchdog_configurable_timeout() {
    let mut patcher = AutoPatcher::new(DegradationLevel::Adaptive);
    
    let diagnostics = vec![
        Diagnostic {
            kind: DiagnosticKind::UnboundedLoop { 
                line: 10, 
                function: "crypto".to_string() 
            },
            line: 10,
            function: "crypto".to_string(),
            message: "Crypto loop".to_string(),
        }
    ];
    
    let mut program = create_simple_program();
    let result = patcher.auto_patch(&mut program, &diagnostics);
    
    // Should mention watchdog configuration
    let report = patcher.generate_patch_report();
    assert!(report.contains("watchdog") || report.contains("__zeus"));
}

#[test]
fn test_degradation_level_from_string() {
    // Test that we can parse degradation levels from strings
    let levels = vec!["strict", "adaptive", "permissive", "unknown"];
    
    for level_str in &levels {
        let level = match *level_str {
            "strict" => DegradationLevel::Strict,
            "adaptive" => DegradationLevel::Adaptive,
            "permissive" => DegradationLevel::Permissive,
            _ => DegradationLevel::Adaptive, // default
        };
        
        // Just verify no panic
        let patcher = AutoPatcher::new(level);
        let report = patcher.generate_patch_report();
        assert!(!report.is_empty());
    }
}

#[test]
fn test_timeout_diagnostic_handling() {
    let mut patcher = AutoPatcher::new(DegradationLevel::Permissive);
    
    let diagnostics = vec![
        Diagnostic {
            kind: DiagnosticKind::Timeout { ms: 2000 },
            line: 1,
            function: "complex".to_string(),
            message: "Z3 timeout".to_string(),
        }
    ];
    
    let mut program = create_simple_program();
    let result = patcher.auto_patch(&mut program, &diagnostics);
    
    // Timeout should trigger BMC in permissive mode
    assert!(result.success);
    assert!(result.patches_applied.iter().any(|p| p.contains("bounded")));
}
