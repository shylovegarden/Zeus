use std::fs;
use std::path::Path;
use std::process::Command;
use serde_json::{Value, json, Map};
use sha2::{Sha256, Digest};

pub struct PackageManager {
    packages: Vec<String>,
}

impl PackageManager {
    pub fn new() -> Self {
        PackageManager {
            packages: Vec::new(),
        }
    }

    pub fn add_package(&mut self, url: &str) {
        self.packages.push(url.to_string());
    }

    pub fn fetch_dependencies(&self) {
        let modules_dir = Path::new(".zeus_modules");
        if !modules_dir.exists() {
            fs::create_dir_all(modules_dir).expect("Failed to create .zeus_modules directory");
        }

        let lock_path = Path::new("zeus.lock");
        let mut lock_data: Map<String, Value> = Map::new();
        
        if lock_path.exists() {
            let content = fs::read_to_string(lock_path).unwrap_or_default();
            if let Ok(Value::Object(map)) = serde_json::from_str(&content) {
                lock_data = map;
            }
        }

        // 3. Transitive Sandboxing Restrictions:
        // Since the PM is simple, if any existing dependency in zeus.lock has allow_network: false,
        // we cascade this capability restriction strictly, blocking any new network fetching.
        let mut network_allowed = true;
        for (_, entry) in lock_data.iter() {
            if let Some(allow) = entry.get("allow_network") {
                if allow.as_bool() == Some(false) {
                    network_allowed = false;
                }
            }
        }

        for url in &self.packages {
            let is_file = url.starts_with("file://");
            
            if !is_file && !network_allowed {
                panic!("Network access is disabled by a transitive capability restriction. Cannot fetch {}", url);
            }

            let filename = url.split('/').last().unwrap_or("package.zs");
            let dest_path = modules_dir.join(filename);

            println!("Fetching {}...", url);

            // 2. Air-Gapped Local Mirror: Use local file if file://
            if is_file {
                let local_path = url.strip_prefix("file://").unwrap();
                if let Err(e) = fs::copy(local_path, &dest_path) {
                    panic!("Failed to copy local file from {}: {}", local_path, e);
                }
                println!("Successfully copied {} to {:?}", url, dest_path);
            } else {
                let status = Command::new("curl")
                    .args(&["-L", "-s", "-o"])
                    .arg(&dest_path)
                    .arg(url)
                    .status()
                    .expect("Failed to execute curl command");

                if status.success() {
                    println!("Successfully downloaded {} to {:?}", url, dest_path);
                } else {
                    panic!("Failed to download {}. curl exited with status: {}", url, status);
                }
            }

            // 1. SHA-256 Pinning
            let file_bytes = fs::read(&dest_path).expect("Failed to read downloaded module");
            let mut hasher = Sha256::new();
            hasher.update(&file_bytes);
            let result = hasher.finalize();
            // Prefixing with sha256: to match standard format
            let hash_hex = format!("sha256:{:x}", result);

            if let Some(existing_entry) = lock_data.get(url) {
                if let Some(existing_hash) = existing_entry.get("hash").and_then(|v| v.as_str()) {
                    if existing_hash != hash_hex {
                        panic!("Security Abort: Hash mismatch for {}. Expected {}, got {}", url, existing_hash, hash_hex);
                    }
                } else {
                    let mut obj = existing_entry.as_object().unwrap().clone();
                    obj.insert("hash".to_string(), json!(hash_hex));
                    lock_data.insert(url.to_string(), Value::Object(obj));
                }
            } else {
                // By default, dependencies downloaded with file:// have no network access.
                // HTTP dependencies default to false unless configured otherwise.
                let allow_network = if !network_allowed { false } else { !is_file };
                lock_data.insert(url.to_string(), json!({
                    "url": url,
                    "hash": hash_hex,
                    "allow_network": allow_network
                }));
            }
        }

        let lock_content = serde_json::to_string_pretty(&lock_data).expect("Failed to serialize zeus.lock");
        fs::write("zeus.lock", lock_content).expect("Failed to write zeus.lock file");
        println!("Generated zeus.lock mapping URLs to capabilities.");
    }
}
