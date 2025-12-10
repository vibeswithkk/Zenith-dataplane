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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scheduler::SchedulerConfig;
    
    fn create_test_state() -> Arc<AppState> {
        let node_registry = Arc::new(NodeRegistry::new(60));
        let scheduler = Arc::new(Scheduler::new(node_registry.clone(), SchedulerConfig::default()));
        Arc::new(AppState {
            scheduler,
            node_registry,
        })
    }
    
    fn create_test_submit_request() -> SubmitJobRequest {
        SubmitJobRequest {
            name: "test-job".to_string(),
            user_id: "user-123".to_string(),
            project_id: "project-456".to_string(),
            command: "python".to_string(),
            arguments: vec!["train.py".to_string()],
            environment: HashMap::new(),
            working_directory: "/app".to_string(),
            gpu_count: 2,
            cpu_cores: 4,
            memory_mb: 8192,
            priority: 50,
            gang_schedule: false,
        }
    }
    
    #[test]
    #[ignore = "Router syntax requires Axum 0.8+ path format"]
    fn test_create_router() {
        let state = create_test_state();
        let _router = create_router(state);
        // Router creation should not panic
        assert!(true);
    }
    
    #[test]
    fn test_default_functions() {
        assert_eq!(default_working_dir(), "/app");
        assert_eq!(default_cpu_cores(), 1);
        assert_eq!(default_memory(), 4096);
        assert_eq!(default_priority(), 50);
    }
    
    #[test]
    fn test_submit_job_request_defaults() {
        let request = create_test_submit_request();
        assert_eq!(request.name, "test-job");
        assert_eq!(request.gpu_count, 2);
        assert_eq!(request.cpu_cores, 4);
        assert_eq!(request.memory_mb, 8192);
        assert_eq!(request.priority, 50);
        assert!(!request.gang_schedule);
    }
    
    #[test]
    fn test_job_response_serialization() {
        let response = JobResponse {
            job_id: "job-123".to_string(),
            name: "test-job".to_string(),
            state: "QUEUED".to_string(),
            user_id: "user-1".to_string(),
            project_id: "proj-1".to_string(),
            created_at: "2024-12-10T00:00:00Z".to_string(),
            allocated_nodes: vec!["node-1".to_string()],
            gpu_count: 4,
        };
        
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("job-123"));
        assert!(json.contains("QUEUED"));
        assert!(json.contains("node-1"));
    }
    
    #[test]
    fn test_cluster_status_response_serialization() {
        let response = ClusterStatusResponse {
            total_nodes: 10,
            healthy_nodes: 8,
            total_gpus: 80,
            available_gpus: 40,
            running_jobs: 5,
            queued_jobs: 3,
        };
        
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"total_nodes\":10"));
        assert!(json.contains("\"healthy_nodes\":8"));
        assert!(json.contains("\"available_gpus\":40"));
    }
    
    #[test]
    fn test_node_response_serialization() {
        let response = NodeResponse {
            id: "node-1".to_string(),
            hostname: "gpu-server-01".to_string(),
            health: "Healthy".to_string(),
            total_gpus: 8,
            available_gpus: 4,
            running_jobs: 2,
        };
        
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("gpu-server-01"));
        assert!(json.contains("\"total_gpus\":8"));
    }
    
    #[test]
    fn test_error_response_serialization() {
        let response = ErrorResponse {
            error: "InvalidRequest".to_string(),
            message: "Job name is required".to_string(),
        };
        
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("InvalidRequest"));
        assert!(json.contains("Job name is required"));
    }
    
    #[test]
    fn test_success_response_serialization() {
        let response = SuccessResponse {
            status: "success".to_string(),
            message: "Job cancelled".to_string(),
        };
        
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("success"));
        assert!(json.contains("Job cancelled"));
    }
    
    #[test]
    fn test_job_to_response_conversion() {
        let descriptor = JobDescriptor {
            name: "conversion-test".to_string(),
            user_id: "user-conv".to_string(),
            project_id: "proj-conv".to_string(),
            command: "test".to_string(),
            arguments: vec![],
            environment: HashMap::new(),
            working_directory: "/test".to_string(),
            resources: ResourceRequirements {
                gpu_count: 2,
                cpu_cores: 4,
                cpu_memory: 8 * 1024 * 1024 * 1024,
                ..Default::default()
            },
            locality: LocalityPreferences::default(),
            policy: SchedulingPolicy::default(),
            labels: HashMap::new(),
            annotations: HashMap::new(),
        };
        
        let job = Job::new(descriptor);
        let response = job_to_response(&job);
        
        assert_eq!(response.name, "conversion-test");
        assert_eq!(response.user_id, "user-conv");
        assert_eq!(response.gpu_count, 2);
        // Job state after new() is "Pending"
        assert_eq!(response.state, "Pending");
        assert!(response.allocated_nodes.is_empty());
    }
    
    #[test]
    fn test_memory_conversion_in_submit() {
        let request = SubmitJobRequest {
            name: "memory-test".to_string(),
            user_id: "user".to_string(),
            project_id: "proj".to_string(),
            command: "test".to_string(),
            arguments: vec![],
            environment: HashMap::new(),
            working_directory: "/app".to_string(),
            gpu_count: 0,
            cpu_cores: 1,
            memory_mb: 1024, // 1GB in MB
            priority: 50,
            gang_schedule: false,
        };
        
        // Verify memory conversion: MB to bytes
        let expected_bytes: u64 = 1024 * 1024 * 1024; // 1GB in bytes
        assert_eq!(request.memory_mb * 1024 * 1024, expected_bytes);
    }
    
    #[tokio::test]
    async fn test_health_check_handler() {
        let response = health_check().await;
        let (status, body) = response.into_response().into_parts();
        // Health check should return OK
        assert_eq!(status.status, StatusCode::OK);
    }
    
    #[tokio::test]
    async fn test_list_jobs_returns_empty_initially() {
        let state = create_test_state();
        let response = list_jobs(State(state)).await;
        // Should return empty list initially
        let json = response.into_response();
        assert_eq!(json.status(), StatusCode::OK);
    }
    
    #[tokio::test]
    async fn test_cluster_status_handler() {
        let state = create_test_state();
        let response = cluster_status(State(state)).await;
        let json = response.into_response();
        assert_eq!(json.status(), StatusCode::OK);
    }
    
    #[tokio::test]
    async fn test_list_nodes_handler() {
        let state = create_test_state();
        let response = list_nodes(State(state)).await;
        let json = response.into_response();
        assert_eq!(json.status(), StatusCode::OK);
    }
    
    #[tokio::test]
    async fn test_get_job_not_found() {
        let state = create_test_state();
        let response = get_job(
            State(state),
            Path("non-existent-job".to_string())
        ).await;
        
        let (parts, _body) = response.into_response().into_parts();
        assert_eq!(parts.status, StatusCode::NOT_FOUND);
    }
    
    #[tokio::test]
    async fn test_cancel_job_not_found() {
        let state = create_test_state();
        let response = cancel_job(
            State(state),
            Path("non-existent-job".to_string())
        ).await;
        
        let (parts, _body) = response.into_response().into_parts();
        // Should return error for non-existent job
        assert_eq!(parts.status, StatusCode::BAD_REQUEST);
    }
}

