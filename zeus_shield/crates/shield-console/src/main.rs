use axum::{
    extract::State,
    routing::{get, post},
    Json, Router,
};
use shield_core::types::*;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing::info;

#[derive(Clone)]
struct AppState {
    vulnerabilities: Arc<RwLock<Vec<Vulnerability>>>,
    agents: Arc<RwLock<Vec<AgentInfo>>>,
    scan_jobs: Arc<RwLock<Vec<ScanJob>>>,
    patches: Arc<RwLock<Vec<Patch>>>,
}

impl AppState {
    fn new() -> Self {
        Self {
            vulnerabilities: Arc::new(RwLock::new(Vec::new())),
            agents: Arc::new(RwLock::new(Vec::new())),
            scan_jobs: Arc::new(RwLock::new(Vec::new())),
            patches: Arc::new(RwLock::new(Vec::new())),
        }
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    let state = AppState::new();

    let app = Router::new()
        // Dashboard
        .route("/api/v1/status", get(get_status))
        // Vulnerabilities
        .route("/api/v1/vulnerabilities", get(list_vulnerabilities))
        .route("/api/v1/vulnerabilities", post(report_vulnerability))
        // Agents
        .route("/api/v1/agents", get(list_agents))
        .route("/api/v1/agents/heartbeat", post(agent_heartbeat))
        // Scans
        .route("/api/v1/scans", get(list_scans))
        .route("/api/v1/scans", post(create_scan))
        // Patches
        .route("/api/v1/patches", get(list_patches))
        .route("/api/v1/patches/generate", post(generate_patch))
        // Health
        .route("/health", get(health_check))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let addr = "0.0.0.0:8443";
    info!("Zeus Shield Console starting on {}", addr);
    info!("API: http://{}/api/v1/status", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
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
    let mut vulns = state.vulnerabilities.write().await;
    let id = vuln.id;
    vulns.push(vuln);
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
    let mut agents = state.agents.write().await;
    
    // Update existing or add new
    if let Some(existing) = agents.iter_mut().find(|a| a.id == agent.id) {
        existing.last_heartbeat = agent.last_heartbeat;
        existing.status = agent.status;
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
    let mut scans = state.scan_jobs.write().await;
    let id = scan.id;
    scans.push(scan);
    Json(serde_json::json!({ "id": id, "status": "created" }))
}

async fn list_patches(State(state): State<AppState>) -> Json<Vec<Patch>> {
    let patches = state.patches.read().await;
    Json(patches.clone())
}

async fn generate_patch(
    State(state): State<AppState>,
    Json(vuln_id): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    // TODO: Integrate with PatchEngine
    Json(serde_json::json!({
        "status": "patch_generation_queued",
        "vuln_id": vuln_id,
    }))
}

async fn health_check() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "healthy",
        "timestamp": chrono::Utc::now().to_rfc3339(),
    }))
}
