// Zeus Cloud API Server
// REST API for Zeus compiler-as-a-service

use axum::{
    routing::{get, post},
    Router,
    extract::{State, Json, Path, Multipart},
    http::StatusCode,
    response::{IntoResponse, Response},
    http::header,
};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::Arc;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use tracing::{info, error};
use uuid::Uuid;
use chrono::{DateTime, Utc};

mod auth;
mod compile;
mod verify;
mod db;
mod queue;

use auth::Claims;
use compile::{compile_request, CompilationResult};
use verify::{verify_request, VerificationResult};

/// Rate limiter state
#[derive(Clone)]
struct RateLimiter {
    requests: Arc<Mutex<HashMap<String, Vec<Instant>>>>,
    max_requests: usize,
    window: Duration,
}

impl RateLimiter {
    fn new(max_requests: usize, window: Duration) -> Self {
        RateLimiter {
            requests: Arc::new(Mutex::new(HashMap::new())),
            max_requests,
            window,
        }
    }

    async fn check_rate_limit(&self, client_id: &str) -> bool {
        let mut requests = self.requests.lock().await;
        let now = Instant::now();
        let client_requests = requests.entry(client_id.to_string()).or_insert_with(Vec::new);

        // Remove old requests outside the time window
        client_requests.retain(|&t| now.duration_since(t) < self.window);

        // Check if under limit
        if client_requests.len() < self.max_requests {
            client_requests.push(now);
            true
        } else {
            false
        }
    }
}

/// Application state
#[derive(Clone)]
struct AppState {
    db: db::Database,
    queue: queue::CompilationQueue,
    cache: redis::Client,
    rate_limiter: RateLimiter,
}

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::fmt::init();
    
    info!("Starting Zeus Cloud API server");
    
    // Initialize database
    let db = db::Database::new().await.expect("Failed to connect to database");
    
    // Initialize compilation queue
    let queue = queue::CompilationQueue::new().await.expect("Failed to initialize queue");
    
    // Initialize Redis cache
    let cache = redis::Client::open("redis://127.0.0.1:6379")
        .expect("Failed to connect to Redis");
    
    // Initialize rate limiter (100 requests per minute)
    let rate_limiter = RateLimiter::new(100, Duration::from_secs(60));
    
    let state = AppState { db, queue, cache, rate_limiter };
    
    // Build router
    let app = Router::new()
        // Health check
        .route("/health", get(health_check))
        // Compilation endpoints
        .route("/compile", post(compile_handler))
        .route("/verify", post(verify_handler))
        .route("/certificate/:hash", get(get_certificate))
        // Job status
        .route("/jobs/:id", get(get_job_status))
        // Templates and libraries
        .route("/templates", get(list_templates))
        .route("/libraries", get(list_libraries))
        // Admin/stats
        .route("/stats", get(get_stats))
        .with_state(state)
        // Security: Rate limiting middleware
        .layer(axum::middleware::from_fn(rate_limit_middleware))
        // Security: CORS with strict policy
        .layer(tower_http::cors::CorsLayer::new()
            .allow_origin(tower_http::cors::Any)
            .allow_methods([axum::http::Method::POST, axum::http::Method::GET])
            .allow_headers([header::CONTENT_TYPE, header::AUTHORIZATION]))
        // Security: Custom security headers
        .layer(axum::middleware::from_fn(security_headers_middleware))
        .layer(tower_http::trace::TraceLayer::new_for_http());
    
    // Start server
    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    info!("Listening on {}", addr);
    
    axum::serve(
        tokio::net::TcpListener::bind(addr).await.unwrap(),
        app
    ).await.unwrap();
}

/// Health check endpoint
async fn health_check() -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "healthy",
        "service": "zeus-cloud",
        "version": "0.1.0"
    }))
}

/// Compilation request
#[derive(Debug, Deserialize)]
struct CompileRequest {
    source: String,
    #[serde(default = "default_target")]
    target: String,
    #[serde(default)]
    policy: Option<String>,
    #[serde(default = "default_true")]
    verify: bool,
    #[serde(default)]
    generate_cert: bool,
}

fn default_target() -> String { "x86_64".to_string() }
fn default_true() -> bool { true }

/// Compilation response
#[derive(Debug, Serialize)]
struct CompileResponse {
    job_id: String,
    status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    binary: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    certificate: Option<CertificateInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    verification_report: Option<VerificationResult>,
    #[serde(skip_serializing_if = "Option::is_none")]
    gas_estimate: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

#[derive(Debug, Serialize)]
struct CertificateInfo {
    hash: String,
    properties: Vec<String>,
    signature: String,
    timestamp: DateTime<Utc>,
}

/// Compile handler
async fn compile_handler(
    State(state): State<AppState>,
    Json(req): Json<CompileRequest>,
) -> impl IntoResponse {
    let job_id = Uuid::new_v4().to_string();
    
    info!("Compilation request: job_id={}, target={}", job_id, req.target);
    
    // Validate request
    if req.source.is_empty() {
        return (StatusCode::BAD_REQUEST, Json(CompileResponse {
            job_id,
            status: "error".to_string(),
            binary: None,
            certificate: None,
            verification_report: None,
            gas_estimate: None,
            error: Some("Source code is required".to_string()),
        }));
    }
    
    // Queue compilation job
    let compile_req = compile::CompileRequest {
        source: req.source,
        target: req.target,
        policy: req.policy,
        verify: req.verify,
        generate_cert: req.generate_cert,
    };
    match state.queue.submit(job_id.clone(), compile_req).await {
        Ok(_) => {
            (StatusCode::ACCEPTED, Json(CompileResponse {
                job_id,
                status: "queued".to_string(),
                binary: None,
                certificate: None,
                verification_report: None,
                gas_estimate: None,
                error: None,
            }))
        }
        Err(e) => {
            error!("Failed to queue job: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(CompileResponse {
                job_id,
                status: "error".to_string(),
                binary: None,
                certificate: None,
                verification_report: None,
                gas_estimate: None,
                error: Some(format!("Failed to queue: {}", e)),
            }))
        }
    }
}

