use rusqlite::{Connection, params};
use rusqlite_migration::{Migrations, M};
use shield_core::types::*;
use std::path::Path;
use std::sync::{Arc, Mutex};

pub type Db = Arc<Mutex<Connection>>;

const MIGRATIONS: &[M] = &[
    M::up("
        CREATE TABLE IF NOT EXISTS vulnerabilities (
            id          TEXT PRIMARY KEY,
            target_id   TEXT NOT NULL,
            title       TEXT NOT NULL,
            description TEXT NOT NULL,
            severity    TEXT NOT NULL,
            category    TEXT NOT NULL,
            cve_id      TEXT,
            cvss_score  REAL,
            evidence    TEXT NOT NULL,
            status      TEXT NOT NULL DEFAULT 'Open',
            discovered_at TEXT NOT NULL,
            fixed_at    TEXT
        );
        CREATE TABLE IF NOT EXISTS agents (
            id          TEXT PRIMARY KEY,
            hostname    TEXT NOT NULL,
            ip_address  TEXT NOT NULL,
            version     TEXT NOT NULL,
            status      TEXT NOT NULL DEFAULT 'Online',
            last_seen   TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS scan_jobs (
            id          TEXT PRIMARY KEY,
            target      TEXT NOT NULL,
            scan_type   TEXT NOT NULL,
            status      TEXT NOT NULL DEFAULT 'Pending',
            agent_id    TEXT,
            created_at  TEXT NOT NULL,
            started_at  TEXT,
            finished_at TEXT
        );
        CREATE TABLE IF NOT EXISTS patches (
            id          TEXT PRIMARY KEY,
            vuln_id     TEXT NOT NULL,
            description TEXT NOT NULL,
            diff        TEXT NOT NULL,
            patch_type  TEXT NOT NULL,
            confidence  REAL NOT NULL,
            verified    INTEGER NOT NULL DEFAULT 0,
            created_at  TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS api_keys (
            key_hash    TEXT PRIMARY KEY,
            label       TEXT NOT NULL,
            created_at  TEXT NOT NULL
        );
    "),
];

pub fn open(db_path: &str) -> Result<Db, rusqlite::Error> {
    let mut conn = if db_path == ":memory:" {
        Connection::open_in_memory()?
    } else {
        if let Some(parent) = Path::new(db_path).parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        Connection::open(db_path)?
    };

    // WAL mode for concurrent readers
    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;")?;

    // Run migrations
    let migrations = Migrations::new(MIGRATIONS.to_vec());
    migrations.to_latest(&mut conn)
        .map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;

    Ok(Arc::new(Mutex::new(conn)))
}

// ─── Vulnerabilities ─────────────────────────────────────────────────────────

pub fn insert_vulnerability(db: &Db, v: &Vulnerability) -> rusqlite::Result<()> {
    let conn = db.lock().unwrap();
    conn.execute(
        "INSERT OR REPLACE INTO vulnerabilities
         (id, target_id, title, description, severity, category, cve_id, cvss_score,
          evidence, status, discovered_at, fixed_at)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)",
        params![
            v.id.to_string(),
            v.target_id.to_string(),
            v.title,
            v.description,
            format!("{:?}", v.severity),
            format!("{:?}", v.category),
            v.cve_id,
            v.cvss_score,
            serde_json::to_string(&v.evidence).unwrap_or_default(),
            format!("{:?}", v.status),
            v.discovered_at.to_rfc3339(),
            v.fixed_at.map(|t| t.to_rfc3339()),
        ],
    )?;
    Ok(())
}

pub fn load_vulnerabilities(db: &Db) -> rusqlite::Result<Vec<Vulnerability>> {
    let conn = db.lock().unwrap();
    let mut stmt = conn.prepare(
        "SELECT id,target_id,title,description,severity,category,cve_id,cvss_score,
                evidence,status,discovered_at,fixed_at FROM vulnerabilities"
    )?;
    let rows = stmt.query_map([], |row| {
        let evidence_json: String = row.get(8)?;
        let evidence: Evidence = serde_json::from_str(&evidence_json)
            .unwrap_or_else(|_| Evidence {
                raw: evidence_json,
                reproduction_steps: vec![],
                affected_component: String::new(),
                network_trace: None,
            });
        Ok(Vulnerability {
            id: row.get::<_, String>(0)?.parse().unwrap_or_default(),
            target_id: row.get::<_, String>(1)?.parse().unwrap_or_default(),
            title: row.get(2)?,
            description: row.get(3)?,
            severity: parse_severity(&row.get::<_, String>(4)?),
            category: parse_category(&row.get::<_, String>(5)?),
            cve_id: row.get(6)?,
            cvss_score: row.get(7)?,
            evidence,
            status: parse_vuln_status(&row.get::<_, String>(9)?),
            discovered_at: row.get::<_, String>(10)?
                .parse().unwrap_or_else(|_| chrono::Utc::now()),
            fixed_at: row.get::<_, Option<String>>(11)?
                .and_then(|s| s.parse().ok()),
        })
    })?;
    rows.collect()
}

// ─── Agents ──────────────────────────────────────────────────────────────────

pub fn upsert_agent(db: &Db, a: &AgentInfo) -> rusqlite::Result<()> {
    let conn = db.lock().unwrap();
    conn.execute(
        "INSERT OR REPLACE INTO agents (id, hostname, ip_address, version, status, last_seen)
         VALUES (?1,?2,?3,?4,?5,?6)",
        params![
            a.id.to_string(),
            a.hostname,
            a.os,
            a.version,
            format!("{:?}", a.status),
            a.last_heartbeat.to_rfc3339(),
        ],
    )?;
    Ok(())
}

pub fn load_agents(db: &Db) -> rusqlite::Result<Vec<AgentInfo>> {
    let conn = db.lock().unwrap();
    let mut stmt = conn.prepare(
        "SELECT id, hostname, ip_address, version, status, last_seen FROM agents"
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(AgentInfo {
            id: row.get::<_, String>(0)?.parse().unwrap_or_default(),
            hostname: row.get(1)?,
            os: row.get(2)?,
            arch: String::new(),
            version: row.get(3)?,
            capabilities: vec![],
            status: parse_agent_status(&row.get::<_, String>(4)?),
            last_heartbeat: row.get::<_, String>(5)?
                .parse().unwrap_or_else(|_| chrono::Utc::now()),
        })
    })?;
    rows.collect()
}

// ─── Scan Jobs ───────────────────────────────────────────────────────────────

pub fn insert_scan_job(db: &Db, j: &ScanJob) -> rusqlite::Result<()> {
    let conn = db.lock().unwrap();
    let targets_json = serde_json::to_string(&j.targets).unwrap_or_default();
    conn.execute(
        "INSERT OR REPLACE INTO scan_jobs
         (id, target, scan_type, status, agent_id, created_at, started_at, finished_at)
         VALUES (?1,?2,?3,?4,NULL,?5,?6,?7)",
        params![
            j.id.to_string(),
            targets_json,
            format!("{:?}", j.scan_type),
            format!("{:?}", j.status),
            j.started_at.to_rfc3339(),
            j.started_at.to_rfc3339(),
            j.completed_at.map(|t| t.to_rfc3339()),
        ],
    )?;
    Ok(())
}

pub fn load_scan_jobs(db: &Db) -> rusqlite::Result<Vec<ScanJob>> {
    let conn = db.lock().unwrap();
    let mut stmt = conn.prepare(
        "SELECT id, target, scan_type, status, started_at, finished_at
         FROM scan_jobs"
    )?;
    let rows = stmt.query_map([], |row| {
        let targets_json: String = row.get(1)?;
        let targets: Vec<Target> = serde_json::from_str(&targets_json).unwrap_or_default();
        Ok(ScanJob {
            id: row.get::<_, String>(0)?.parse().unwrap_or_default(),
            targets,
            scan_type: parse_scan_type(&row.get::<_, String>(2)?),
            status: parse_job_status(&row.get::<_, String>(3)?),
            findings: vec![],
            started_at: row.get::<_, String>(4)?
                .parse().unwrap_or_else(|_| chrono::Utc::now()),
            completed_at: row.get::<_, Option<String>>(5)?
                .and_then(|s| s.parse().ok()),
        })
    })?;
    rows.collect()
}

pub fn update_scan_job_status(db: &Db, id: &str, status: &str) -> rusqlite::Result<()> {
    let conn = db.lock().unwrap();
    conn.execute(
        "UPDATE scan_jobs SET status=?1 WHERE id=?2",
        params![status, id],
    )?;
    Ok(())
}

// ─── Patches ─────────────────────────────────────────────────────────────────

pub fn insert_patch(db: &Db, p: &Patch) -> rusqlite::Result<()> {
    let conn = db.lock().unwrap();
    conn.execute(
        "INSERT OR REPLACE INTO patches
         (id, vuln_id, description, diff, patch_type, confidence, verified, created_at)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
        params![
            p.id.to_string(),
            p.vuln_id.to_string(),
            p.description,
            p.diff,
            format!("{:?}", p.patch_type),
            p.confidence,
            p.verified as i32,
            p.created_at.to_rfc3339(),
        ],
    )?;
    Ok(())
}

pub fn load_patches(db: &Db) -> rusqlite::Result<Vec<Patch>> {
    let conn = db.lock().unwrap();
    let mut stmt = conn.prepare(
        "SELECT id, vuln_id, description, diff, patch_type, confidence, verified, created_at
         FROM patches"
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(Patch {
            id: row.get::<_, String>(0)?.parse().unwrap_or_default(),
            vuln_id: row.get::<_, String>(1)?.parse().unwrap_or_default(),
            description: row.get(2)?,
            diff: row.get(3)?,
            patch_type: parse_patch_type(&row.get::<_, String>(4)?),
            confidence: row.get(5)?,
            verified: row.get::<_, i32>(6)? != 0,
            certificate: None,
            created_at: row.get::<_, String>(7)?
                .parse().unwrap_or_else(|_| chrono::Utc::now()),
        })
    })?;
    rows.collect()
}

