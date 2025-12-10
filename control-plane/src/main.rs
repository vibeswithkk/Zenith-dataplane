use axum::{
    routing::{get, post, delete},
    Router, Json, extract::{Path, State},
    http::StatusCode,
};
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use tower_http::cors::CorsLayer;
use tracing::info;
use uuid::Uuid;

mod models;
use models::*;

/// Application state
#[derive(Clone)]
struct AppState {
    nodes: Arc<Mutex<HashMap<String, DataNode>>>,
    plugins: Arc<Mutex<HashMap<String, Plugin>>>,
    deployments: Arc<Mutex<HashMap<String, Deployment>>>,
    start_time: std::time::Instant,
}

impl AppState {
    fn new() -> Self {
        Self {
            nodes: Arc::new(Mutex::new(HashMap::new())),
            plugins: Arc::new(Mutex::new(HashMap::new())),
            deployments: Arc::new(Mutex::new(HashMap::new())),
            start_time: std::time::Instant::now(),
        }
    }
}

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    let state = AppState::new();

    // Build router
    let app = Router::new()
        // Health & Info
        .route("/health", get(health_check))
        .route("/api/v1/info", get(get_info))
        
        // Node Management
        .route("/api/v1/nodes", get(list_nodes))
        .route("/api/v1/nodes", post(register_node))
        .route("/api/v1/nodes/:id", get(get_node))
        .route("/api/v1/nodes/:id", delete(deregister_node))
        
        // Plugin Management
        .route("/api/v1/plugins", get(list_plugins))
        .route("/api/v1/plugins", post(register_plugin))
        .route("/api/v1/plugins/:id", delete(delete_plugin))
        
        // Deployment Management
        .route("/api/v1/deployments", get(list_deployments))
        .route("/api/v1/deployments", post(create_deployment))
        .route("/api/v1/deployments/:id", delete(delete_deployment))
        
        .layer(CorsLayer::permissive())
        .with_state(state);

    let addr = "0.0.0.0:9090";
    info!("[START] Zenith Control Plane starting on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

// Health check
async fn health_check() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "healthy".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

// Get system info
async fn get_info(State(state): State<AppState>) -> Json<SystemInfo> {
    let nodes = state.nodes.lock().unwrap();
    let plugins = state.plugins.lock().unwrap();
    let deployments = state.deployments.lock().unwrap();
    let uptime = state.start_time.elapsed().as_secs();

    Json(SystemInfo {
        node_count: nodes.len(),
        plugin_count: plugins.len(),
        deployment_count: deployments.len(),
        uptime_seconds: uptime,
    })
}

// Node management
async fn list_nodes(State(state): State<AppState>) -> Json<Vec<DataNode>> {
    let nodes = state.nodes.lock().unwrap();
    Json(nodes.values().cloned().collect())
}

async fn register_node(
    State(state): State<AppState>,
    Json(req): Json<RegisterNodeRequest>,
) -> Result<Json<DataNode>, StatusCode> {
    let node = DataNode {
        id: Uuid::new_v4().to_string(),
        address: req.address,
        capacity: req.capacity,
        status: NodeStatus::Active,
        registered_at: chrono::Utc::now(),
    };

    let mut nodes = state.nodes.lock().unwrap();
    nodes.insert(node.id.clone(), node.clone());

    info!("Registered node: {}", node.id);
    Ok(Json(node))
}

async fn get_node(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<DataNode>, StatusCode> {
    let nodes = state.nodes.lock().unwrap();
    nodes.get(&id)
        .cloned()
        .map(Json)
        .ok_or(StatusCode::NOT_FOUND)
}

async fn deregister_node(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, StatusCode> {
    let mut nodes = state.nodes.lock().unwrap();
    nodes.remove(&id)
        .ok_or(StatusCode::NOT_FOUND)?;
    
    info!("Deregistered node: {}", id);
    Ok(StatusCode::NO_CONTENT)
}

// Plugin management
async fn list_plugins(State(state): State<AppState>) -> Json<Vec<Plugin>> {
    let plugins = state.plugins.lock().unwrap();
    Json(plugins.values().cloned().collect())
}

async fn register_plugin(
    State(state): State<AppState>,
    Json(req): Json<RegisterPluginRequest>,
) -> Result<Json<Plugin>, StatusCode> {
    let plugin = Plugin {
        id: Uuid::new_v4().to_string(),
        name: req.name,
        version: req.version,
        wasm_url: req.wasm_url,
        created_at: chrono::Utc::now(),
    };

    let mut plugins = state.plugins.lock().unwrap();
    plugins.insert(plugin.id.clone(), plugin.clone());

    info!("Registered plugin: {}", plugin.id);
    Ok(Json(plugin))
}

async fn delete_plugin(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, StatusCode> {
    let mut plugins = state.plugins.lock().unwrap();
    plugins.remove(&id)
        .ok_or(StatusCode::NOT_FOUND)?;
    
    info!("Deleted plugin: {}", id);
    Ok(StatusCode::NO_CONTENT)
}

// Deployment management
async fn list_deployments(State(state): State<AppState>) -> Json<Vec<Deployment>> {
    let deployments = state.deployments.lock().unwrap();
    Json(deployments.values().cloned().collect())
}

async fn create_deployment(
    State(state): State<AppState>,
    Json(req): Json<CreateDeploymentRequest>,
) -> Result<Json<Deployment>, StatusCode> {
    let deployment = Deployment {
        id: Uuid::new_v4().to_string(),
        plugin_id: req.plugin_id,
        node_ids: req.node_ids,
        status: DeploymentStatus::Pending,
        created_at: chrono::Utc::now(),
    };

    let mut deployments = state.deployments.lock().unwrap();
    deployments.insert(deployment.id.clone(), deployment.clone());

    info!("Created deployment: {}", deployment.id);
    Ok(Json(deployment))
}

async fn delete_deployment(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, StatusCode> {
    let mut deployments = state.deployments.lock().unwrap();
    deployments.remove(&id)
        .ok_or(StatusCode::NOT_FOUND)?;
    
    info!("Deleted deployment: {}", id);
    Ok(StatusCode::NO_CONTENT)
}
