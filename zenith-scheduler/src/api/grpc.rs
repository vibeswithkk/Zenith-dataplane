//! gRPC Service Implementation for Scheduler

use tonic::Status;
use std::sync::Arc;
use crate::scheduler::Scheduler;
use crate::node::NodeRegistry;
use crate::job::{Job, JobDescriptor, ResourceRequirements, LocalityPreferences, SchedulingPolicy};
use std::collections::HashMap;

/// Job submission request
#[derive(Debug, Clone)]
pub struct SubmitJobRequest {
    pub name: String,
    pub user_id: String,
    pub project_id: String,
    pub command: String,
    pub arguments: Vec<String>,
    pub environment: HashMap<String, String>,
    pub working_directory: String,
    pub gpu_count: u32,
    pub cpu_cores: u32,
    pub memory_mb: u64,
    pub priority: i32,
    pub gang_schedule: bool,
}

/// Job submission response
#[derive(Debug, Clone)]
pub struct SubmitJobResponse {
    pub job_id: String,
    pub status: String,
}

/// Job status request
#[derive(Debug, Clone)]
pub struct GetJobStatusRequest {
    pub job_id: String,
}

/// Job status response
#[derive(Debug, Clone)]
pub struct GetJobStatusResponse {
    pub job_id: String,
    pub state: String,
    pub message: String,
    pub allocated_nodes: Vec<String>,
}

/// Cancel job request
#[derive(Debug, Clone)]
pub struct CancelJobRequest {
    pub job_id: String,
    pub reason: String,
}

/// Cancel job response
#[derive(Debug, Clone)]
pub struct CancelJobResponse {
    pub success: bool,
    pub message: String,
}

/// Cluster status response
#[derive(Debug, Clone)]
pub struct ClusterStatusResponse {
    pub total_nodes: usize,
    pub healthy_nodes: usize,
    pub total_gpus: usize,
    pub available_gpus: usize,
    pub running_jobs: usize,
    pub queued_jobs: usize,
}

/// Scheduler gRPC service
pub struct SchedulerService {
    scheduler: Arc<Scheduler>,
    node_registry: Arc<NodeRegistry>,
}

impl SchedulerService {
    /// Create a new scheduler service
    pub fn new(scheduler: Arc<Scheduler>, node_registry: Arc<NodeRegistry>) -> Self {
        Self {
            scheduler,
            node_registry,
        }
    }
    
    /// Submit a job
    #[allow(clippy::result_large_err)]
    pub fn submit_job(&self, request: SubmitJobRequest) -> Result<SubmitJobResponse, Status> {
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
        
        match self.scheduler.submit(job) {
            Ok(job_id) => Ok(SubmitJobResponse {
                job_id,
                status: "QUEUED".to_string(),
            }),
            Err(e) => Err(Status::internal(e.to_string())),
        }
    }
    
    /// Get job status
    #[allow(clippy::result_large_err)]
    pub fn get_job_status(&self, request: GetJobStatusRequest) -> Result<GetJobStatusResponse, Status> {
        match self.scheduler.get_job(&request.job_id) {
            Some(job) => Ok(GetJobStatusResponse {
                job_id: job.id.to_string(),
                state: format!("{:?}", job.state),
                message: job.message.clone(),
                allocated_nodes: job.allocated_nodes,
            }),
            None => Err(Status::not_found(format!("Job not found: {}", request.job_id))),
        }
    }
    
    /// Cancel a job
    #[allow(clippy::result_large_err)]
    pub fn cancel_job(&self, request: CancelJobRequest) -> Result<CancelJobResponse, Status> {
        match self.scheduler.cancel(&request.job_id, &request.reason) {
            Ok(()) => Ok(CancelJobResponse {
                success: true,
                message: "Job cancelled".to_string(),
            }),
            Err(e) => Err(Status::internal(e.to_string())),
        }
    }
    
