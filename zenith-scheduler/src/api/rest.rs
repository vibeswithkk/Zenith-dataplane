//! REST API Implementation for Scheduler

use axum::{
    Router,
    routing::{get, post, delete},
    response::{Json, IntoResponse},
    extract::{State, Path},
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::collections::HashMap;

use crate::scheduler::Scheduler;
use crate::node::NodeRegistry;
use crate::job::{Job, JobDescriptor, ResourceRequirements, LocalityPreferences, SchedulingPolicy};

/// Application state
pub struct AppState {
    pub scheduler: Arc<Scheduler>,
    pub node_registry: Arc<NodeRegistry>,
}

/// Create REST API router
pub fn create_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/api/v1/jobs", post(submit_job))
        .route("/api/v1/jobs", get(list_jobs))
        .route("/api/v1/jobs/:job_id", get(get_job))
        .route("/api/v1/jobs/:job_id", delete(cancel_job))
        .route("/api/v1/cluster/status", get(cluster_status))
        .route("/api/v1/nodes", get(list_nodes))
        .route("/health", get(health_check))
        .with_state(state)
}

// === Request/Response Types ===

#[derive(Debug, Deserialize)]
pub struct SubmitJobRequest {
    pub name: String,
    pub user_id: String,
    pub project_id: String,
    pub command: String,
    #[serde(default)]
    pub arguments: Vec<String>,
    #[serde(default)]
    pub environment: HashMap<String, String>,
    #[serde(default = "default_working_dir")]
    pub working_directory: String,
    #[serde(default)]
    pub gpu_count: u32,
    #[serde(default = "default_cpu_cores")]
    pub cpu_cores: u32,
    #[serde(default = "default_memory")]
    pub memory_mb: u64,
    #[serde(default = "default_priority")]
    pub priority: i32,
    #[serde(default)]
    pub gang_schedule: bool,
}

fn default_working_dir() -> String { "/app".to_string() }
fn default_cpu_cores() -> u32 { 1 }
fn default_memory() -> u64 { 4096 }
fn default_priority() -> i32 { 50 }

#[derive(Debug, Serialize)]
pub struct JobResponse {
    pub job_id: String,
    pub name: String,
    pub state: String,
    pub user_id: String,
    pub project_id: String,
    pub created_at: String,
    pub allocated_nodes: Vec<String>,
    pub gpu_count: u32,
}

#[derive(Debug, Serialize)]
pub struct ClusterStatusResponse {
    pub total_nodes: usize,
    pub healthy_nodes: usize,
    pub total_gpus: usize,
    pub available_gpus: usize,
    pub running_jobs: usize,
    pub queued_jobs: usize,
}

#[derive(Debug, Serialize)]
pub struct NodeResponse {
    pub id: String,
    pub hostname: String,
    pub health: String,
    pub total_gpus: usize,
    pub available_gpus: usize,
    pub running_jobs: usize,
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub message: String,
}

/// Response for successful operations
#[derive(Debug, Serialize)]
pub struct SuccessResponse {
    /// Status of the operation (e.g., "success", "error")
    pub status: String,
    /// Human-readable message describing the result
    pub message: String,
}

// === Handlers ===

