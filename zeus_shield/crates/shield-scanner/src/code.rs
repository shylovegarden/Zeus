use async_trait::async_trait;
use shield_core::{
    error::ShieldResult,
    types::*,
    traits::Scanner,
};
use uuid::Uuid;
use chrono::Utc;
use std::path::PathBuf;

pub struct CodeScanner {
    zeus_binary: PathBuf,
}

impl CodeScanner {
    pub fn new(zeus_binary: PathBuf) -> Self {
        Self { zeus_binary }
    }

    /// Run Zeus audit on a source file
    async fn zeus_audit(&self, path: &str) -> ShieldResult<Vec<Vulnerability>> {
        let output = tokio::process::Command::new(&self.zeus_binary)
            .args(["audit", path, "--json"])
            .output()
            .await
            .map_err(|e| shield_core::error::ShieldError::Scan(
                format!("Failed to run zeus audit: {}", e)
            ))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        
        // Parse Zeus audit JSON output for findings
        let mut vulns = Vec::new();
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&stdout) {
            if let Some(findings) = json.get("findings").and_then(|f| f.as_array()) {
                for finding in findings {
                    let title = finding.get("title")
                        .and_then(|t| t.as_str())
                        .unwrap_or("Unknown finding")
                        .to_string();
                    let description = finding.get("description")
                        .and_then(|d| d.as_str())
                        .unwrap_or("")
                        .to_string();
                    let severity = match finding.get("severity")
                        .and_then(|s| s.as_str())
                        .unwrap_or("medium") 
                    {
                        "critical" => Severity::Critical,
                        "high" => Severity::High,
                        "medium" => Severity::Medium,
                        "low" => Severity::Low,
                        _ => Severity::Info,
                    };

                    vulns.push(Vulnerability {
                        id: Uuid::new_v4(),
                        target_id: Uuid::nil(),
                        title,
                        description,
                        severity,
                        category: VulnCategory::CodeVulnerability,
                        cve_id: None,
                        cvss_score: None,
                        evidence: Evidence {
                            raw: finding.to_string(),
                            reproduction_steps: vec![],
                            affected_component: path.to_string(),
                            network_trace: None,
                        },
                        status: VulnStatus::Open,
                        discovered_at: Utc::now(),
                        fixed_at: None,
                    });
                }
            }
        }

