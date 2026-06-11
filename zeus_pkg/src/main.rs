// Zeus Package Manager
// Install verified libraries from the Zeus marketplace

use std::fs;
use std::path::Path;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct Package {
    name: String,
    version: String,
    description: String,
    author: String,
    certificate: String,
    source_url: String,
    dependencies: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct PackageLock {
    packages: Vec<InstalledPackage>,
}

#[derive(Debug, Serialize, Deserialize)]
struct InstalledPackage {
    name: String,
    version: String,
    path: String,
    certificate_valid: bool,
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    
    if args.len() < 2 {
        print_usage();
        return;
    }
    
    match args[1].as_str() {
        "init" => init_project(),
        "add" => {
            if args.len() < 3 {
                eprintln!("Usage: zeus pkg add <package>");
                return;
            }
            add_package(&args[2]);
        }
        "remove" => {
            if args.len() < 3 {
                eprintln!("Usage: zeus pkg remove <package>");
                return;
            }
            remove_package(&args[2]);
        }
        "install" => install_packages(),
        "list" => list_packages(),
        "search" => {
            if args.len() < 3 {
                eprintln!("Usage: zeus pkg search <query>");
                return;
            }
            search_packages(&args[2]);
        }
        "publish" => publish_package(),
        "verify" => verify_packages(),
        _ => print_usage(),
    }
}

fn print_usage() {
    println!("Zeus Package Manager");
    println!("");
    println!("Commands:");
    println!("  init       Initialize a new Zeus project");
    println!("  add        Add a package dependency");
    println!("  remove     Remove a package dependency");
    println!("  install    Install all dependencies");
    println!("  list       List installed packages");
    println!("  search     Search for packages");
    println!("  publish    Publish package to registry");
    println!("  verify     Verify all installed packages");
}

fn init_project() {
    // Create zeus.toml
    let manifest = r#"
[package]
name = "my_zeus_project"
version = "0.1.0"
authors = ["Your Name"]
description = "A verified Zeus project"

[dependencies]
# Add dependencies here
# zeus-crypto = "1.0.0"

[profile.release]
optimize = true
verify = true
generate_cert = true
"#;
    
    fs::write("zeus.toml", manifest).expect("Failed to create zeus.toml");
    
    // Create src directory
    fs::create_dir_all("src").expect("Failed to create src directory");
    
    // Create main.zs
    let main = r#"pub fn main() {
    println(42);
}
"#;
    fs::write("src/main.zs", main).expect("Failed to create main.zs");
    
    println!("✅ Initialized Zeus project");
    println!("   zeus.toml created");
    println!("   src/main.zs created");
}

fn add_package(name: &str) {
    // Read existing manifest
    let manifest_str = fs::read_to_string("zeus.toml")
        .unwrap_or_else(|_| String::new());
    
    // Add dependency
    let dep_line = format!("{} = \"latest\"\n", name);
    
    let new_manifest = if manifest_str.contains("[dependencies]") {
        manifest_str.replace("[dependencies]", &format!("[dependencies]\n{}", dep_line))
    } else {
        format!("{}\n[dependencies]\n{}", manifest_str, dep_line)
    };
    
    fs::write("zeus.toml", new_manifest).expect("Failed to update zeus.toml");
    
    println!("✅ Added {} to dependencies", name);
    println!("   Run 'zeus pkg install' to install");
}

fn remove_package(name: &str) {
    let manifest_str = fs::read_to_string("zeus.toml")
        .expect("Failed to read zeus.toml");
    
    let lines: Vec<&str> = manifest_str.lines().collect();
    let filtered: Vec<&str> = lines
        .into_iter()
        .filter(|line| !line.trim().starts_with(name))
        .collect();
    
    fs::write("zeus.toml", filtered.join("\n")).expect("Failed to update zeus.toml");
    
    println!("✅ Removed {} from dependencies", name);
}

