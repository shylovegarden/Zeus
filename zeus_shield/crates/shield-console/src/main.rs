mod db;

use axum::{
    extract::{State, Request},
    http::StatusCode,
    middleware::{self, Next},
    response::Response,
    routing::{get, post},
    Json, Router,
};
use axum_server::tls_rustls::RustlsConfig;
use db::Db;
use sha2::{Sha256, Digest};
use shield_core::traits::Patcher;
use shield_core::types::*;
use shield_patch::PatchEngine;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing::{info, warn};

#[derive(Clone)]
struct AppState {
    db: Db,
    // In-memory cache (populated from DB on startup, kept in sync on writes)
    vulnerabilities: Arc<RwLock<Vec<Vulnerability>>>,
    agents: Arc<RwLock<Vec<AgentInfo>>>,
    scan_jobs: Arc<RwLock<Vec<ScanJob>>>,
    patches: Arc<RwLock<Vec<Patch>>>,
    // API key auth: SHA-256 hashed keys
    require_auth: bool,
}

impl AppState {
    fn new(db: Db, require_auth: bool) -> Self {
        let vulns   = db::load_vulnerabilities(&db).unwrap_or_default();
        let agents  = db::load_agents(&db).unwrap_or_default();
        let scans   = db::load_scan_jobs(&db).unwrap_or_default();
        let patches = db::load_patches(&db).unwrap_or_default();

        info!("Loaded from DB: {} vulns, {} agents, {} scans, {} patches",
            vulns.len(), agents.len(), scans.len(), patches.len());

        Self {
            db,
            vulnerabilities: Arc::new(RwLock::new(vulns)),
            agents: Arc::new(RwLock::new(agents)),
            scan_jobs: Arc::new(RwLock::new(scans)),
            patches: Arc::new(RwLock::new(patches)),
            require_auth,
        }
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    // DB path: $ZEUS_DB_PATH or ~/.zeus-shield/console.db
    let db_path = std::env::var("ZEUS_DB_PATH").unwrap_or_else(|_| {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
        format!("{}/.zeus-shield/console.db", home)
    });
    info!("Database: {}", db_path);

    let db = db::open(&db_path).expect("Failed to open database");

    // Auto-provision a default API key if none exist
    let require_auth = std::env::var("ZEUS_NO_AUTH").is_err();
    if require_auth {
        let key_count = {
            let conn = db.lock().unwrap();
            conn.query_row("SELECT COUNT(*) FROM api_keys", [], |r| r.get::<_, i64>(0))
                .unwrap_or(0)
        };
        if key_count == 0 {
            let default_key = format!("zs-{}", uuid::Uuid::new_v4().to_string().replace('-', ""));
            let hash = hash_api_key(&default_key);
            db::insert_api_key(&db, &hash, "default").ok();
            info!("╔══════════════════════════════════════════════════════╗");
            info!("║  AUTO-GENERATED API KEY (save this — shown once)    ║");
            info!("║  {}  ║", default_key);
            info!("╚══════════════════════════════════════════════════════╝");
            info!("Use: -H \"X-API-Key: {}\"", default_key);
        }
    } else {
        info!("Auth disabled (ZEUS_NO_AUTH set)");
    }

    let state = AppState::new(db, require_auth);

    let protected = Router::new()
        .route("/api/v1/status", get(get_status))
        .route("/api/v1/vulnerabilities", get(list_vulnerabilities))
        .route("/api/v1/vulnerabilities", post(report_vulnerability))
        .route("/api/v1/agents", get(list_agents))
        .route("/api/v1/agents/heartbeat", post(agent_heartbeat))
        .route("/api/v1/scans", get(list_scans))
        .route("/api/v1/scans", post(create_scan))
        .route("/api/v1/patches", get(list_patches))
        .route("/api/v1/patches/generate", post(generate_patch))
        .route_layer(middleware::from_fn_with_state(state.clone(), auth_middleware));

    let app = Router::new()
        .merge(protected)
        .route("/health", get(health_check))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let addr: std::net::SocketAddr = "0.0.0.0:8443".parse().unwrap();

    // Generate a self-signed TLS certificate if no external cert is configured
    let cert_path = std::env::var("ZEUS_TLS_CERT").unwrap_or_else(|_| {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
        format!("{}/.zeus-shield/tls/cert.pem", home)
    });
    let key_path = std::env::var("ZEUS_TLS_KEY").unwrap_or_else(|_| {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
        format!("{}/.zeus-shield/tls/key.pem", home)
    });

    // Generate self-signed cert if it doesn't exist yet
    if !std::path::Path::new(&cert_path).exists() {
        generate_self_signed_cert(&cert_path, &key_path)
            .expect("Failed to generate self-signed TLS cert");
        info!("Generated self-signed TLS certificate at {}", cert_path);
    }

    let tls_config = RustlsConfig::from_pem_file(&cert_path, &key_path)
        .await
        .expect("Failed to load TLS config");

    info!("Zeus Shield Console starting on https://{}", addr);
    info!("API: https://{}/api/v1/status", addr);
    info!("TLS: self-signed cert at {}", cert_path);

    axum_server::bind_rustls(addr, tls_config)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

fn generate_self_signed_cert(cert_path: &str, key_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    use rcgen::{generate_simple_self_signed, CertifiedKey};
    let subject_alt_names = vec!["localhost".to_string(), "127.0.0.1".to_string()];
    let CertifiedKey { cert, key_pair } = generate_simple_self_signed(subject_alt_names)?;

    if let Some(parent) = std::path::Path::new(cert_path).parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(cert_path, cert.pem())?;
    std::fs::write(key_path, key_pair.serialize_pem())?;
    Ok(())
}

fn hash_api_key(key: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(key.as_bytes());
    format!("{:x}", hasher.finalize())
}

async fn auth_middleware(
    State(state): State<AppState>,
    req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    if !state.require_auth {
        return Ok(next.run(req).await);
    }
    let key = req.headers()
        .get("X-API-Key")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if key.is_empty() {
        warn!("Request missing X-API-Key header");
        return Err(StatusCode::UNAUTHORIZED);
    }

    let hash = hash_api_key(key);
    if !db::api_key_exists(&state.db, &hash) {
        warn!("Invalid API key attempt");
        return Err(StatusCode::FORBIDDEN);
    }
    Ok(next.run(req).await)
}

// ─── Handlers ───────────────────────────────────────────────────────────────

async fn get_status(State(state): State<AppState>) -> Json<serde_json::Value> {
    let vulns = state.vulnerabilities.read().await;
    let agents = state.agents.read().await;
    let scans = state.scan_jobs.read().await;
    let patches = state.patches.read().await;

    Json(serde_json::json!({
        "version": env!("CARGO_PKG_VERSION"),
        "product": "Zeus Shield",
        "stats": {
            "total_vulnerabilities": vulns.len(),
            "critical_vulns": vulns.iter().filter(|v| v.severity == Severity::Critical).count(),
            "high_vulns": vulns.iter().filter(|v| v.severity == Severity::High).count(),
            "active_agents": agents.iter().filter(|a| a.status == AgentStatus::Online).count(),
            "total_agents": agents.len(),
            "active_scans": scans.iter().filter(|s| s.status == JobStatus::Running).count(),
            "patches_applied": patches.iter().filter(|p| p.verified).count(),
        }
    }))
}

async fn list_vulnerabilities(State(state): State<AppState>) -> Json<Vec<Vulnerability>> {
    let vulns = state.vulnerabilities.read().await;
    Json(vulns.clone())
}

async fn report_vulnerability(
    State(state): State<AppState>,
    Json(vuln): Json<Vulnerability>,
) -> Json<serde_json::Value> {
    let id = vuln.id;
    if let Err(e) = db::insert_vulnerability(&state.db, &vuln) {
        warn!("DB insert_vulnerability failed: {}", e);
    }
    let mut vulns = state.vulnerabilities.write().await;
    if let Some(existing) = vulns.iter_mut().find(|v| v.id == id) {
        *existing = vuln;
    } else {
        vulns.push(vuln);
    }
    Json(serde_json::json!({ "id": id, "status": "reported" }))
}

async fn list_agents(State(state): State<AppState>) -> Json<Vec<AgentInfo>> {
    let agents = state.agents.read().await;
    Json(agents.clone())
}

async fn agent_heartbeat(
    State(state): State<AppState>,
    Json(agent): Json<AgentInfo>,
) -> Json<serde_json::Value> {
    if let Err(e) = db::upsert_agent(&state.db, &agent) {
        warn!("DB upsert_agent failed: {}", e);
    }
    let mut agents = state.agents.write().await;
    if let Some(existing) = agents.iter_mut().find(|a| a.id == agent.id) {
        existing.last_heartbeat = agent.last_heartbeat;
        existing.status = agent.status.clone();
    } else {
        agents.push(agent);
    }
    Json(serde_json::json!({ "status": "ok" }))
}

async fn list_scans(State(state): State<AppState>) -> Json<Vec<ScanJob>> {
    let scans = state.scan_jobs.read().await;
    Json(scans.clone())
}

async fn create_scan(
    State(state): State<AppState>,
    Json(scan): Json<ScanJob>,
) -> Json<serde_json::Value> {
    let id = scan.id;
    if let Err(e) = db::insert_scan_job(&state.db, &scan) {
        warn!("DB insert_scan_job failed: {}", e);
    }
    state.scan_jobs.write().await.push(scan.clone());

    // Dispatch to an online agent if one exists
    let agent_url = {
        let agents = state.agents.read().await;
        agents.iter()
            .find(|a| a.status == AgentStatus::Online)
            .map(|a| a.hostname.clone())
    };

    let dispatched = if let Some(hostname) = agent_url {
        // Build the job payload agents understand: {id, scan_type, target}
        let first_target = scan.targets.first().map(|t| t.address.clone()).unwrap_or_default();
        let scan_type_str = match scan.scan_type {
            ScanType::NetworkScan | ScanType::PortScan => "network",
            ScanType::CodeAudit                        => "code",
            ScanType::ConfigAudit | ScanType::DependencyCheck => "device",
            _                                          => "full",
        };

        let payload = serde_json::json!({
            "id": id.to_string(),
            "scan_type": scan_type_str,
            "target": first_target,
        });

        // POST the job directly to the agent's /run-job endpoint
        let agent_endpoint = format!("http://{}:9443/run-job", hostname);
        let client = reqwest::Client::new();
        match client.post(&agent_endpoint).json(&payload).send().await {
            Ok(resp) if resp.status().is_success() => {
                info!("Dispatched scan {} to agent at {}", id, hostname);
                db::update_scan_job_status(&state.db, &id.to_string(), "Running").ok();
                true
            }
            Ok(resp) => {
                warn!("Agent at {} returned {}", hostname, resp.status());
                false
            }
            Err(e) => {
                warn!("Failed to dispatch to agent at {}: {}", hostname, e);
                false
            }
        }
    } else {
        warn!("No online agents available to dispatch scan {}", id);
        false
    };

    Json(serde_json::json!({
        "id": id,
        "status": if dispatched { "dispatched" } else { "queued" },
        "dispatched_to_agent": dispatched,
    }))
}

async fn list_patches(State(state): State<AppState>) -> Json<Vec<Patch>> {
    let patches = state.patches.read().await;
    Json(patches.clone())
}

async fn generate_patch(
    State(state): State<AppState>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    // Find vulnerability by id from state
    let vuln_id_str = body.get("vuln_id")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let vuln = {
        let vulns = state.vulnerabilities.read().await;
        vulns.iter()
            .find(|v| v.id.to_string() == vuln_id_str)
            .cloned()
    };

    let vuln = match vuln {
        Some(v) => v,
        None => return Json(serde_json::json!({
            "error": format!("Vulnerability {} not found", vuln_id_str)
        })),
    };

    let engine = PatchEngine::new();
    if !engine.can_fix(&vuln) {
        return Json(serde_json::json!({
            "status": "no_fix_available",
            "vuln_id": vuln_id_str,
            "title": vuln.title,
        }));
    }

    match engine.generate_patch(&vuln).await {
        Ok(patch) => {
            let patch_id = patch.id;
            if let Err(e) = db::insert_patch(&state.db, &patch) {
                warn!("DB insert_patch failed: {}", e);
            }
            state.patches.write().await.push(patch.clone());
            Json(serde_json::json!({
                "status": "generated",
                "patch_id": patch_id,
                "vuln_id": vuln_id_str,
                "description": patch.description,
                "confidence": patch.confidence,
                "patch_type": format!("{:?}", patch.patch_type),
                "diff": patch.diff,
            }))
        }
        Err(e) => Json(serde_json::json!({
            "error": format!("Patch generation failed: {}", e),
            "vuln_id": vuln_id_str,
        }))
    }
}

async fn health_check() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "healthy",
        "timestamp": chrono::Utc::now().to_rfc3339(),
    }))
}