        Ok(vulns)
    }

    /// Check dependencies for known CVEs via OSV API
    async fn check_dependencies(&self, path: &str) -> ShieldResult<Vec<Vulnerability>> {
        let base = std::path::Path::new(path);

        // Collect (ecosystem, package_name) pairs from manifest files
        let mut packages: Vec<(String, String)> = Vec::new();

        // Cargo.toml
        let cargo_path = if base.is_dir() {
            base.join("Cargo.toml")
        } else {
            base.to_path_buf()
        };
        if cargo_path.exists() {
            if let Ok(content) = tokio::fs::read_to_string(&cargo_path).await {
                parse_cargo_toml(&content, &mut packages);
            }
        }

        // package.json
        let pkg_path = if base.is_dir() { base.join("package.json") } else { base.to_path_buf() };
        if pkg_path.exists() && pkg_path.to_string_lossy().ends_with("package.json") {
            if let Ok(content) = tokio::fs::read_to_string(&pkg_path).await {
                parse_package_json(&content, &mut packages);
            }
        }

        // requirements.txt
        let req_path = if base.is_dir() { base.join("requirements.txt") } else { base.to_path_buf() };
        if req_path.exists() && req_path.to_string_lossy().ends_with("requirements.txt") {
            if let Ok(content) = tokio::fs::read_to_string(&req_path).await {
                parse_requirements_txt(&content, &mut packages);
            }
        }

        if packages.is_empty() {
            return Ok(vec![]);
        }

        tracing::info!("Checking {} packages against OSV", packages.len());

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .build()
            .map_err(|e| shield_core::error::ShieldError::Scan(e.to_string()))?;

        let mut vulns = Vec::new();

        for (ecosystem, pkg_name) in &packages {
            let body = serde_json::json!({
                "package": { "name": pkg_name, "ecosystem": ecosystem }
            });

            let resp = client
                .post("https://api.osv.dev/v1/query")
                .json(&body)
                .send()
                .await;

            let json: serde_json::Value = match resp {
                Ok(r) if r.status().is_success() => r.json().await.unwrap_or_default(),
                _ => continue,
            };

            let advisories = match json.get("vulns").and_then(|v| v.as_array()) {
                Some(a) if !a.is_empty() => a,
                _ => continue,
            };

            for adv in advisories {
                let osv_id = adv.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
                let summary = adv.get("summary").and_then(|v| v.as_str())
                    .unwrap_or("Vulnerable dependency").to_string();
                let details = adv.get("details").and_then(|v| v.as_str())
                    .unwrap_or("").to_string();

                // Detect CVE alias
                let cve_id = adv.get("aliases")
                    .and_then(|a| a.as_array())
                    .and_then(|arr| arr.iter().find(|s| {
                        s.as_str().map(|x| x.starts_with("CVE-")).unwrap_or(false)
                    }))
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                // Derive severity from CVSS if present
                let severity = adv.get("severity")
                    .and_then(|s| s.as_array())
                    .and_then(|arr| arr.first())
                    .and_then(|entry| entry.get("score").and_then(|s| s.as_f64()))
                    .map(|score| {
                        if score >= 9.0 { Severity::Critical }
                        else if score >= 7.0 { Severity::High }
                        else if score >= 4.0 { Severity::Medium }
                        else { Severity::Low }
                    })
                    .unwrap_or(Severity::Medium);

                vulns.push(Vulnerability {
                    id: Uuid::new_v4(),
                    target_id: Uuid::nil(),
                    title: format!("{}: {} ({})", pkg_name, summary, osv_id),
                    description: if details.is_empty() { summary.clone() } else { details },
                    severity,
                    category: VulnCategory::SupplyChain,
                    cve_id,
                    cvss_score: None,
                    evidence: Evidence {
                        raw: format!("OSV advisory {} for {} ({})", osv_id, pkg_name, ecosystem),
                        reproduction_steps: vec![
                            format!("Package: {} ({})", pkg_name, ecosystem),
                            format!("Advisory: https://osv.dev/vulnerability/{}", osv_id),
                        ],
                        affected_component: pkg_name.clone(),
                        network_trace: None,
                    },
                    status: VulnStatus::Open,
                    discovered_at: Utc::now(),
                    fixed_at: None,
                });
            }
        }

        tracing::info!("Found {} dependency CVEs", vulns.len());
        Ok(vulns)
    }

    /// Detect hardcoded secrets in source files
    async fn detect_secrets(&self, path: &str) -> ShieldResult<Vec<Vulnerability>> {
        use std::path::Path;
        let p = Path::new(path);
        if !p.exists() {
            return Ok(vec![]);
        }

        // Walk files if directory, single file otherwise
        let files: Vec<std::path::PathBuf> = if p.is_dir() {
            walkdir(p)
        } else {
            vec![p.to_path_buf()]
        };

        let mut vulns = Vec::new();
        for file in files {
            if let Ok(content) = tokio::fs::read_to_string(&file).await {
                let findings = scan_for_secrets(&content, &file.to_string_lossy());
                vulns.extend(findings);
            }
        }
        Ok(vulns)
    }
}

#[async_trait]
impl Scanner for CodeScanner {
    fn name(&self) -> &str {
        "code-scanner"
    }

    fn supported_targets(&self) -> Vec<TargetKind> {
        vec![TargetKind::Repository, TargetKind::Application]
    }

