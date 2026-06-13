use serde::{Deserialize, Serialize};

/// CVE Database entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CveEntry {
    pub id: String,
    pub description: String,
    pub severity: String,
    pub cvss_score: f64,
    pub affected_packages: Vec<AffectedPackage>,
    pub references: Vec<String>,
    pub published: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AffectedPackage {
    pub name: String,
    pub ecosystem: String,
    pub version_range: String,
    pub fixed_version: Option<String>,
}

/// CVE database client (queries OSV or NVD)
pub struct CveDatabase {
    api_url: String,
}

impl CveDatabase {
    pub fn new() -> Self {
        Self {
            api_url: "https://api.osv.dev/v1".to_string(),
        }
    }

    pub fn with_api_url(mut self, url: &str) -> Self {
        self.api_url = url.to_string();
        self
    }

    /// Query CVEs for a specific package
    pub async fn query_package(
        &self,
        package: &str,
        version: &str,
        ecosystem: &str,
    ) -> Vec<CveEntry> {
        // TODO: Implement actual OSV/NVD API query
        // POST to https://api.osv.dev/v1/query
        // {
        //   "package": { "name": package, "ecosystem": ecosystem },
        //   "version": version
        // }
        vec![]
    }

    /// Batch query multiple packages
    pub async fn batch_query(
        &self,
        packages: &[(String, String, String)], // (name, version, ecosystem)
    ) -> Vec<(String, Vec<CveEntry>)> {
        let mut results = Vec::new();
        for (name, version, ecosystem) in packages {
            let cves = self.query_package(name, version, ecosystem).await;
            if !cves.is_empty() {
                results.push((name.clone(), cves));
            }
        }
        results
    }
}
