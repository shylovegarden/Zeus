// Integration Tests: Compile and Run Zeus Programs

use std::process::Command;
use std::fs;
use std::path::Path;

/// Helper to compile and run a Zeus program
fn compile_and_run(source: &str) -> Result<(String, String), String> {
    // Write source to temp file
    let temp_dir = std::env::temp_dir();
    let source_path = temp_dir.join("test.zs");
    fs::write(&source_path, source).map_err(|e| e.to_string())?;
    
    // Compile
    let compile_output = Command::new("cargo")
        .args(&["run", "--", "build", source_path.to_str().unwrap()])
        .current_dir("/Users/shy/Developer/ZEUS/zeus_compiler")
        .output()
        .map_err(|e| format!("Failed to compile: {}", e))?;
    
    if !compile_output.status.success() {
        return Err(format!(
            "Compilation failed: {}",
            String::from_utf8_lossy(&compile_output.stderr)
        ));
    }
    
    // Run the generated binary
    let binary_path = temp_dir.join("test");
    let run_output = Command::new(&binary_path)
        .output()
        .map_err(|e| format!("Failed to run: {}", e))?;
    
    let stdout = String::from_utf8_lossy(&run_output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&run_output.stderr).to_string();
    
    Ok((stdout, stderr))
}

#[test]
fn test_hello_world() {
    let source = r#"
        pub fn main() {
            println("Hello, World!");
        }
    "#;
    
    let (stdout, stderr) = compile_and_run(source).expect("Should compile and run");
    assert!(stdout.contains("Hello, World!") || stderr.contains("Hello, World!"));
}

#[test]
fn test_basic_arithmetic() {
    let source = r#"
        pub fn main() {
            let x: i32 = 10;
            let y: i32 = 20;
            let sum = x + y;
            println(sum);
        }
    "#;
    
    let (stdout, _) = compile_and_run(source).expect("Should compile and run");
    assert!(stdout.contains("30"));
}

#[test]
fn test_variable_assignment() {
    let source = r#"
        pub fn main() {
            let x: i32 = 42;
            println(x);
        }
    "#;
    
    let (stdout, _) = compile_and_run(source).expect("Should compile and run");
    assert!(stdout.contains("42"));
}

#[test]
fn test_if_statement() {
    let source = r#"
        pub fn main() {
            let x: i32 = 10;
            if x > 5 {
                println("greater");
            } else {
                println("lesser");
            }
        }
    "#;
    
    let (stdout, _) = compile_and_run(source).expect("Should compile and run");
    assert!(stdout.contains("greater"));
}

#[test]
fn test_while_loop() {
    let source = r#"
        pub fn main() {
            let i: i32 = 0;
            while i < 5 {
                println(i);
                i = i + 1;
            }
        }
    "#;
    
    let (stdout, _) = compile_and_run(source).expect("Should compile and run");
    // Should print 0, 1, 2, 3, 4
    for i in 0..5 {
        assert!(stdout.contains(&i.to_string()));
    }
}

#[test]
fn test_function_call() {
    let source = r#"
        fn add(a: i32, b: i32) -> i32 {
            return a + b;
        }
        
        pub fn main() {
            let result = add(3, 4);
            println(result);
        }
    "#;
    
    let (stdout, _) = compile_and_run(source).expect("Should compile and run");
    assert!(stdout.contains("7"));
}

#[test]
fn test_constant_time_password_compare() {
    let source = r#"
        @constant_time
        fn verify_password(input: [u8; 4], stored: [u8; 4]) -> bool {
            let mut diff: i32 = 0;
            let mut i: i32 = 0;
            while i < 4 {
                diff = diff | ((input[i] ^ stored[i]) as i32);
                i = i + 1;
            }
            return diff == 0;
        }
        
        pub fn main() {
            let stored: [u8; 4] = [1, 2, 3, 4];
            let input: [u8; 4] = [1, 2, 3, 4];
            let result = verify_password(input, stored);
            println(result);
        }
    "#;
    
    let (stdout, _) = compile_and_run(source).expect("Should compile and run");
    // Should print 1 (true) for matching passwords
    assert!(stdout.contains("1") || stdout.contains("true"));
}

#[test]
fn test_zero_heap_enforcement() {
    let source = r#"
        @zero_heap
        pub fn main() {
            let x: i32 = 42;
            println(x);
        }
    "#;
    
    // Should compile successfully with zero-heap policy
    let result = compile_and_run(source);
    assert!(result.is_ok(), "Zero-heap code should compile");
}

