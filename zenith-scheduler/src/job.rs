//! Job definition and management

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;
use chrono::{DateTime, Utc};

/// Job state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derive(Default)]
pub enum JobState {
    /// Job is pending submission
    #[default]
    Pending,
    /// Job is queued waiting for resources
    Queued,
    /// Job has been scheduled to nodes
    Scheduled,
    /// Job is running
    Running,
    /// Job has been suspended (preempted)
    Suspended,
    /// Job completed successfully
    Completed,
    /// Job failed
    Failed,
    /// Job was cancelled
    Cancelled,
    /// Job timed out
    Timeout,
}


/// Resource requirements for a job
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceRequirements {
    /// Number of GPUs required
    pub gpu_count: u32,
    /// GPU memory per device in bytes
    pub gpu_memory_per_device: u64,
    /// Number of CPU cores required
    pub cpu_cores: u32,
    /// CPU memory in bytes
    pub cpu_memory: u64,
    /// Required GPU models (empty = any)
    pub required_gpu_models: Vec<String>,
    /// Minimum NVLink version (0 = not required)
    pub min_nvlink_version: u32,
    /// Require NVSwitch
    pub require_nvswitch: bool,
    /// Require RDMA
    pub require_rdma: bool,
}

impl Default for ResourceRequirements {
    fn default() -> Self {
        Self {
            gpu_count: 1,
            gpu_memory_per_device: 0,
            cpu_cores: 1,
            cpu_memory: 1024 * 1024 * 1024, // 1GB
            required_gpu_models: vec![],
            min_nvlink_version: 0,
            require_nvswitch: false,
            require_rdma: false,
        }
    }
}

/// Locality preferences
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LocalityPreferences {
    /// Prefer allocation on same node
    pub prefer_same_node: bool,
    /// Prefer allocation on same rack
    pub prefer_same_rack: bool,
    /// Prefer allocation on same NVSwitch domain
    pub prefer_same_nvswitch_domain: bool,
    /// Preferred node IDs
    pub preferred_nodes: Vec<String>,
    /// Excluded node IDs
    pub excluded_nodes: Vec<String>,
}

/// Scheduling policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulingPolicy {
    /// Job priority (higher = more urgent)
    pub priority: i32,
    /// Can this job be preempted?
    pub preemptible: bool,
    /// Can this job preempt others?
    pub can_preempt_others: bool,
    /// Maximum wait time in queue (seconds)
    pub max_wait_time_seconds: u64,
    /// Maximum runtime (seconds, 0 = unlimited)
    pub max_runtime_seconds: u64,
    /// Queue/partition name
    pub queue_name: String,
    /// Gang scheduling (all resources together)
    pub gang_schedule: bool,
    /// Maximum retry attempts
    pub max_retries: u32,
}

impl Default for SchedulingPolicy {
    fn default() -> Self {
        Self {
            priority: 0,
            preemptible: true,
            can_preempt_others: false,
            max_wait_time_seconds: 3600, // 1 hour
            max_runtime_seconds: 0,       // unlimited
            queue_name: "default".to_string(),
            gang_schedule: true,
            max_retries: 3,
        }
    }
}

/// Job descriptor - the core unit of work submission
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobDescriptor {
    /// User-provided job name
    pub name: String,
    /// User ID
    pub user_id: String,
    /// Project ID
    pub project_id: String,
    /// Command to execute
    pub command: String,
    /// Command arguments
    pub arguments: Vec<String>,
    /// Environment variables
    pub environment: HashMap<String, String>,
    /// Working directory
    pub working_directory: String,
    /// Resource requirements
    pub resources: ResourceRequirements,
    /// Locality preferences
    pub locality: LocalityPreferences,
    /// Scheduling policy
    pub policy: SchedulingPolicy,
    /// Labels for filtering
    pub labels: HashMap<String, String>,
    /// Annotations for metadata
    pub annotations: HashMap<String, String>,
}

/// A job instance with state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    /// Unique job ID
    pub id: Uuid,
    /// Job descriptor
    pub descriptor: JobDescriptor,
    /// Current state
    pub state: JobState,
    /// Submission time
    pub submit_time: DateTime<Utc>,
    /// Schedule time (when resources were allocated)
    pub schedule_time: Option<DateTime<Utc>>,
    /// Start time (when execution began)
    pub start_time: Option<DateTime<Utc>>,
    /// End time (when execution finished)
    pub end_time: Option<DateTime<Utc>>,
    /// Allocated node IDs
    pub allocated_nodes: Vec<String>,
    /// Allocated GPU device IDs per node
    pub allocated_gpus: HashMap<String, Vec<String>>,
    /// Retry count
    pub retry_count: u32,
    /// Last state change message
    pub message: String,
}

impl Job {
    /// Create a new job from a descriptor
    pub fn new(descriptor: JobDescriptor) -> Self {
        Self {
            id: Uuid::new_v4(),
            descriptor,
            state: JobState::Pending,
            submit_time: Utc::now(),
            schedule_time: None,
            start_time: None,
            end_time: None,
            allocated_nodes: vec![],
            allocated_gpus: HashMap::new(),
            retry_count: 0,
            message: String::new(),
        }
    }
    
    /// Transition to a new state
    pub fn transition(&mut self, new_state: JobState, message: &str) {
        self.state = new_state;
        self.message = message.to_string();
        
        match new_state {
            JobState::Scheduled => {
                self.schedule_time = Some(Utc::now());
            }
            JobState::Running => {
                self.start_time = Some(Utc::now());
            }
            JobState::Completed | JobState::Failed | JobState::Cancelled | JobState::Timeout => {
                self.end_time = Some(Utc::now());
            }
            _ => {}
        }
    }
    
    /// Get job runtime in seconds
    pub fn runtime_seconds(&self) -> Option<i64> {
        match (self.start_time, self.end_time) {
            (Some(start), Some(end)) => Some((end - start).num_seconds()),
            (Some(start), None) => Some((Utc::now() - start).num_seconds()),
            _ => None,
        }
    }
    
    /// Get queue wait time in seconds
    pub fn wait_time_seconds(&self) -> i64 {
        match self.schedule_time {
            Some(scheduled) => (scheduled - self.submit_time).num_seconds(),
            None => (Utc::now() - self.submit_time).num_seconds(),
        }
    }
    
    /// Check if job can be retried
    pub fn can_retry(&self) -> bool {
        self.retry_count < self.descriptor.policy.max_retries
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    fn create_test_descriptor() -> JobDescriptor {
        JobDescriptor {
            name: "test-job".to_string(),
            user_id: "user1".to_string(),
            project_id: "project1".to_string(),
            command: "python".to_string(),
            arguments: vec!["train.py".to_string()],
            environment: HashMap::new(),
            working_directory: "/app".to_string(),
            resources: ResourceRequirements::default(),
            locality: LocalityPreferences::default(),
            policy: SchedulingPolicy::default(),
            labels: HashMap::new(),
            annotations: HashMap::new(),
        }
    }
    
    #[test]
    fn test_job_creation() {
        let descriptor = create_test_descriptor();
        let job = Job::new(descriptor);
        
        assert_eq!(job.state, JobState::Pending);
        assert!(job.start_time.is_none());
    }
    
    #[test]
    fn test_job_transition() {
        let descriptor = create_test_descriptor();
        let mut job = Job::new(descriptor);
        
        job.transition(JobState::Queued, "Submitted to queue");
        assert_eq!(job.state, JobState::Queued);
        
        job.transition(JobState::Running, "Started");
        assert_eq!(job.state, JobState::Running);
        assert!(job.start_time.is_some());
    }
}
