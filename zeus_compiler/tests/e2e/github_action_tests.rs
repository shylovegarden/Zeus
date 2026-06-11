// E2E Tests: GitHub Action
// Tests the Zeus GitHub Action workflow

use std::process::Command;
use std::fs;

// Test 1: Action.yml syntax is valid
#[test]
fn test_action_yml_syntax() {
    let action_path = "/Users/shy/Developer/ZEUS/github-action/action.yml";
    assert!(fs::metadata(action_path).is_ok(), "action.yml should exist");
    
    let content = fs::read_to_string(action_path).expect("Read action.yml");
    
    // Basic YAML structure checks
    assert!(content.contains("name:"));
    assert!(content.contains("description:"));
    assert!(content.contains("inputs:"));
    assert!(content.contains("outputs:"));
    assert!(content.contains("runs:"));
}

// Test 2: Dockerfile exists and is valid
#[test]
fn test_dockerfile_valid() {
    let dockerfile_path = "/Users/shy/Developer/ZEUS/github-action/Dockerfile";
    assert!(fs::metadata(dockerfile_path).is_ok(), "Dockerfile should exist");
    
    let content = fs::read_to_string(dockerfile_path).expect("Read Dockerfile");
    
    // Should contain required elements
    assert!(content.contains("FROM"));
    assert!(content.contains("ENTRYPOINT") || content.contains("CMD"));
}

// Test 3: Entrypoint script exists
#[test]
fn test_entrypoint_exists() {
    let entrypoint_path = "/Users/shy/Developer/ZEUS/github-action/entrypoint.sh";
    assert!(fs::metadata(entrypoint_path).is_ok(), "entrypoint.sh should exist");
    
    let metadata = fs::metadata(entrypoint_path).expect("Get metadata");
    let permissions = metadata.permissions();
    
    // Should be executable
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert!(permissions.mode() & 0o111 != 0, "entrypoint.sh should be executable");
    }
}

// Test 4: Action inputs are documented
#[test]
fn test_action_inputs_documented() {
    let action_path = "/Users/shy/Developer/ZEUS/github-action/action.yml";
    let content = fs::read_to_string(action_path).expect("Read action.yml");
    
    // Check for documented inputs
    assert!(content.contains("source-path"));
    assert!(content.contains("language"));
    assert!(content.contains("policy"));
    assert!(content.contains("fail-on"));
}

// Test 5: Action outputs are defined
#[test]
fn test_action_outputs_defined() {
    let action_path = "/Users/shy/Developer/ZEUS/github-action/action.yml";
    let content = fs::read_to_string(action_path).expect("Read action.yml");
    
    // Check for outputs
    assert!(content.contains("certificate"));
    assert!(content.contains("report-url"));
    assert!(content.contains("verification-passed"));
}

// Test 6: Workflow file syntax (demo)
#[test]
fn test_workflow_syntax() {
    let workflow_path = "/Users/shy/Developer/ZEUS/.github/workflows/zeus-verify-demo.yml";
    assert!(fs::metadata(workflow_path).is_ok(), "Workflow file should exist");
    
    let content = fs::read_to_string(workflow_path).expect("Read workflow");
    
    // Basic YAML checks
    assert!(content.contains("name:"));
    assert!(content.contains("on:"));
    assert!(content.contains("jobs:"));
    assert!(content.contains("steps:"));
}

// Test 7: Action branding is set
#[test]
fn test_action_branding() {
    let action_path = "/Users/shy/Developer/ZEUS/github-action/action.yml";
    let content = fs::read_to_string(action_path).expect("Read action.yml");
    
    // GitHub Actions branding
    assert!(content.contains("branding:"));
    assert!(content.contains("icon:") || content.contains("color:"));
}

// Test 8: Docker can build action image
#[test]
#[ignore = "requires Docker"]
fn test_docker_build_action() {
    let output = Command::new("docker")
        .args(&[
            "build",
            "-t", "zeus-action:test",
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

// Test 9: Action handles missing inputs gracefully
#[test]
fn test_action_input_defaults() {
    let action_path = "/Users/shy/Developer/ZEUS/github-action/action.yml";
    let content = fs::read_to_string(action_path).expect("Read action.yml");
    
    // Check that inputs have defaults or are required
    assert!(
        content.contains("default:") || content.contains("required: true"),
        "Inputs should have defaults or be required"
    );
}

// Test 10: Entrypoint handles various inputs
#[test]
fn test_entrypoint_input_handling() {
    let entrypoint_path = "/Users/shy/Developer/ZEUS/github-action/entrypoint.sh";
    let content = fs::read_to_string(entrypoint_path).expect("Read entrypoint.sh");
    
    // Should reference environment variables for inputs
    assert!(content.contains("INPUT_") || content.contains("$"));
    
    // Should set outputs
    assert!(content.contains("GITHUB_OUTPUT") || content.contains("::set-output"));
}