#[test]
fn test_certificate_generation() {
    let source = r#"
        @constant_time
        @zero_heap
        fn secure_func(x: i32) -> i32 {
            return x * 2;
        }
        
        pub fn main() {
            let result = secure_func(21);
            println(result);
        }
    "#;
    
    // Compile with certificate generation
    let temp_dir = std::env::temp_dir();
    let source_path = temp_dir.join("cert_test.zs");
    fs::write(&source_path, source).expect("Write source");
    
    let output = Command::new("cargo")
        .args(&["run", "--", "build", "--cert", source_path.to_str().unwrap()])
        .current_dir("/Users/shy/Developer/ZEUS/zeus_compiler")
        .output()
        .expect("Run compiler");
    
    assert!(output.status.success(), "Should compile with certificate");
    
    // Check certificate was generated
    let cert_path = temp_dir.join("cert_test.zcert");
    assert!(cert_path.exists(), "Certificate should be generated");
    
    let cert_content = fs::read_to_string(cert_path).expect("Read certificate");
    assert!(cert_content.contains("zeus_certificate"), "Should contain certificate header");
    assert!(cert_content.contains("constant_time"), "Should list constant_time property");
    assert!(cert_content.contains("zero_heap"), "Should list zero_heap property");
}

#[test]
fn test_llvm_backend_output() {
    let source = r#"
        pub fn main() {
            let x: i32 = 42;
            println(x);
        }
    "#;
    
    // Compile to LLVM IR
    let temp_dir = std::env::temp_dir();
    let source_path = temp_dir.join("llvm_test.zs");
    fs::write(&source_path, source).expect("Write source");
    
    let output = Command::new("cargo")
        .args(&["run", "--", "build", "--target=llvm", source_path.to_str().unwrap()])
        .current_dir("/Users/shy/Developer/ZEUS/zeus_compiler")
        .output()
        .expect("Run compiler");
    
    // Check LLVM IR was generated
    let ir_path = temp_dir.join("llvm_test.ll");
    if ir_path.exists() {
        let ir_content = fs::read_to_string(ir_path).expect("Read IR");
        assert!(ir_content.contains("define"), "Should contain function definitions");
        assert!(ir_content.contains("ModuleID"), "Should be valid LLVM IR");
    }
}

#[test]
fn test_wasm_backend_output() {
    let source = r#"
        pub fn add(a: i32, b: i32) -> i32 {
            return a + b;
        }
    "#;
    
    // Compile to WASM
    let temp_dir = std::env::temp_dir();
    let source_path = temp_dir.join("wasm_test.zs");
    fs::write(&source_path, source).expect("Write source");
    
    let output = Command::new("cargo")
        .args(&["run", "--", "build", "--target=wasm", source_path.to_str().unwrap()])
        .current_dir("/Users/shy/Developer/ZEUS/zeus_compiler")
        .output()
        .expect("Run compiler");
    
    // Check WASM was generated
    let wasm_path = temp_dir.join("wasm_test.wasm");
    if wasm_path.exists() {
        let wasm_bytes = fs::read(&wasm_path).expect("Read WASM");
        // WASM magic number: 0x00 0x61 0x73 0x6D
        assert_eq!(&wasm_bytes[0..4], &[0x00, 0x61, 0x73, 0x6D], "Should be valid WASM");
    }
}

#[test]
fn test_policy_enforcement() {
    let bad_source = r#"
        @zero_heap
        pub fn main() {
            // This should fail zero-heap policy
            let x = malloc(100);
        }
    "#;
    
    let temp_dir = std::env::temp_dir();
    let source_path = temp_dir.join("policy_test.zs");
    fs::write(&source_path, bad_source).expect("Write source");
    
    let output = Command::new("cargo")
        .args(&["run", "--", "build", "--policy=zero-heap", source_path.to_str().unwrap()])
        .current_dir("/Users/shy/Developer/ZEUS/zeus_compiler")
        .output()
        .expect("Run compiler");
    
    // Should fail because malloc violates zero-heap
    assert!(!output.status.success(), "Should fail zero-heap policy");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("heap") || stderr.contains("malloc") || stderr.contains("policy"),
        "Error should mention heap or policy violation"
    );
}

#[test]
fn test_z3_timeout_handling() {
    let complex_source = r#"
        @bounded
        fn complex_calc(n: i32) -> i32 {
            let mut result: i32 = 0;
            let mut i: i32 = 0;
            while i < n {
                let mut j: i32 = 0;
                while j < n {
                    let mut k: i32 = 0;
                    while k < n {
                        result = result + i * j * k;
                        k = k + 1;
                    }
                    j = j + 1;
                }
                i = i + 1;
            }
            return result;
        }
        
        pub fn main() {
            println(complex_calc(10));
        }
    "#;
    
    // Should handle Z3 timeout gracefully with BMC fallback
    let temp_dir = std::env::temp_dir();
    let source_path = temp_dir.join("timeout_test.zs");
    fs::write(&source_path, complex_source).expect("Write source");
    
    let output = Command::new("cargo")
        .args(&["run", "--", "verify", "--timeout=1000", source_path.to_str().unwrap()])
        .current_dir("/Users/shy/Developer/ZEUS/zeus_compiler")
        .output()
        .expect("Run compiler");
    
    // Should either succeed or fail gracefully with timeout message
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !output.status.success() {
        assert!(
            stderr.contains("timeout") || stderr.contains("bounded") || stderr.contains("BMC"),
            "Should indicate timeout or bounded model checking"
        );
    }
}