    /// Get cluster status
    pub fn get_cluster_status(&self) -> ClusterStatusResponse {
        let summary = self.node_registry.summary();
        
        ClusterStatusResponse {
            total_nodes: summary.total_nodes,
            healthy_nodes: summary.healthy_nodes,
            total_gpus: summary.total_gpus,
            available_gpus: summary.available_gpus,
            running_jobs: summary.running_jobs,
            queued_jobs: self.scheduler.queue_size(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scheduler::SchedulerConfig;
    
    // Helper to create test state
    fn create_test_service() -> SchedulerService {
        let node_registry = Arc::new(NodeRegistry::new(60)); // 60s heartbeat timeout
        let scheduler = Arc::new(Scheduler::new(node_registry.clone(), SchedulerConfig::default()));
        SchedulerService::new(scheduler, node_registry)
    }
    
    fn create_test_request() -> SubmitJobRequest {
        SubmitJobRequest {
            name: "test-job".to_string(),
            user_id: "user1".to_string(),
            project_id: "project1".to_string(),
            command: "python".to_string(),
            arguments: vec!["train.py".to_string()],
            environment: HashMap::new(),
            working_directory: "/app".to_string(),
            gpu_count: 4,
            cpu_cores: 8,
            memory_mb: 16384,
            priority: 50,
            gang_schedule: true,
        }
    }
    
    // ===================== Struct Tests =====================
    
    #[test]
    fn test_submit_request_creation() {
        let request = create_test_request();
        assert_eq!(request.name, "test-job");
        assert_eq!(request.user_id, "user1");
        assert_eq!(request.project_id, "project1");
        assert_eq!(request.command, "python");
        assert_eq!(request.arguments.len(), 1);
        assert_eq!(request.gpu_count, 4);
        assert_eq!(request.cpu_cores, 8);
        assert_eq!(request.memory_mb, 16384);
        assert_eq!(request.priority, 50);
        assert!(request.gang_schedule);
    }
    
    #[test]
    fn test_submit_request_with_environment() {
        let mut env = HashMap::new();
        env.insert("CUDA_VISIBLE_DEVICES".to_string(), "0,1".to_string());
        
        let request = SubmitJobRequest {
            name: "env-job".to_string(),
            user_id: "user1".to_string(),
            project_id: "project1".to_string(),
            command: "python".to_string(),
            arguments: vec![],
            environment: env.clone(),
            working_directory: "/app".to_string(),
            gpu_count: 2,
            cpu_cores: 4,
            memory_mb: 8192,
            priority: 100,
            gang_schedule: false,
        };
        
        assert_eq!(request.environment.get("CUDA_VISIBLE_DEVICES"), Some(&"0,1".to_string()));
    }
    
    #[test]
    fn test_submit_response_creation() {
        let response = SubmitJobResponse {
            job_id: "job-123".to_string(),
            status: "QUEUED".to_string(),
        };
        assert_eq!(response.job_id, "job-123");
        assert_eq!(response.status, "QUEUED");
    }
    
    #[test]
    fn test_get_job_status_request() {
        let request = GetJobStatusRequest {
            job_id: "job-456".to_string(),
        };
        assert_eq!(request.job_id, "job-456");
    }
    
    #[test]
    fn test_get_job_status_response() {
        let response = GetJobStatusResponse {
            job_id: "job-789".to_string(),
            state: "Running".to_string(),
            message: "Job is running".to_string(),
            allocated_nodes: vec!["node1".to_string(), "node2".to_string()],
        };
        assert_eq!(response.job_id, "job-789");
        assert_eq!(response.state, "Running");
        assert_eq!(response.allocated_nodes.len(), 2);
    }
    
    #[test]
    fn test_cancel_job_request() {
        let request = CancelJobRequest {
            job_id: "job-to-cancel".to_string(),
            reason: "User requested".to_string(),
        };
        assert_eq!(request.job_id, "job-to-cancel");
        assert_eq!(request.reason, "User requested");
    }
    
    #[test]
    fn test_cancel_job_response_success() {
        let response = CancelJobResponse {
            success: true,
            message: "Job cancelled".to_string(),
        };
        assert!(response.success);
        assert_eq!(response.message, "Job cancelled");
    }
    
    #[test]
    fn test_cancel_job_response_failure() {
        let response = CancelJobResponse {
            success: false,
            message: "Job not found".to_string(),
        };
        assert!(!response.success);
    }
    
    #[test]
    fn test_cluster_status_response() {
        let response = ClusterStatusResponse {
            total_nodes: 10,
            healthy_nodes: 8,
            total_gpus: 80,
            available_gpus: 40,
            running_jobs: 5,
            queued_jobs: 10,
        };
        assert_eq!(response.total_nodes, 10);
        assert_eq!(response.healthy_nodes, 8);
        assert_eq!(response.total_gpus, 80);
        assert_eq!(response.available_gpus, 40);
        assert_eq!(response.running_jobs, 5);
        assert_eq!(response.queued_jobs, 10);
    }
    
    // ===================== Service Tests =====================
    
    #[test]
    fn test_scheduler_service_creation() {
        let service = create_test_service();
        // Service should be created without panic
        let status = service.get_cluster_status();
        assert_eq!(status.total_nodes, 0);
        assert_eq!(status.queued_jobs, 0);
    }
    
    #[test]
    fn test_submit_job_success() {
        let service = create_test_service();
        let request = create_test_request();
        
        let result = service.submit_job(request);
        assert!(result.is_ok());
        
        let response = result.unwrap();
        assert!(!response.job_id.is_empty());
        assert_eq!(response.status, "QUEUED");
    }
    
    #[test]
    fn test_submit_multiple_jobs() {
        let service = create_test_service();
        
        for i in 0..5 {
            let mut request = create_test_request();
            request.name = format!("job-{}", i);
            
            let result = service.submit_job(request);
            assert!(result.is_ok());
        }
        
        let status = service.get_cluster_status();
        assert_eq!(status.queued_jobs, 5);
    }
    
    #[test]
    fn test_get_job_status_not_found() {
        let service = create_test_service();
        let request = GetJobStatusRequest {
            job_id: "non-existent-job".to_string(),
        };
        
        let result = service.get_job_status(request);
        assert!(result.is_err());
    }
    
    #[test]
    fn test_get_job_status_after_submit() {
        let service = create_test_service();
        
        // Submit a job first
        let submit_request = create_test_request();
        let submit_result = service.submit_job(submit_request);
        assert!(submit_result.is_ok());
        let job_id = submit_result.unwrap().job_id;
        
        // Now get its status
        let status_request = GetJobStatusRequest {
            job_id: job_id.clone(),
        };
        let status_result = service.get_job_status(status_request);
        assert!(status_result.is_ok());
        
        let status = status_result.unwrap();
        assert_eq!(status.job_id, job_id);
    }
    
    #[test]
    fn test_cancel_job_not_found() {
        let service = create_test_service();
        let request = CancelJobRequest {
            job_id: "non-existent-job".to_string(),
            reason: "Test".to_string(),
        };
        
        let result = service.cancel_job(request);
        assert!(result.is_err());
    }
    
    #[test]
    fn test_get_cluster_status_empty() {
        let service = create_test_service();
        let status = service.get_cluster_status();
        
        assert_eq!(status.total_nodes, 0);
        assert_eq!(status.healthy_nodes, 0);
        assert_eq!(status.total_gpus, 0);
        assert_eq!(status.available_gpus, 0);
        assert_eq!(status.running_jobs, 0);
        assert_eq!(status.queued_jobs, 0);
    }
    
    // ===================== Clone Tests =====================
    
    #[test]
    fn test_submit_request_clone() {
        let request = create_test_request();
        let cloned = request.clone();
        
        assert_eq!(request.name, cloned.name);
        assert_eq!(request.gpu_count, cloned.gpu_count);
    }
    
    #[test]
    fn test_response_structs_clone() {
        let submit_resp = SubmitJobResponse {
            job_id: "job-1".to_string(),
            status: "OK".to_string(),
        };
        let cloned = submit_resp.clone();
        assert_eq!(submit_resp.job_id, cloned.job_id);
        
        let status_resp = GetJobStatusResponse {
            job_id: "job-1".to_string(),
            state: "Running".to_string(),
            message: "OK".to_string(),
            allocated_nodes: vec![],
        };
        let cloned = status_resp.clone();
        assert_eq!(status_resp.state, cloned.state);
        
        let cancel_resp = CancelJobResponse {
            success: true,
            message: "Done".to_string(),
        };
        let cloned = cancel_resp.clone();
        assert_eq!(cancel_resp.success, cloned.success);
        
        let cluster_resp = ClusterStatusResponse {
            total_nodes: 1,
            healthy_nodes: 1,
            total_gpus: 4,
            available_gpus: 2,
            running_jobs: 1,
            queued_jobs: 0,
        };
        let cloned = cluster_resp.clone();
        assert_eq!(cluster_resp.total_gpus, cloned.total_gpus);
    }
    
    // ===================== Debug Tests =====================
    
    #[test]
    fn test_debug_implementations() {
        let request = create_test_request();
        let debug_str = format!("{:?}", request);
        assert!(debug_str.contains("SubmitJobRequest"));
        assert!(debug_str.contains("test-job"));
        
        let response = SubmitJobResponse {
            job_id: "j1".to_string(),
            status: "OK".to_string(),
        };
        let debug_str = format!("{:?}", response);
        assert!(debug_str.contains("SubmitJobResponse"));
    }
}