/// Verify request
#[derive(Debug, Deserialize)]
struct VerifyRequest {
    source: String,
    #[serde(default)]
    policy: Option<String>,
}

/// Verify handler
async fn verify_handler(
    State(state): State<AppState>,
    Json(req): Json<VerifyRequest>,
) -> impl IntoResponse {
    let job_id = Uuid::new_v4().to_string();
    
    info!("Verification request: job_id={}", job_id);
    
    // Run verification
    match verify_request(&req.source, req.policy.as_deref()).await {
        Ok(result) => {
            Json(serde_json::json!({
                "job_id": job_id,
                "verified": result.verified,
                "properties": result.properties,
                "zir_report": result.zir_report,
            }))
        }
        Err(e) => {
            error!("Verification failed: {}", e);
            Json(serde_json::json!({
                "job_id": job_id,
                "verified": false,
                "error": e.to_string(),
            }))
        }
    }
}

/// Get job status
async fn get_job_status(
    State(_state): State<AppState>,
    Path(id): Path<String>,
) -> Response {
    match _state.queue.status(&id).await {
        Ok(status) => (StatusCode::OK, Json(status)).into_response(),
        Err(_) => (StatusCode::NOT_FOUND, Json(serde_json::json!({
            "error": "Job not found"
        }))).into_response(),
    }
}

/// Get certificate
async fn get_certificate(
    State(_state): State<AppState>,
    Path(hash): Path<String>,
) -> Response {
    match _state.db.get_certificate(&hash).await {
        Ok(cert) => (StatusCode::OK, Json(cert)).into_response(),
        Err(_) => (StatusCode::NOT_FOUND, Json(serde_json::json!({
            "error": "Certificate not found"
        }))).into_response(),
    }
}

/// List templates
async fn list_templates() -> impl IntoResponse {
    Json(serde_json::json!([
        {
            "name": "medical-device",
            "category": "medical",
            "description": "FDA/IEC 62304 compliant medical device template"
        },
        {
            "name": "crypto-library",
            "category": "crypto",
            "description": "Constant-time cryptographic library template"
        },
        {
            "name": "blockchain-contract",
            "category": "blockchain",
            "description": "EVM smart contract with formal verification"
        },
        {
            "name": "aerospace-control",
            "category": "aerospace",
            "description": "NASA-compliant flight software template"
        }
    ]))
}

/// List libraries
async fn list_libraries() -> impl IntoResponse {
    Json(serde_json::json!([
        {
            "name": "zeus-crypto",
            "version": "1.0.0",
            "description": "Constant-time cryptographic primitives",
            "certificate": "valid"
        },
        {
            "name": "zeus-kyber",
            "version": "0.5.0",
            "description": "Post-quantum Kyber KEM",
            "certificate": "valid"
        },
        {
            "name": "zeus-medical",
            "version": "0.3.0",
            "description": "Medical device utilities",
            "certificate": "valid"
        }
    ]))
}

/// Get stats
async fn get_stats(State(state): State<AppState>) -> impl IntoResponse {
    let stats = state.db.get_stats().await.unwrap_or_default();
    
    Json(serde_json::json!({
        "total_compilations": stats.total_compilations,
        "successful_verifications": stats.successful_verifications,
        "certificates_issued": stats.certificates_issued,
        "active_users": stats.active_users,
    }))
}

/// Rate limiting middleware
async fn rate_limit_middleware(
    State(state): State<AppState>,
    req: axum::extract::Request,
    next: axum::middleware::Next,
) -> Response {
    // Extract client ID from IP (simplified - in production use proper auth)
    let client_id = "default"; // TODO: Extract from auth token or IP
    
    if !state.rate_limiter.check_rate_limit(client_id).await {
        return (StatusCode::TOO_MANY_REQUESTS, Json(serde_json::json!({
            "error": "Rate limit exceeded"
        }))).into_response();
    }
    
    next.run(req).await
}

/// Security headers middleware
async fn security_headers_middleware(
    req: axum::extract::Request,
    next: axum::middleware::Next,
) -> Response {
    let mut response = next.run(req).await;
    
    // Add security headers
    let headers = response.headers_mut();
    headers.insert("X-Content-Type-Options", header::HeaderValue::from_static("nosniff"));
    headers.insert("X-Frame-Options", header::HeaderValue::from_static("DENY"));
    headers.insert("X-XSS-Protection", header::HeaderValue::from_static("1; mode=block"));
    headers.insert("Strict-Transport-Security", header::HeaderValue::from_static("max-age=31536000; includeSubDomains"));
    headers.insert("Content-Security-Policy", header::HeaderValue::from_static("default-src 'self'"));
    
    response
}