async fn submit_job(
    State(state): State<Arc<AppState>>,
    Json(request): Json<SubmitJobRequest>,
) -> impl IntoResponse {
    let descriptor = JobDescriptor {
        name: request.name,
        user_id: request.user_id,
        project_id: request.project_id,
        command: request.command,
        arguments: request.arguments,
        environment: request.environment,
        working_directory: request.working_directory,
        resources: ResourceRequirements {
            gpu_count: request.gpu_count,
            cpu_cores: request.cpu_cores,
            cpu_memory: request.memory_mb * 1024 * 1024, // Convert MB to bytes
            ..Default::default()
        },
        locality: LocalityPreferences::default(),
        policy: SchedulingPolicy {
            priority: request.priority,
            gang_schedule: request.gang_schedule,
            ..Default::default()
        },
        labels: HashMap::new(),
        annotations: HashMap::new(),
    };
    
    let job = Job::new(descriptor);
    
    match state.scheduler.submit(job) {
        Ok(job_id) => {
            if let Some(job) = state.scheduler.get_job(&job_id) {
                (StatusCode::CREATED, Json(job_to_response(&job)))
            } else {
                (StatusCode::INTERNAL_SERVER_ERROR, Json(JobResponse {
                    job_id,
                    name: "unknown".to_string(),
                    state: "QUEUED".to_string(),
                    user_id: "unknown".to_string(),
                    project_id: "unknown".to_string(),
                    created_at: chrono::Utc::now().to_rfc3339(),
                    allocated_nodes: vec![],
                    gpu_count: 0,
                }))
            }
        }
        Err(e) => {
            (StatusCode::BAD_REQUEST, Json(JobResponse {
                job_id: "".to_string(),
                name: "error".to_string(),
                state: e.to_string(),
                user_id: "".to_string(),
                project_id: "".to_string(),
                created_at: "".to_string(),
                allocated_nodes: vec![],
                gpu_count: 0,
            }))
        }
    }
}

async fn get_job(
    State(state): State<Arc<AppState>>,
    Path(job_id): Path<String>,
) -> impl IntoResponse {
    match state.scheduler.get_job(&job_id) {
        Some(job) => (StatusCode::OK, Json(job_to_response(&job))),
        None => (StatusCode::NOT_FOUND, Json(JobResponse {
            job_id,
            name: "not_found".to_string(),
            state: "NOT_FOUND".to_string(),
            user_id: "".to_string(),
            project_id: "".to_string(),
            created_at: "".to_string(),
            allocated_nodes: vec![],
            gpu_count: 0,
        })),
    }
}

async fn list_jobs(
    State(_state): State<Arc<AppState>>,
) -> impl IntoResponse {
    // Get jobs in different states
    let jobs: Vec<JobResponse> = vec![];
    // In production: iterate all jobs and convert
    Json(jobs)
}

async fn cancel_job(
    State(state): State<Arc<AppState>>,
    Path(job_id): Path<String>,
) -> impl IntoResponse {
    match state.scheduler.cancel(&job_id, "User requested cancellation") {
        Ok(()) => (StatusCode::OK, Json(SuccessResponse {
            status: "success".to_string(),
            message: format!("Job {} cancelled", job_id),
        })),
        Err(e) => (StatusCode::BAD_REQUEST, Json(SuccessResponse {
            status: "error".to_string(),
            message: e.to_string(),
        })),
    }
}

async fn cluster_status(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let summary = state.node_registry.summary();
    
    Json(ClusterStatusResponse {
        total_nodes: summary.total_nodes,
        healthy_nodes: summary.healthy_nodes,
        total_gpus: summary.total_gpus,
        available_gpus: summary.available_gpus,
        running_jobs: summary.running_jobs,
        queued_jobs: state.scheduler.queue_size(),
    })
}

async fn list_nodes(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let nodes: Vec<NodeResponse> = state.node_registry.healthy_nodes()
        .iter()
        .map(|n| NodeResponse {
            id: n.id.clone(),
            hostname: n.hostname.clone(),
            health: format!("{:?}", n.health),
            total_gpus: n.total_gpus(),
            available_gpus: n.available_gpus(),
            running_jobs: n.running_jobs.len(),
        })
        .collect();
    
    Json(nodes)
}

async fn health_check() -> impl IntoResponse {
    (StatusCode::OK, "OK")
}

// === Helpers ===

fn job_to_response(job: &Job) -> JobResponse {
    JobResponse {
        job_id: job.id.to_string(),
        name: job.descriptor.name.clone(),
        state: format!("{:?}", job.state),
        user_id: job.descriptor.user_id.clone(),
        project_id: job.descriptor.project_id.clone(),
        created_at: job.submit_time.to_rfc3339(),
        allocated_nodes: job.allocated_nodes.clone(),
        gpu_count: job.descriptor.resources.gpu_count,
    }
}
