use serde::{Deserialize, Serialize};
use tracing::{info, warn};

/// CVE Database entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CveEntry {
    pub id: String,
    pub description: String,
    pub severity: String,
    pub cvss_score: Option<f64>,
    pub affected_packages: Vec<AffectedPackage>,
    pub references: Vec<String>,
    pub published: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AffectedPackage {
    pub name: String,
    pub ecosystem: String,
    pub fixed_version: Option<String>,
}

// ─── OSV API types ──────────────────────────────────────────────────────────

#[derive(Serialize)]
struct OsvQuery {
    package: OsvPackage,
    version: String,
}

#[derive(Serialize)]
struct OsvPackage {
    name: String,
    ecosystem: String,
}

#[derive(Deserialize, Debug)]
struct OsvResponse {
    vulns: Option<Vec<OsvVuln>>,
}

#[derive(Deserialize, Debug)]
struct OsvVuln {
    id: String,
    summary: Option<String>,
    details: Option<String>,
    published: Option<String>,
    references: Option<Vec<OsvRef>>,
    affected: Option<Vec<OsvAffected>>,
    severity: Option<Vec<OsvSeverity>>,
}

#[derive(Deserialize, Debug)]
struct OsvRef {
    url: String,
}

#[derive(Deserialize, Debug)]
struct OsvAffected {
    package: Option<OsvPkg>,
    ranges: Option<Vec<OsvRange>>,
}

#[derive(Deserialize, Debug)]
struct OsvPkg {
    name: Option<String>,
    ecosystem: Option<String>,
}

#[derive(Deserialize, Debug)]
struct OsvRange {
    events: Option<Vec<OsvEvent>>,
}

#[derive(Deserialize, Debug, Default)]
struct OsvEvent {
    fixed: Option<String>,
    introduced: Option<String>,
}

#[derive(Deserialize, Debug)]
struct OsvSeverity {
    score: Option<String>,
}

/// CVE database client — queries the OSV API (https://osv.dev)
pub struct CveDatabase {
    client: reqwest::Client,
    api_url: String,
}

impl CveDatabase {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .expect("failed to build HTTP client"),
            api_url: "https://api.osv.dev/v1".to_string(),
        }
    }

    /// Query OSV for vulnerabilities in a specific package version.
    /// ecosystem: "crates.io", "npm", "PyPI", "Maven", "Go", "Debian", etc.
    pub async fn query_package(
        &self,
        package: &str,
        version: &str,
        ecosystem: &str,
    ) -> Vec<CveEntry> {
        let url = format!("{}/query", self.api_url);
        let body = OsvQuery {
            package: OsvPackage {
                name: package.to_string(),
                ecosystem: ecosystem.to_string(),
            },
            version: version.to_string(),
        };

        match self.client.post(&url).json(&body).send().await {
            Ok(resp) if resp.status().is_success() => {
                match resp.json::<OsvResponse>().await {
                    Ok(osv) => {
                        let vulns = osv.vulns.unwrap_or_default();
                        info!("OSV: {} vulns for {}@{}", vulns.len(), package, version);
                        vulns.into_iter().map(|v| osv_to_entry(v)).collect()
                    }
                    Err(e) => {
                        warn!("OSV parse error: {}", e);
                        vec![]
                    }
                }
            }
            Ok(resp) => {
                warn!("OSV API returned HTTP {}", resp.status());
                vec![]
            }
            Err(e) => {
                warn!("OSV API unreachable: {}", e);
                vec![]
            }
        }
    }

    /// Batch query multiple packages concurrently
    pub async fn batch_query(
        &self,
        packages: &[(String, String, String)], // (name, version, ecosystem)
    ) -> Vec<(String, Vec<CveEntry>)> {
        let mut handles = Vec::new();

        for (name, version, ecosystem) in packages {
            let url = format!("{}/query", self.api_url);
            let client = self.client.clone();
            let name = name.clone();
            let version = version.clone();
            let ecosystem = ecosystem.clone();

            handles.push(tokio::spawn(async move {
                let body = OsvQuery {
                    package: OsvPackage { name: name.clone(), ecosystem },
                    version,
                };
                let result = client.post(&url).json(&body).send().await;
                match result {
                    Ok(resp) if resp.status().is_success() => {
                        let osv = resp.json::<OsvResponse>().await.unwrap_or(OsvResponse { vulns: None });
                        let entries: Vec<CveEntry> = osv.vulns.unwrap_or_default()
                            .into_iter().map(osv_to_entry).collect();
                        (name, entries)
                    }
                    _ => (name, vec![]),
                }
            }));
        }

        let mut results = Vec::new();
        for h in handles {
            if let Ok((name, cves)) = h.await {
                if !cves.is_empty() {
                    results.push((name, cves));
                }
            }
        }
        results
    }
}

fn osv_to_entry(v: OsvVuln) -> CveEntry {
    let description = v.summary.or(v.details).unwrap_or_default();
    let published = v.published.unwrap_or_default();
    let references = v.references.unwrap_or_default()
        .into_iter().map(|r| r.url).collect();

    // Extract CVSS score from severity block
    let cvss_score = v.severity
        .and_then(|s| s.into_iter().next())
        .and_then(|s| s.score)
        .and_then(|s| {
            // CVSS vectors like "CVSS:3.1/AV:N/AC:L/..." — extract base score from the end
            s.split('/').last().and_then(|sc| sc.parse::<f64>().ok())
        });

    let affected_packages = v.affected.unwrap_or_default()
        .into_iter()
        .map(|a| {
            let fixed = a.ranges.unwrap_or_default()
                .into_iter()
                .flat_map(|r| r.events.unwrap_or_default())
                .find_map(|e| e.fixed);
            AffectedPackage {
                name: a.package.as_ref().and_then(|p| p.name.clone()).unwrap_or_default(),
                ecosystem: a.package.and_then(|p| p.ecosystem).unwrap_or_default(),
                fixed_version: fixed,
            }
        })
        .collect();

    let severity = match cvss_score {
        Some(s) if s >= 9.0 => "critical",
        Some(s) if s >= 7.0 => "high",
        Some(s) if s >= 4.0 => "medium",
        Some(_) => "low",
        None => "unknown",
    }.to_string();

    CveEntry {
        id: v.id,
        description,
        severity,
        cvss_score,
        affected_packages,
        references,
        published,
    }
}