// ─── API Keys ────────────────────────────────────────────────────────────────

pub fn insert_api_key(db: &Db, key_hash: &str, label: &str) -> rusqlite::Result<()> {
    let conn = db.lock().unwrap();
    conn.execute(
        "INSERT OR REPLACE INTO api_keys (key_hash, label, created_at) VALUES (?1,?2,?3)",
        params![key_hash, label, chrono::Utc::now().to_rfc3339()],
    )?;
    Ok(())
}

pub fn api_key_exists(db: &Db, key_hash: &str) -> bool {
    let conn = db.lock().unwrap();
    conn.query_row(
        "SELECT 1 FROM api_keys WHERE key_hash=?1",
        params![key_hash],
        |_| Ok(true),
    ).unwrap_or(false)
}

// ─── Enum parsers (Debug string → enum) ──────────────────────────────────────

fn parse_severity(s: &str) -> Severity {
    match s {
        "Critical" => Severity::Critical,
        "High"     => Severity::High,
        "Medium"   => Severity::Medium,
        "Low"      => Severity::Low,
        _          => Severity::Info,
    }
}

fn parse_category(s: &str) -> VulnCategory {
    match s {
        "NetworkExposure"   => VulnCategory::NetworkExposure,
        "Misconfiguration"  => VulnCategory::Misconfiguration,
        "OutdatedSoftware"  => VulnCategory::OutdatedSoftware,
        "SupplyChain"       => VulnCategory::SupplyChain,
        "DataExposure"      => VulnCategory::DataExposure,
        "WeakCredentials"   => VulnCategory::WeakCredentials,
        "InjectionFlaw"     => VulnCategory::InjectionFlaw,
        "BufferOverflow"    => VulnCategory::BufferOverflow,
        "TimingLeak"        => VulnCategory::TimingLeak,
        "PrivilegeEscalation" => VulnCategory::PrivilegeEscalation,
        "DenialOfService"   => VulnCategory::DenialOfService,
        "CodeVulnerability" => VulnCategory::CodeVulnerability,
        _                   => VulnCategory::Misconfiguration,
    }
}

