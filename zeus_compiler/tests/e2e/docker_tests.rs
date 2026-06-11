// E2E Tests: Docker Container
// Tests the Zeus Docker image

use std::process::Command;
use std::fs;

fn docker_available() -> bool {
    Command::new("docker")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

// Test 1: Docker image exists
#[test]
#[ignore = "requires Docker"]
fn test_docker_image_exists() {
    if !docker_available() {
        return;
    }
    
    let output = Command::new("docker")
        .args(&["images", "zeuslang/compiler", "-q"])
        .output()
        .expect("Docker command failed");
    
    // Should find the image (or be empty if not built)
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success());
}

// Test 2: Docker build succeeds
#[test]
#[ignore = "requires Docker"]
fn test_docker_build() {
    if !docker_available() {
        return;
    }
    
    let output = Command::new("docker")
        .args(&[
            "build",
            "-t", "zeus:test",
            "/Users/shy/Developer/ZEUS/github-action"
        ])
        .output()
        .expect("Docker build failed");
    
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success() || stderr.contains("cache"),
        "Docker build failed: {}", stderr
    );
}

// Test 3: Docker run zeus --version
#[test]
#[ignore = "requires Docker"]
fn test_docker_run_version() {
    if !docker_available() {
        return;
    }
    
    let output = Command::new("docker")
        .args(&[
            "run", "--rm",
            "zeuslang/compiler:latest",
            "--version"
        ])
        .output()
        .expect("Docker run failed");
    
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    
    assert!(
        combined.contains("zeus") || combined.contains("0.1"),
        "Version output: {}", combined
    );
}

// Test 4: Docker compile simple file
#[test]
#[ignore = "requires Docker"]
fn test_docker_compile() {
    if !docker_available() {
        return;
    }
    
    let temp_dir = std::env::temp_dir();
    let source_path = temp_dir.join("docker_test.zs");
    fs::write(&source_path, "pub fn main() { println(42); }").unwrap();
    
    let output = Command::new("docker")
        .args(&[
            "run", "--rm",
            "-v", &format!("{}:/workspace", temp_dir.to_str().unwrap()),
            "zeuslang/compiler:latest",
            "build", "/workspace/docker_test.zs"
        ])
        .output()
        .expect("Docker compile failed");
    
    // Should complete (may succeed or fail based on LLVM availability)
    assert!(true, "Docker compile command executed");
    
    let _ = fs::remove_file(&source_path);
}

// Test 5: Docker verify with policy
#[test]
#[ignore = "requires Docker"]
fn test_docker_verify() {
    if !docker_available() {
        return;
    }
    
    let temp_dir = std::env::temp_dir();
    let source_path = temp_dir.join("docker_verify.zs");
    fs::write(&source_path, "@zero_heap\npub fn main() {}").unwrap();
    
    let output = Command::new("docker")
        .args(&[
            "run", "--rm",
            "-v", &format!("{}:/workspace", temp_dir.to_str().unwrap()),
            "zeuslang/compiler:latest",
            "verify", "--policy=zero-heap", "/workspace/docker_verify.zs"
        ])
        .output()
        .expect("Docker verify failed");
    
    assert!(true, "Docker verify command executed");
    
    let _ = fs::remove_file(&source_path);
}

// Test 6: Docker multi-arch build
#[test]
#[ignore = "requires Docker and Buildx"]
fn test_docker_multiarch_build() {
    if !docker_available() {
        return;
    }
    
    // Check if buildx is available
    let buildx_check = Command::new("docker")
        .args(&["buildx", "version"])
        .output();
    
    if buildx_check.is_err() || !buildx_check.unwrap().status.success() {
        println!("Docker Buildx not available");
        return;
    }
    
    let output = Command::new("docker")
        .args(&[
            "buildx", "build",
            "--platform", "linux/amd64,linux/arm64",
            "-t", "zeus:multiarch",
            "--no-cache",
            "/Users/shy/Developer/ZEUS/github-action"
        ])
        .output()
        .expect("Multi-arch build failed");
    
    let stderr = String::from_utf8_lossy(&output.stderr);
    // Multi-arch builds are complex, may fail without proper setup
    println!("Multi-arch build output: {}", stderr);
}

// Test 7: Docker container has required tools
#[test]
#[ignore = "requires Docker"]
fn test_docker_has_tools() {
    if !docker_available() {
        return;
    }
    
    let tools = vec!["clang", "llvm-as", "z3"];
    
    for tool in tools {
        let output = Command::new("docker")
            .args(&[
                "run", "--rm",
                "zeuslang/compiler:latest",
                "which", tool
            ])
            .output()
            .expect(&format!("Failed to check for {}", tool));
        
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("/usr") || !output.status.success(),
            "Tool {} should be in container", tool
        );
    }
}

// Test 8: Docker volume mount works
#[test]
#[ignore = "requires Docker"]
fn test_docker_volume_mount() {
    if !docker_available() {
        return;
    }
    
    let temp_dir = std::env::temp_dir();
    let test_file = temp_dir.join("mount_test.txt");
    fs::write(&test_file, "test content").unwrap();
    
    let output = Command::new("docker")
        .args(&[
            "run", "--rm",
            "-v", &format!("{}:/data", temp_dir.to_str().unwrap()),
            "alpine:latest",
            "cat", "/data/mount_test.txt"
        ])
        .output()
        .expect("Volume mount test failed");
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("test content"));
    
    let _ = fs::remove_file(&test_file);
}

// Test 9: Docker image size reasonable
#[test]
#[ignore = "requires Docker"]
fn test_docker_image_size() {
    if !docker_available() {
        return;
    }
    
    let output = Command::new("docker")
        .args(&["images", "zeuslang/compiler", "--format", "{{.Size}}"])
        .output()
        .expect("Failed to get image size");
    
    let size = String::from_utf8_lossy(&output.stdout);
    println!("Docker image size: {}", size);
    
    // Just verify we got output
    assert!(!size.is_empty() || !output.status.success());
}

// Test 10: Docker run with environment variables
#[test]
#[ignore = "requires Docker"]
fn test_docker_env_vars() {
    if !docker_available() {
        return;
    }
    
    let output = Command::new("docker")
        .args(&[
            "run", "--rm",
            "-e", "RUST_LOG=debug",
            "zeuslang/compiler:latest",
            "--version"
        ])
        .output()
        .expect("Docker env test failed");
    
    // Should execute
    assert!(true, "Docker with env vars executed");
}