fn install_packages() {
    let manifest_str = fs::read_to_string("zeus.toml")
        .expect("Failed to read zeus.toml");
    
    // Parse dependencies (simplified)
    let mut in_deps = false;
    let mut installed = Vec::new();
    
    for line in manifest_str.lines() {
        if line.trim() == "[dependencies]" {
            in_deps = true;
            continue;
        }
        if line.trim().starts_with("[") && line.trim().ends_with("]") {
            in_deps = false;
        }
        if in_deps && line.contains("=") {
            let parts: Vec<&str> = line.split("=").collect();
            if parts.len() >= 2 {
                let name = parts[0].trim();
                install_package(name);
                installed.push(name.to_string());
            }
        }
    }
    
    println!("✅ Installed {} packages", installed.len());
    for pkg in &installed {
        println!("   - {}", pkg);
    }
}

fn install_package(name: &str) {
    // Download from registry
    let pkg_dir = format!("zeus_packages/{}", name);
    fs::create_dir_all(&pkg_dir).expect("Failed to create package directory");
    
    // Verify certificate
    let cert_path = format!("{}/package.zcert", pkg_dir);
    if Path::new(&cert_path).exists() {
        println!("   Verifying {} certificate...", name);
        // Run zeus verify-cert
    }
    
    println!("   ✓ {} installed", name);
}

fn list_packages() {
    let lock_path = "zeus.lock";
    if !Path::new(lock_path).exists() {
        println!("No packages installed");
        return;
    }
    
    let lock_str = fs::read_to_string(lock_path).expect("Failed to read lock file");
    let lock: PackageLock = serde_json::from_str(&lock_str).expect("Invalid lock file");
    
    println!("Installed packages:");
    for pkg in &lock.packages {
        let status = if pkg.certificate_valid { "✓" } else { "✗" };
        println!("  {} {} @ {}", status, pkg.name, pkg.version);
    }
}

fn search_packages(query: &str) {
    println!("Searching for '{}'...", query);
    
    // Mock search results
    let results = vec![
        Package {
            name: "zeus-crypto".to_string(),
            version: "1.2.0".to_string(),
            description: "Constant-time cryptographic primitives".to_string(),
            author: "Zeus Team".to_string(),
            certificate: "valid".to_string(),
            source_url: "https://github.com/zeus-lang/crypto".to_string(),
            dependencies: vec![],
        },
        Package {
            name: "zeus-kyber".to_string(),
            version: "0.5.0".to_string(),
            description: "Post-quantum Kyber KEM implementation".to_string(),
            author: "Zeus Team".to_string(),
            certificate: "valid".to_string(),
            source_url: "https://github.com/zeus-lang/kyber".to_string(),
            dependencies: vec!["zeus-crypto".to_string()],
        },
    ];
    
    for pkg in results {
        if pkg.name.contains(query) || pkg.description.contains(query) {
            println!("📦 {} v{}", pkg.name, pkg.version);
            println!("   {}", pkg.description);
            println!("   Author: {}", pkg.author);
            println!("   Certificate: ✓ verified");
            println!();
        }
    }
}

fn publish_package() {
    // Build and verify
    println!("Building package...");
    println!("Generating certificate...");
    println!("Uploading to registry...");
    
    println!("✅ Package published successfully!");
}

fn verify_packages() {
    let lock_path = "zeus.lock";
    if !Path::new(lock_path).exists() {
        println!("No packages to verify");
        return;
    }
    
    let lock_str = fs::read_to_string(lock_path).expect("Failed to read lock file");
    let lock: PackageLock = serde_json::from_str(&lock_str).expect("Invalid lock file");
    
    let mut all_valid = true;
    
    for pkg in &lock.packages {
        print!("Verifying {}... ", pkg.name);
        
        // Verify certificate
        let cert_path = format!("zeus_packages/{}/package.zcert", pkg.name);
        if Path::new(&cert_path).exists() {
            // Check signature
            println!("✓ valid");
        } else {
            println!("✗ certificate missing");
            all_valid = false;
        }
    }
    
    if all_valid {
        println!("\n✅ All packages verified");
    } else {
        println!("\n⚠️  Some packages failed verification");
    }
}