fn parse_vuln_status(s: &str) -> VulnStatus {
    match s {
        "Open"         => VulnStatus::Open,
        "Confirmed"    => VulnStatus::Confirmed,
        "Reproducing"  => VulnStatus::Reproducing,
        "Patching"     => VulnStatus::Patching,
        "Verifying"    => VulnStatus::Verifying,
        "Fixed"        => VulnStatus::Fixed,
        "Accepted"     => VulnStatus::Accepted,
        "FalsePositive"=> VulnStatus::FalsePositive,
        _              => VulnStatus::Open,
    }
}

fn parse_agent_status(s: &str) -> AgentStatus {
    match s {
        "Online"  => AgentStatus::Online,
        "Offline" => AgentStatus::Offline,
        _         => AgentStatus::Offline,
    }
}

fn parse_scan_type(s: &str) -> ScanType {
    match s {
        "NetworkScan"      => ScanType::NetworkScan,
        "PortScan"         => ScanType::PortScan,
        "CodeAudit"        => ScanType::CodeAudit,
        "DependencyCheck"  => ScanType::DependencyCheck,
        "ConfigAudit"      => ScanType::ConfigAudit,
        "Continuous"       => ScanType::Continuous,
        _                  => ScanType::FullSuite,
    }
}

fn parse_job_status(s: &str) -> JobStatus {
    match s {
        "Pending"   => JobStatus::Pending,
        "Running"   => JobStatus::Running,
        "Completed" => JobStatus::Completed,
        "Failed"    => JobStatus::Failed,
        _           => JobStatus::Pending,
    }
}

fn parse_patch_type(s: &str) -> PatchType {
    match s {
        "CodeFix"          => PatchType::CodeFix,
        "ZeusSource"       => PatchType::ZeusSource,
        "ConfigChange"     => PatchType::ConfigChange,
        "DependencyUpdate" => PatchType::DependencyUpdate,
        "FirewallRule"     => PatchType::FirewallRule,
        "AccessControl"    => PatchType::AccessControl,
        "NetworkPolicy"    => PatchType::NetworkPolicy,
        _                  => PatchType::ConfigChange,
    }
}