    async fn discover(&self) -> ShieldResult<Vec<Target>> {
        // Walk CWD and $HOME for directories containing a source manifest
        let search_roots = {
            let mut roots = vec![std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."))];
            if let Ok(home) = std::env::var("HOME") {
                roots.push(std::path::PathBuf::from(home));
            }
            roots
        };

        let manifest_names = ["Cargo.toml", "package.json", "requirements.txt", "go.mod", "pom.xml"];
        let mut targets = Vec::new();
        let mut seen = std::collections::HashSet::new();

        for root in search_roots {
            find_repos(&root, &manifest_names, 4, &mut targets, &mut seen);
        }

        tracing::info!("CodeScanner discovered {} source repositories", targets.len());
        Ok(targets)
    }

    async fn scan(&self, target: &Target) -> ShieldResult<Vec<Vulnerability>> {
        let path = &target.address;
        let mut all_vulns = Vec::new();

        // Run Zeus audit
        let audit_vulns = self.zeus_audit(path).await?;
        all_vulns.extend(audit_vulns);

        // Check dependencies
        let dep_vulns = self.check_dependencies(path).await?;
        all_vulns.extend(dep_vulns);

        // Detect secrets
        let secret_vulns = self.detect_secrets(path).await?;
        all_vulns.extend(secret_vulns);

        // Set target_id on all vulns
        for vuln in &mut all_vulns {
            vuln.target_id = target.id;
        }

        Ok(all_vulns)
    }

    async fn health_check(&self) -> ShieldResult<()> {
        if !self.zeus_binary.exists() {
            return Err(shield_core::error::ShieldError::Config(
                format!("Zeus binary not found at {:?}", self.zeus_binary)
            ));
        }
        Ok(())
    }
}

// ─── Secret Detection ────────────────────────────────────────────────────────

/// (pattern_name, regex_pattern, severity)
static SECRET_PATTERNS: &[(&str, &str, &str)] = &[
    ("AWS Access Key",          r"(?i)AKIA[0-9A-Z]{16}",                         "critical"),
    ("AWS Secret Key",          r"(?i)aws[_\-\.]?secret[_\-\.]?key\s*[:=]\s*\S+", "critical"),
    ("GitHub Token",            r"ghp_[A-Za-z0-9]{36}",                           "critical"),
    ("GitHub OAuth",            r"gho_[A-Za-z0-9]{36}",                           "critical"),
    ("Slack Token",             r"xox[baprs]-[A-Za-z0-9\-]+",                     "high"),
    ("Private Key PEM",         r"-----BEGIN (?:RSA |EC )?PRIVATE KEY-----",       "critical"),
    ("Generic password",        r#"(?i)password\s*[:=]\s*["']?[^\s"']{8,}"#,      "high"),
    ("Generic secret",          r#"(?i)secret\s*[:=]\s*["']?[^\s"']{8,}"#,        "high"),
    ("Generic API key",         r#"(?i)api[_\-]?key\s*[:=]\s*["']?[^\s"']{16,}"#, "high"),
    ("Bearer token",            r"(?i)bearer\s+[A-Za-z0-9\-_\.]{20,}",            "medium"),
    ("Basic auth in URL",       r"https?://[^:@\s]+:[^@\s]+@",                    "high"),
    ("Database URL with creds", r"(?i)(postgres|mysql|mongodb)://[^:]+:[^@]+@",   "critical"),
    ("JWT token",               r"eyJ[A-Za-z0-9\-_]+\.eyJ[A-Za-z0-9\-_]+\.[A-Za-z0-9\-_]+", "medium"),
];

/// Skip binary and media files
static SKIP_EXTENSIONS: &[&str] = &[
    "png", "jpg", "jpeg", "gif", "svg", "ico", "pdf", "zip", "tar",
    "gz", "exe", "bin", "so", "dll", "lock", "sum",
];

fn should_skip(path: &str) -> bool {
    let path_lower = path.to_lowercase();
    if path_lower.contains("/.git/") || path_lower.contains("/target/") {
        return true;
    }
    SKIP_EXTENSIONS.iter().any(|ext| path_lower.ends_with(ext))
}

fn scan_for_secrets(content: &str, file_path: &str) -> Vec<Vulnerability> {
    if should_skip(file_path) {
        return vec![];
    }

    let mut findings = Vec::new();
    for (name, pattern, severity) in SECRET_PATTERNS {
        // Simple linear scan without regex crate dependency
        let matches = simple_pattern_match(content, pattern, name);
        for (line_num, matched_text) in matches {
            let severity_level = match *severity {
                "critical" => Severity::Critical,
                "high"     => Severity::High,
                "medium"   => Severity::Medium,
                _          => Severity::Low,
            };
            findings.push(Vulnerability {
                id: uuid::Uuid::new_v4(),
                target_id: uuid::Uuid::nil(),
                title: format!("Hardcoded secret: {}", name),
                description: format!(
                    "Possible {} found in {}:{}. Remove and rotate immediately.",
                    name, file_path, line_num
                ),
                severity: severity_level,
                category: VulnCategory::DataExposure,
                cve_id: None,
                cvss_score: None,
                evidence: Evidence {
                    raw: format!("{}:{} matched pattern '{}'", file_path, line_num, name),
                    reproduction_steps: vec![
                        format!("cat {} | grep -n '{}'", file_path, name),
                    ],
                    affected_component: file_path.to_string(),
                    network_trace: None,
                },
                status: VulnStatus::Open,
                discovered_at: chrono::Utc::now(),
                fixed_at: None,
            });
        }
    }
    findings
}

/// Minimal pattern matcher: checks each line for case-insensitive keyword presence.
/// For production, swap in the `regex` crate.
fn simple_pattern_match(content: &str, pattern: &str, name: &str) -> Vec<(usize, String)> {
    let mut hits = Vec::new();
    // Extract a simple keyword from the pattern for fast scanning
    let keyword = extract_keyword(pattern);
    for (i, line) in content.lines().enumerate() {
        if line.to_lowercase().contains(&keyword.to_lowercase()) {
            hits.push((i + 1, line.trim().chars().take(80).collect()));
        }
    }
    hits
}

fn extract_keyword(pattern: &str) -> String {
    // Strip regex metacharacters to get the literal anchor
    let cleaned: String = pattern.chars()
        .take_while(|c| c.is_alphanumeric() || *c == '_' || *c == '-')
        .collect();
    if cleaned.len() >= 4 {
        cleaned.to_lowercase()
    } else {
        // Fallback: grab any 4+ char alpha run
        pattern.chars()
            .filter(|c| c.is_alphabetic())
            .collect::<String>()
            .chars()
            .take(10)
            .collect::<String>()
            .to_lowercase()
    }
}

/// Parse [dependencies] block from Cargo.toml into (ecosystem, name) pairs
fn parse_cargo_toml(content: &str, out: &mut Vec<(String, String)>) {
    let mut in_deps = false;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed == "[dependencies]" || trimmed == "[dev-dependencies]" || trimmed == "[build-dependencies]" {
            in_deps = true;
            continue;
        }
        if trimmed.starts_with('[') {
            in_deps = false;
            continue;
        }
        if in_deps {
            if let Some(name) = trimmed.split('=').next() {
                let name = name.trim().trim_matches('"');
                if !name.is_empty() && !name.starts_with('#') {
                    out.push(("crates.io".to_string(), name.to_string()));
                }
            }
        }
    }
}

/// Parse dependencies from package.json into (ecosystem, name) pairs
fn parse_package_json(content: &str, out: &mut Vec<(String, String)>) {
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(content) {
        for key in &["dependencies", "devDependencies", "peerDependencies"] {
            if let Some(deps) = json.get(key).and_then(|d| d.as_object()) {
                for name in deps.keys() {
                    out.push(("npm".to_string(), name.clone()));
                }
            }
        }
    }
}

/// Parse requirements.txt into (ecosystem, name) pairs
fn parse_requirements_txt(content: &str, out: &mut Vec<(String, String)>) {
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with('-') {
            continue;
        }
        // Strip version specifiers: requests>=2.0 → requests
        let name = line
            .split(|c: char| c == '>' || c == '<' || c == '=' || c == '!' || c == '[' || c == ';')
            .next()
            .unwrap_or(line)
            .trim();
        if !name.is_empty() {
            out.push(("PyPI".to_string(), name.to_string()));
        }
    }
}

/// Recursively find directories containing source manifests, up to `max_depth`
fn find_repos(
    dir: &std::path::Path,
    manifests: &[&str],
    max_depth: usize,
    out: &mut Vec<Target>,
    seen: &mut std::collections::HashSet<std::path::PathBuf>,
) {
    if max_depth == 0 { return; }

    // Skip hidden dirs, node_modules, target, .git, venv
    let dir_name = dir.file_name().and_then(|n| n.to_str()).unwrap_or("");
    if dir_name.starts_with('.') || dir_name == "node_modules" || dir_name == "target"
        || dir_name == "venv" || dir_name == "__pycache__" {
        return;
    }

    let canonical = match dir.canonicalize() {
        Ok(p) => p,
        Err(_) => return,
    };
    if !seen.insert(canonical.clone()) { return; }

    for manifest in manifests {
        if dir.join(manifest).exists() {
            out.push(Target {
                id: uuid::Uuid::new_v4(),
                name: canonical.to_string_lossy().to_string(),
                kind: TargetKind::Repository,
                address: canonical.to_string_lossy().to_string(),
                metadata: serde_json::json!({ "manifest": manifest }),
                created_at: chrono::Utc::now(),
            });
            return; // one target per dir, even if multiple manifests
        }
    }

    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                find_repos(&path, manifests, max_depth - 1, out, seen);
            }
        }
    }
}

fn walkdir(dir: &std::path::Path) -> Vec<std::path::PathBuf> {
    let mut out = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                out.extend(walkdir(&path));
            } else {
                out.push(path);
            }
        }
    }
    out
}
