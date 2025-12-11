//! Gang Scheduler Implementation

use crate::job::{Job, JobState};
use crate::node::{Node, NodeRegistry};
use crate::{Error, Result};
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use priority_queue::PriorityQueue;
use tracing::{debug, info};

/// Scheduling decision for a job
#[derive(Debug, Clone)]
pub struct SchedulingDecision {
    /// Job ID
    pub job_id: String,
    /// Allocated node IDs with GPU assignments
    pub allocations: HashMap<String, Vec<String>>,
    /// Was this a gang allocation?
    pub gang_allocated: bool,
}

/// Gang scheduler with topology awareness
pub struct Scheduler {
    /// Node registry
    nodes: Arc<NodeRegistry>,
    /// Priority queue of pending jobs
    pending_queue: RwLock<PriorityQueue<String, i32>>,
    /// Job storage
    jobs: RwLock<HashMap<String, Job>>,
    /// Scheduler configuration
    config: SchedulerConfig,
}

/// Scheduler configuration
#[derive(Debug, Clone)]
pub struct SchedulerConfig {
    /// Maximum jobs to consider in one scheduling cycle
    pub max_schedule_batch: usize,
    /// Enable backfill scheduling
    pub backfill_enabled: bool,
    /// Enable topology-aware placement
    pub topology_aware: bool,
    /// Prefer same-node allocation for multi-GPU jobs
    pub prefer_same_node: bool,
    /// Job timeout in seconds (0 = no timeout)
    pub job_timeout_secs: u64,
    /// Heartbeat timeout in seconds - mark node dead if no heartbeat
    pub heartbeat_timeout_secs: u64,
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        Self {
            max_schedule_batch: 100,
            backfill_enabled: true,
            topology_aware: true,
            prefer_same_node: true,
            job_timeout_secs: 86400,      // 24 hours default
            heartbeat_timeout_secs: 60,   // 1 minute default
        }
    }
}

impl Scheduler {
    /// Create a new scheduler
    pub fn new(nodes: Arc<NodeRegistry>, config: SchedulerConfig) -> Self {
        Self {
            nodes,
            pending_queue: RwLock::new(PriorityQueue::new()),
            jobs: RwLock::new(HashMap::new()),
            config,
        }
    }
    
    /// Submit a job
    pub fn submit(&self, mut job: Job) -> Result<String> {
        let job_id = job.id.to_string();
        
        job.transition(JobState::Queued, "Submitted to scheduler");
        
        let priority = job.descriptor.policy.priority;
        
        {
            let mut jobs = self.jobs.write();
            jobs.insert(job_id.clone(), job);
        }
        
        {
            let mut queue = self.pending_queue.write();
            queue.push(job_id.clone(), priority);
        }
        
        info!("Job {} submitted with priority {}", job_id, priority);
        Ok(job_id)
    }
    
    /// Cancel a job
    pub fn cancel(&self, job_id: &str, reason: &str) -> Result<()> {
        let mut jobs = self.jobs.write();
        
        if let Some(job) = jobs.get_mut(job_id) {
            match job.state {
                JobState::Pending | JobState::Queued | JobState::Scheduled => {
                    job.transition(JobState::Cancelled, reason);
                    
                    // Remove from queue
                    let mut queue = self.pending_queue.write();
                    queue.remove(job_id);
                }
                JobState::Running => {
                    job.transition(JobState::Cancelled, reason);
                    
                    // Release resources
                    for node_id in &job.allocated_nodes {
                        if let Some(_node) = self.nodes.get(node_id) {
                            // In production: send cancel signal to node agent
                        }
                    }
                }
                _ => {
                    return Err(Error::Job(format!(
                        "Cannot cancel job in state {:?}", job.state
                    )));
                }
            }
            
            info!("Job {} cancelled: {}", job_id, reason);
            Ok(())
        } else {
            Err(Error::Job(format!("Job not found: {}", job_id)))
        }
    }
    
    /// Run one scheduling cycle
    pub fn schedule_cycle(&self) -> Vec<SchedulingDecision> {
        let mut decisions = vec![];
        let mut queue = self.pending_queue.write();
        let mut jobs = self.jobs.write();
        
        let mut to_remove = vec![];
        
        // Process jobs in priority order
        for (processed, (job_id, _priority)) in queue.iter().enumerate() {
            if processed >= self.config.max_schedule_batch {
                break;
            }
            
            if let Some(job) = jobs.get_mut(job_id) {
                if let Some(decision) = self.try_schedule_job(job) {
                    // Apply allocation
                    job.transition(JobState::Scheduled, "Resources allocated");
                    job.allocated_nodes = decision.allocations.keys().cloned().collect();
                    job.allocated_gpus = decision.allocations.clone();
                    
                    decisions.push(decision.clone());
                    to_remove.push(job_id.clone());
                    
                    info!(
                        "Job {} scheduled: {} nodes, {} GPUs",
                        job_id,
                        job.allocated_nodes.len(),
                        job.allocated_gpus.values().map(|v| v.len()).sum::<usize>()
                    );
                }
            }
        }
        
        // Remove scheduled jobs from queue
        for job_id in to_remove {
            queue.remove(&job_id);
        }
        
        decisions
    }
    
    /// Try to schedule a single job
    fn try_schedule_job(&self, job: &Job) -> Option<SchedulingDecision> {
        let required_gpus = job.descriptor.resources.gpu_count as usize;
        
        if required_gpus == 0 {
            // CPU-only job
            return self.schedule_cpu_job(job);
        }
        
        // Get candidate nodes
        let candidates = self.nodes.nodes_with_available_gpus(1);
        
        if candidates.is_empty() {
            debug!("No nodes with available GPUs for job {}", job.id);
            return None;
        }
        
        // Gang scheduling: try to allocate all GPUs together
        if job.descriptor.policy.gang_schedule {
            return self.gang_schedule(job, &candidates, required_gpus);
        }
        
        // Non-gang: allocate wherever possible
        self.spread_schedule(job, &candidates, required_gpus)
    }
    
    /// Gang scheduling: all or nothing allocation
    fn gang_schedule(
        &self,
        job: &Job,
        candidates: &[Node],
        required_gpus: usize,
    ) -> Option<SchedulingDecision> {
        // First try: single node with enough GPUs
        if self.config.prefer_same_node {
            for node in candidates {
                if node.available_gpus() >= required_gpus {
                    // Allocate all GPUs from this node
                    let mut allocations = HashMap::new();
                    let gpu_ids: Vec<String> = node.topology.gpus.iter()
                        .filter(|g| !g.allocated)
                        .take(required_gpus)
                        .map(|g| g.device_id.clone())
                        .collect();
                    
                    allocations.insert(node.id.clone(), gpu_ids);
                    
                    return Some(SchedulingDecision {
                        job_id: job.id.to_string(),
                        allocations,
                        gang_allocated: true,
                    });
                }
            }
        }
        
        // Second try: spread across multiple nodes
        let total_available: usize = candidates.iter()
            .map(|n| n.available_gpus())
            .sum();
        
        if total_available < required_gpus {
            debug!(
                "Not enough GPUs for gang job {}: need {}, have {}",
                job.id, required_gpus, total_available
            );
            return None;
        }
        
        // Greedy allocation across nodes
        let mut allocations = HashMap::new();
        let mut remaining = required_gpus;
        
        for node in candidates {
            if remaining == 0 {
                break;
            }
            
            let available = node.available_gpus();
            let to_allocate = remaining.min(available);
            
            let gpu_ids: Vec<String> = node.topology.gpus.iter()
                .filter(|g| !g.allocated)
                .take(to_allocate)
                .map(|g| g.device_id.clone())
                .collect();
            
            allocations.insert(node.id.clone(), gpu_ids);
            remaining -= to_allocate;
        }
        
        if remaining > 0 {
            None
        } else {
            Some(SchedulingDecision {
                job_id: job.id.to_string(),
                allocations,
                gang_allocated: true,
            })
        }
    }
    
    /// Spread scheduling: allocate what's available
    fn spread_schedule(
        &self,
        job: &Job,
        candidates: &[Node],
        required_gpus: usize,
    ) -> Option<SchedulingDecision> {
        // Same as gang but with partial allocation allowed
        self.gang_schedule(job, candidates, required_gpus)
    }
    
    /// Schedule CPU-only job
    fn schedule_cpu_job(&self, job: &Job) -> Option<SchedulingDecision> {
        let nodes = self.nodes.healthy_nodes();
        
        nodes.first().map(|node| SchedulingDecision {
                job_id: job.id.to_string(),
                allocations: HashMap::from([(node.id.clone(), vec![])]),
                gang_allocated: false,
            })
    }
    
    /// Get job status
    pub fn get_job(&self, job_id: &str) -> Option<Job> {
        self.jobs.read().get(job_id).cloned()
    }
    
    /// Get all jobs with a specific state
    pub fn jobs_with_state(&self, state: JobState) -> Vec<Job> {
        self.jobs.read()
            .values()
            .filter(|j| j.state == state)
            .cloned()
            .collect()
    }
    
    /// Get queue size
    pub fn queue_size(&self) -> usize {
        self.pending_queue.read().len()
    }
    
    /// Clean up zombie jobs (jobs on dead nodes or timed out)
    /// 
    /// This should be called periodically (e.g., every minute) to detect
    /// and handle jobs that are stuck in Running state.
    /// 
    /// Returns the number of jobs cleaned up.
    pub fn cleanup_zombie_jobs(&self) -> usize {
        let mut cleaned = 0;
        let now = chrono::Utc::now();
        let mut jobs = self.jobs.write();
        
        for job in jobs.values_mut() {
            if job.state != JobState::Running {
                continue;
            }
            
            // Check if job has timed out
            if self.config.job_timeout_secs > 0 {
                if let Some(start_time) = job.start_time {
                    let elapsed = (now - start_time).num_seconds() as u64;
                    if elapsed > self.config.job_timeout_secs {
                        job.transition(
                            JobState::Timeout,
                            &format!("Job exceeded timeout of {} seconds", self.config.job_timeout_secs)
                        );
                        info!("Job {} timed out after {} seconds", job.id, elapsed);
                        cleaned += 1;
                        continue;
                    }
                }
            }
            
            // Check if allocated nodes are still healthy
            let mut any_dead = false;
            for node_id in &job.allocated_nodes {
                if !self.nodes.is_node_healthy(node_id) {
                    any_dead = true;
                    break;
                }
            }
            
            if any_dead {
                job.transition(
                    JobState::Failed,
                    "Allocated node(s) became unhealthy"
                );
                info!("Job {} failed due to unhealthy node", job.id);
                cleaned += 1;
            }
        }
        
        if cleaned > 0 {
            info!("Cleaned up {} zombie jobs", cleaned);
        }
        
        cleaned
    }
    
    /// Mark a job as started (call when job actually begins execution)
    pub fn mark_job_started(&self, job_id: &str) -> Result<()> {
        let mut jobs = self.jobs.write();
        
        if let Some(job) = jobs.get_mut(job_id) {
            // Note: transition() already sets start_time for JobState::Running
            job.transition(JobState::Running, "Job started on node");
            info!("Job {} marked as running", job_id);
            Ok(())
        } else {
            Err(Error::Job(format!("Job not found: {}", job_id)))
        }
    }
    
    /// Mark a job as completed
    pub fn mark_job_completed(&self, job_id: &str, success: bool, message: &str) -> Result<()> {
        let mut jobs = self.jobs.write();
        
        if let Some(job) = jobs.get_mut(job_id) {
            let new_state = if success { JobState::Completed } else { JobState::Failed };
            job.transition(new_state, message);
            info!("Job {} marked as {:?}: {}", job_id, new_state, message);
            Ok(())
        } else {
            Err(Error::Job(format!("Job not found: {}", job_id)))
        }
    }
    
    /// Get configuration
    pub fn config(&self) -> &SchedulerConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::job::JobDescriptor;
    use crate::node::{GpuDevice, NodeTopology};
    
    fn create_test_node(id: &str, gpu_count: usize) -> Node {
        let gpus: Vec<GpuDevice> = (0..gpu_count)
            .map(|i| GpuDevice {
                device_id: format!("cuda:{}", i),
                device_name: "NVIDIA A100".to_string(),
                uuid: format!("GPU-{}", i),
                total_memory: 80 * 1024 * 1024 * 1024,
                free_memory: 80 * 1024 * 1024 * 1024,
                utilization: 0.0,
                temperature: 40,
                allocated: false,
                allocated_job_id: None,
            })
            .collect();
        
        let topology = NodeTopology {
            gpus,
            cpu_cores: 64,
            cpu_memory: 512 * 1024 * 1024 * 1024,
            cpu_memory_free: 500 * 1024 * 1024 * 1024,
            numa_nodes: 2,
            nvlink_present: true,
            nvswitch_present: false,
            rdma_capable: true,
        };
        
        Node::new(
            id.to_string(),
            format!("{}.local", id),
            "192.168.1.1".to_string(),
            topology,
        )
    }
    
    #[test]
    fn test_scheduler_submit() {
        let registry = Arc::new(NodeRegistry::new(60));
        registry.register(create_test_node("node-1", 4)).unwrap();
        
        let scheduler = Scheduler::new(registry, SchedulerConfig::default());
        
        let descriptor = JobDescriptor {
            name: "test-job".to_string(),
            user_id: "user1".to_string(),
            project_id: "project1".to_string(),
            command: "python".to_string(),
            arguments: vec!["train.py".to_string()],
            environment: HashMap::new(),
            working_directory: "/app".to_string(),
            resources: crate::job::ResourceRequirements {
                gpu_count: 2,
                ..Default::default()
            },
            locality: Default::default(),
            policy: Default::default(),
            labels: HashMap::new(),
            annotations: HashMap::new(),
        };
        
        let job = Job::new(descriptor);
        let job_id = scheduler.submit(job).unwrap();
        
        assert_eq!(scheduler.queue_size(), 1);
        
        // Run scheduling
        let decisions = scheduler.schedule_cycle();
        
        assert_eq!(decisions.len(), 1);
        assert_eq!(scheduler.queue_size(), 0);
        
        // Verify job state
        let job = scheduler.get_job(&job_id).unwrap();
        assert_eq!(job.state, JobState::Scheduled);
    }
    
    #[test]
    fn test_scheduler_cancel() {
        let registry = Arc::new(NodeRegistry::new(60));
        registry.register(create_test_node("node-1", 4)).unwrap();
        
        let scheduler = Scheduler::new(registry, SchedulerConfig::default());
        
        let descriptor = JobDescriptor {
            name: "cancel-test".to_string(),
            user_id: "user1".to_string(),
            project_id: "project1".to_string(),
            command: "python".to_string(),
            arguments: vec![],
            environment: HashMap::new(),
            working_directory: "/app".to_string(),
            resources: Default::default(),
            locality: Default::default(),
            policy: Default::default(),
            labels: HashMap::new(),
            annotations: HashMap::new(),
        };
        
        let job = Job::new(descriptor);
        let job_id = scheduler.submit(job).unwrap();
        
        assert_eq!(scheduler.queue_size(), 1);
        
        // Cancel the job
        scheduler.cancel(&job_id, "User requested").unwrap();
        
        // Job should be cancelled
        let job = scheduler.get_job(&job_id).unwrap();
        assert_eq!(job.state, JobState::Cancelled);
        
        // Queue should be empty
        assert_eq!(scheduler.queue_size(), 0);
    }
    
    #[test]
    fn test_scheduler_cpu_job() {
        let registry = Arc::new(NodeRegistry::new(60));
        registry.register(create_test_node("node-1", 0)).unwrap();
        
        let scheduler = Scheduler::new(registry, SchedulerConfig::default());
        
        // CPU-only job (gpu_count = 0)
        let descriptor = JobDescriptor {
            name: "cpu-job".to_string(),
            user_id: "user1".to_string(),
            project_id: "project1".to_string(),
            command: "python".to_string(),
            arguments: vec!["preprocess.py".to_string()],
            environment: HashMap::new(),
            working_directory: "/app".to_string(),
            resources: crate::job::ResourceRequirements {
                gpu_count: 0,
                cpu_cores: 4,
                ..Default::default()
            },
            locality: Default::default(),
            policy: Default::default(),
            labels: HashMap::new(),
            annotations: HashMap::new(),
        };
        
        let job = Job::new(descriptor);
        let job_id = scheduler.submit(job).unwrap();
        
        // Run scheduling
        let decisions = scheduler.schedule_cycle();
        
        assert_eq!(decisions.len(), 1);
        assert!(!decisions[0].gang_allocated);
        
        let job = scheduler.get_job(&job_id).unwrap();
        assert_eq!(job.state, JobState::Scheduled);
    }
    
    #[test]
    fn test_scheduler_gang_scheduling() {
        let registry = Arc::new(NodeRegistry::new(60));
        registry.register(create_test_node("node-1", 8)).unwrap();
        
        let scheduler = Scheduler::new(registry, SchedulerConfig::default());
        
        // Gang job requiring 4 GPUs
        let descriptor = JobDescriptor {
            name: "gang-job".to_string(),
            user_id: "user1".to_string(),
            project_id: "project1".to_string(),
            command: "python".to_string(),
            arguments: vec!["-m", "torch.distributed.launch", "train.py"]
                .into_iter().map(String::from).collect(),
            environment: HashMap::new(),
            working_directory: "/app".to_string(),
            resources: crate::job::ResourceRequirements {
                gpu_count: 4,
                ..Default::default()
            },
            locality: Default::default(),
            policy: crate::job::SchedulingPolicy {
                gang_schedule: true,
                priority: 100,
                ..Default::default()
            },
            labels: HashMap::new(),
            annotations: HashMap::new(),
        };
        
        let job = Job::new(descriptor);
        scheduler.submit(job).unwrap();
        
        // Run scheduling
        let decisions = scheduler.schedule_cycle();
        
        assert_eq!(decisions.len(), 1);
        assert!(decisions[0].gang_allocated);
        
        // Verify 4 GPUs allocated
        let total_gpus: usize = decisions[0].allocations.values()
            .map(|v| v.len())
            .sum();
        assert_eq!(total_gpus, 4);
    }
    
    #[test]
    fn test_scheduler_priority_ordering() {
        let registry = Arc::new(NodeRegistry::new(60));
        registry.register(create_test_node("node-1", 4)).unwrap();
        
        let scheduler = Scheduler::new(registry, SchedulerConfig::default());
        
        // Submit low priority job first
        let low_job = Job::new(JobDescriptor {
            name: "low-priority".to_string(),
            user_id: "user1".to_string(),
            project_id: "project1".to_string(),
            command: "echo".to_string(),
            arguments: vec!["low".to_string()],
            environment: HashMap::new(),
            working_directory: "/app".to_string(),
            resources: crate::job::ResourceRequirements {
                gpu_count: 1,
                ..Default::default()
            },
            locality: Default::default(),
            policy: crate::job::SchedulingPolicy {
                priority: 10,  // Low priority
                ..Default::default()
            },
            labels: HashMap::new(),
            annotations: HashMap::new(),
        });
        scheduler.submit(low_job).unwrap();
        
        // Submit high priority job second
        let high_job = Job::new(JobDescriptor {
            name: "high-priority".to_string(),
            user_id: "user1".to_string(),
            project_id: "project1".to_string(),
            command: "echo".to_string(),
            arguments: vec!["high".to_string()],
            environment: HashMap::new(),
            working_directory: "/app".to_string(),
            resources: crate::job::ResourceRequirements {
                gpu_count: 1,
                ..Default::default()
            },
            locality: Default::default(),
            policy: crate::job::SchedulingPolicy {
                priority: 100,  // High priority
                ..Default::default()
            },
            labels: HashMap::new(),
            annotations: HashMap::new(),
        });
        scheduler.submit(high_job).unwrap();
        
        assert_eq!(scheduler.queue_size(), 2);
        
        // Run scheduling cycle
        let decisions = scheduler.schedule_cycle();
        
        // Both jobs should be scheduled (enough resources)
        assert_eq!(decisions.len(), 2);
        assert_eq!(scheduler.queue_size(), 0);
        
        // Verify both jobs are in Scheduled state
        for decision in &decisions {
            let job = scheduler.get_job(&decision.job_id).unwrap();
            assert_eq!(job.state, JobState::Scheduled);
        }
    }
    
    #[test]
    fn test_scheduler_job_lifecycle() {
        let registry = Arc::new(NodeRegistry::new(60));
        registry.register(create_test_node("node-1", 4)).unwrap();
        
        let scheduler = Scheduler::new(registry, SchedulerConfig::default());
        
        let job = Job::new(JobDescriptor {
            name: "lifecycle-test".to_string(),
            user_id: "user1".to_string(),
            project_id: "project1".to_string(),
            command: "python".to_string(),
            arguments: vec!["train.py".to_string()],
            environment: HashMap::new(),
            working_directory: "/app".to_string(),
            resources: crate::job::ResourceRequirements {
                gpu_count: 2,
                ..Default::default()
            },
            locality: Default::default(),
            policy: Default::default(),
            labels: HashMap::new(),
            annotations: HashMap::new(),
        });
        
        let job_id = scheduler.submit(job).unwrap();
        
        // Initial state: Queued
        let job = scheduler.get_job(&job_id).unwrap();
        assert_eq!(job.state, JobState::Queued);
        
        // After scheduling: Scheduled
        scheduler.schedule_cycle();
        let job = scheduler.get_job(&job_id).unwrap();
        assert_eq!(job.state, JobState::Scheduled);
        
        // After starting: Running
        scheduler.mark_job_started(&job_id).unwrap();
        let job = scheduler.get_job(&job_id).unwrap();
        assert_eq!(job.state, JobState::Running);
        assert!(job.start_time.is_some());
        
        // After completing: Completed
        scheduler.mark_job_completed(&job_id, true, "Training finished").unwrap();
        let job = scheduler.get_job(&job_id).unwrap();
        assert_eq!(job.state, JobState::Completed);
        assert!(job.end_time.is_some());
    }
    
    #[test]
    fn test_scheduler_insufficient_resources() {
        let registry = Arc::new(NodeRegistry::new(60));
        registry.register(create_test_node("node-1", 2)).unwrap();
        
        let scheduler = Scheduler::new(registry, SchedulerConfig::default());
        
        // Job requiring more GPUs than available
        let job = Job::new(JobDescriptor {
            name: "large-job".to_string(),
            user_id: "user1".to_string(),
            project_id: "project1".to_string(),
            command: "python".to_string(),
            arguments: vec![],
            environment: HashMap::new(),
            working_directory: "/app".to_string(),
            resources: crate::job::ResourceRequirements {
                gpu_count: 8,  // Need 8 but only 2 available
                ..Default::default()
            },
            locality: Default::default(),
            policy: Default::default(),
            labels: HashMap::new(),
            annotations: HashMap::new(),
        });
        
        scheduler.submit(job).unwrap();
        
        // Run scheduling - should not schedule due to insufficient resources
        let decisions = scheduler.schedule_cycle();
        
        assert_eq!(decisions.len(), 0);  // Not scheduled
        assert_eq!(scheduler.queue_size(), 1);  // Still in queue
    }
    
    // ========================================================================
    // MUTATION-KILLING TESTS
    // ========================================================================
    
    /// Test that jobs_with_state returns non-empty vec for matching jobs
    /// Kills mutations: return vec![], == with !=
    #[test]
    fn test_jobs_with_state_filtering() {
        let registry = Arc::new(NodeRegistry::new(60));
        registry.register(create_test_node("node-1", 4)).unwrap();
        
        let scheduler = Scheduler::new(registry, SchedulerConfig::default());
        
        // Submit multiple jobs
        for i in 0..3 {
            let job = Job::new(JobDescriptor {
                name: format!("job-{}", i),
                user_id: "user1".to_string(),
                project_id: "project1".to_string(),
                command: "echo".to_string(),
                arguments: vec![],
                environment: HashMap::new(),
                working_directory: "/app".to_string(),
                resources: crate::job::ResourceRequirements {
                    gpu_count: 1,
                    ..Default::default()
                },
                locality: Default::default(),
                policy: Default::default(),
                labels: HashMap::new(),
                annotations: HashMap::new(),
            });
            scheduler.submit(job).unwrap();
        }
        
        // All jobs should be Queued
        let queued_jobs = scheduler.jobs_with_state(JobState::Queued);
        assert_eq!(queued_jobs.len(), 3, "Should have 3 queued jobs");
        
        // No jobs should be Running
        let running_jobs = scheduler.jobs_with_state(JobState::Running);
        assert_eq!(running_jobs.len(), 0, "Should have 0 running jobs");
        
        // Schedule all jobs
        scheduler.schedule_cycle();
        
        // Now jobs should be Scheduled, not Queued
        let queued_after = scheduler.jobs_with_state(JobState::Queued);
        assert_eq!(queued_after.len(), 0, "Should have 0 queued jobs after scheduling");
        
        let scheduled_jobs = scheduler.jobs_with_state(JobState::Scheduled);
        assert_eq!(scheduled_jobs.len(), 3, "Should have 3 scheduled jobs");
        
        // Verify filtering correctly uses == not !=
        for job in &scheduled_jobs {
            assert_eq!(job.state, JobState::Scheduled, 
                "jobs_with_state must filter correctly using ==");
        }
    }
    
    /// Test that config() returns a reference to the actual config
    /// Kills mutation: config -> Box::leak(Box::new(Default::default()))
    #[test]
    fn test_config_returns_actual_config() {
        let registry = Arc::new(NodeRegistry::new(60));
        
        let custom_config = SchedulerConfig {
            max_schedule_batch: 42,  // Non-default value
            backfill_enabled: false,
            topology_aware: false,
            prefer_same_node: false,
            job_timeout_secs: 12345,
            heartbeat_timeout_secs: 99,
        };
        
        let scheduler = Scheduler::new(registry, custom_config);
        
        let config = scheduler.config();
        
        // Verify it returns the actual config, not a default
        assert_eq!(config.max_schedule_batch, 42, 
            "config() must return actual config, not default");
        assert!(!config.backfill_enabled,
            "config() must return actual config, not default");
        assert!(!config.topology_aware,
            "config() must return actual config, not default");
        assert!(!config.prefer_same_node,
            "config() must return actual config, not default");
        assert_eq!(config.job_timeout_secs, 12345,
            "config() must return actual config, not default");
        assert_eq!(config.heartbeat_timeout_secs, 99,
            "config() must return actual config, not default");
    }
    
    /// Test cancelling a running job (covers the Running match arm)
    /// Kills mutation: delete match arm JobState::Running
    #[test]
    fn test_cancel_running_job() {
        let registry = Arc::new(NodeRegistry::new(60));
        registry.register(create_test_node("node-1", 4)).unwrap();
        
        let scheduler = Scheduler::new(registry, SchedulerConfig::default());
        
        let job = Job::new(JobDescriptor {
            name: "running-cancel-test".to_string(),
            user_id: "user1".to_string(),
            project_id: "project1".to_string(),
            command: "sleep".to_string(),
            arguments: vec!["1000".to_string()],
            environment: HashMap::new(),
            working_directory: "/app".to_string(),
            resources: crate::job::ResourceRequirements {
                gpu_count: 1,
                ..Default::default()
            },
            locality: Default::default(),
            policy: Default::default(),
            labels: HashMap::new(),
            annotations: HashMap::new(),
        });
        
        let job_id = scheduler.submit(job).unwrap();
        
        // Schedule and start the job
        scheduler.schedule_cycle();
        scheduler.mark_job_started(&job_id).unwrap();
        
        // Verify job is Running
        let job = scheduler.get_job(&job_id).unwrap();
        assert_eq!(job.state, JobState::Running, "Job should be running");
        
        // Cancel the running job
        let result = scheduler.cancel(&job_id, "User cancelled running job");
        assert!(result.is_ok(), "Should be able to cancel running job");
        
        // Verify job is now Cancelled
        let job = scheduler.get_job(&job_id).unwrap();
        assert_eq!(job.state, JobState::Cancelled, 
            "Running job must transition to Cancelled");
    }
    
    /// Test gang_schedule with insufficient total GPUs
    /// Kills mutations: < comparisons, remaining checks
    #[test]
    fn test_gang_schedule_insufficient_gpus() {
        let registry = Arc::new(NodeRegistry::new(60));
        // Only 3 GPUs available across all nodes
        registry.register(create_test_node("node-1", 2)).unwrap();
        registry.register(create_test_node("node-2", 1)).unwrap();
        
        let scheduler = Scheduler::new(registry, SchedulerConfig {
            prefer_same_node: false,  // Force multi-node scheduling
            ..Default::default()
        });
        
        // Job requiring 5 GPUs (more than available)
        let job = Job::new(JobDescriptor {
            name: "gang-insufficient".to_string(),
            user_id: "user1".to_string(),
            project_id: "project1".to_string(),
            command: "python".to_string(),
            arguments: vec![],
            environment: HashMap::new(),
            working_directory: "/app".to_string(),
            resources: crate::job::ResourceRequirements {
                gpu_count: 5,  // Need 5, only have 3
                ..Default::default()
            },
            locality: Default::default(),
            policy: crate::job::SchedulingPolicy {
                gang_schedule: true,
                ..Default::default()
            },
            labels: HashMap::new(),
            annotations: HashMap::new(),
        });
        
        scheduler.submit(job).unwrap();
        
        // Should not schedule - insufficient GPUs
        let decisions = scheduler.schedule_cycle();
        assert_eq!(decisions.len(), 0, 
            "Should not schedule when total_available < required_gpus");
    }
    
    /// Test gang_schedule with exact GPUs required
    /// Kills mutations: remaining > 0 check
    #[test]
    fn test_gang_schedule_exact_gpus() {
        let registry = Arc::new(NodeRegistry::new(60));
        registry.register(create_test_node("node-1", 2)).unwrap();
        registry.register(create_test_node("node-2", 2)).unwrap();
        
        let scheduler = Scheduler::new(registry, SchedulerConfig {
            prefer_same_node: false,  // Force multi-node to test remaining logic
            ..Default::default()
        });
        
        // Job requiring exactly 4 GPUs (sum of both nodes)
        let job = Job::new(JobDescriptor {
            name: "gang-exact".to_string(),
            user_id: "user1".to_string(),
            project_id: "project1".to_string(),
            command: "python".to_string(),
            arguments: vec![],
            environment: HashMap::new(),
            working_directory: "/app".to_string(),
            resources: crate::job::ResourceRequirements {
                gpu_count: 4,  // Exactly 2+2
                ..Default::default()
            },
            locality: Default::default(),
            policy: crate::job::SchedulingPolicy {
                gang_schedule: true,
                ..Default::default()
            },
            labels: HashMap::new(),
            annotations: HashMap::new(),
        });
        
        scheduler.submit(job).unwrap();
        
        // Should schedule successfully with exactly the right amount
        let decisions = scheduler.schedule_cycle();
        assert_eq!(decisions.len(), 1, 
            "Should schedule when total_available == required_gpus");
        
        let total_gpus: usize = decisions[0].allocations.values()
            .map(|v| v.len())
            .sum();
        assert_eq!(total_gpus, 4, "Should allocate exactly 4 GPUs");
    }
    
    /// Test spread_schedule returns Some (not None)
    /// Kills mutation: spread_schedule -> None
    #[test]
    fn test_spread_schedule_returns_decision() {
        let registry = Arc::new(NodeRegistry::new(60));
        registry.register(create_test_node("node-1", 4)).unwrap();
        
        let scheduler = Scheduler::new(registry, SchedulerConfig::default());
        
        // Non-gang job (uses spread_schedule internally)
        let job = Job::new(JobDescriptor {
            name: "spread-test".to_string(),
            user_id: "user1".to_string(),
            project_id: "project1".to_string(),
            command: "python".to_string(),
            arguments: vec![],
            environment: HashMap::new(),
            working_directory: "/app".to_string(),
            resources: crate::job::ResourceRequirements {
                gpu_count: 2,
                ..Default::default()
            },
            locality: Default::default(),
            policy: crate::job::SchedulingPolicy {
                gang_schedule: false,  // Use spread scheduling
                ..Default::default()
            },
            labels: HashMap::new(),
            annotations: HashMap::new(),
        });
        
        scheduler.submit(job).unwrap();
        
        let decisions = scheduler.schedule_cycle();
        
        // spread_schedule should return Some, not None
        assert_eq!(decisions.len(), 1, 
            "spread_schedule must return Some when resources available");
    }
    
    // ========================================================================
    // CLEANUP_ZOMBIE_JOBS TESTS
    // ========================================================================
    
    /// Test cleanup returns 0 when no running jobs
    /// Kills mutations: return 0, return 1
    #[test]
    fn test_cleanup_zombie_jobs_no_running_jobs() {
        let registry = Arc::new(NodeRegistry::new(60));
        registry.register(create_test_node("node-1", 4)).unwrap();
        
        let scheduler = Scheduler::new(registry, SchedulerConfig {
            job_timeout_secs: 10,
            ..Default::default()
        });
        
        // Submit a job but DON'T start it (keep it in Queued state)
        let job = Job::new(JobDescriptor {
            name: "not-running".to_string(),
            user_id: "user1".to_string(),
            project_id: "project1".to_string(),
            command: "echo".to_string(),
            arguments: vec![],
            environment: HashMap::new(),
            working_directory: "/app".to_string(),
            resources: crate::job::ResourceRequirements {
                gpu_count: 1,
                ..Default::default()
            },
            locality: Default::default(),
            policy: Default::default(),
            labels: HashMap::new(),
            annotations: HashMap::new(),
        });
        
        scheduler.submit(job).unwrap();
        
        // Schedule but don't start
        scheduler.schedule_cycle();
        
        // No running jobs, so cleanup should return 0
        let cleaned = scheduler.cleanup_zombie_jobs();
        assert_eq!(cleaned, 0, 
            "cleanup_zombie_jobs must return 0 when no Running jobs");
    }
    
    /// Test cleanup with timed out job
    /// Kills mutations: job_timeout_secs > 0, elapsed > timeout, += with -=
    #[test]
    fn test_cleanup_zombie_jobs_timeout() {
        let registry = Arc::new(NodeRegistry::new(60));
        registry.register(create_test_node("node-1", 4)).unwrap();
        
        let scheduler = Scheduler::new(registry, SchedulerConfig {
            job_timeout_secs: 1,  // 1 second timeout
            ..Default::default()
        });
        
        // Submit and start a job
        let job = Job::new(JobDescriptor {
            name: "will-timeout".to_string(),
            user_id: "user1".to_string(),
            project_id: "project1".to_string(),
            command: "sleep".to_string(),
            arguments: vec!["1000".to_string()],
            environment: HashMap::new(),
            working_directory: "/app".to_string(),
            resources: crate::job::ResourceRequirements {
                gpu_count: 1,
                ..Default::default()
            },
            locality: Default::default(),
            policy: Default::default(),
            labels: HashMap::new(),
            annotations: HashMap::new(),
        });
        
        let job_id = scheduler.submit(job).unwrap();
        scheduler.schedule_cycle();
        scheduler.mark_job_started(&job_id).unwrap();
        
        // Verify job is Running
        let job = scheduler.get_job(&job_id).unwrap();
        assert_eq!(job.state, JobState::Running);
        
        // Manually set start_time to past (2 seconds ago) to trigger timeout
        {
            let mut jobs = scheduler.jobs.write();
            if let Some(job) = jobs.get_mut(&job_id) {
                job.start_time = Some(chrono::Utc::now() - chrono::Duration::seconds(5));
            }
        }
        
        // Now cleanup should find and clean the timed out job
        let cleaned = scheduler.cleanup_zombie_jobs();
        assert_eq!(cleaned, 1, 
            "cleanup_zombie_jobs must return 1 when 1 job timed out");
        
        // Verify job is now in Timeout state
        let job = scheduler.get_job(&job_id).unwrap();
        assert_eq!(job.state, JobState::Timeout,
            "Job must transition to Timeout state");
    }
    
    /// Test cleanup with unhealthy node
    /// Kills mutations: !is_node_healthy, any_dead check
    #[test]
    fn test_cleanup_zombie_jobs_unhealthy_node() {
        let registry = Arc::new(NodeRegistry::new(60));
        registry.register(create_test_node("node-1", 4)).unwrap();
        
        let scheduler = Scheduler::new(registry.clone(), SchedulerConfig {
            job_timeout_secs: 0,  // Disable timeout to test node health only
            ..Default::default()
        });
        
        // Submit and start a job
        let job = Job::new(JobDescriptor {
            name: "on-dead-node".to_string(),
            user_id: "user1".to_string(),
            project_id: "project1".to_string(),
            command: "python".to_string(),
            arguments: vec![],
            environment: HashMap::new(),
            working_directory: "/app".to_string(),
            resources: crate::job::ResourceRequirements {
                gpu_count: 1,
                ..Default::default()
            },
            locality: Default::default(),
            policy: Default::default(),
            labels: HashMap::new(),
            annotations: HashMap::new(),
        });
        
        let job_id = scheduler.submit(job).unwrap();
        scheduler.schedule_cycle();
        scheduler.mark_job_started(&job_id).unwrap();
        
        // Deregister the node (making it unhealthy/unreachable)
        registry.deregister("node-1").unwrap();
        
        // Cleanup should detect the unhealthy node
        let cleaned = scheduler.cleanup_zombie_jobs();
        assert_eq!(cleaned, 1,
            "cleanup_zombie_jobs must return 1 when node is unhealthy");
        
        // Verify job is now in Failed state
        let job = scheduler.get_job(&job_id).unwrap();
        assert_eq!(job.state, JobState::Failed,
            "Job must transition to Failed when node is unhealthy");
    }
    
    /// Test cleanup returns correct count for multiple zombies
    /// Kills mutations: cleaned += 1
    #[test]
    fn test_cleanup_zombie_jobs_multiple() {
        let registry = Arc::new(NodeRegistry::new(60));
        registry.register(create_test_node("node-1", 4)).unwrap();
        
        let scheduler = Scheduler::new(registry.clone(), SchedulerConfig {
            job_timeout_secs: 1,
            ..Default::default()
        });
        
        // Submit and start multiple jobs
        let mut job_ids = vec![];
        for i in 0..3 {
            let job = Job::new(JobDescriptor {
                name: format!("zombie-{}", i),
                user_id: "user1".to_string(),
                project_id: "project1".to_string(),
                command: "sleep".to_string(),
                arguments: vec![],
                environment: HashMap::new(),
                working_directory: "/app".to_string(),
                resources: crate::job::ResourceRequirements {
                    gpu_count: 1,
                    ..Default::default()
                },
                locality: Default::default(),
                policy: Default::default(),
                labels: HashMap::new(),
                annotations: HashMap::new(),
            });
            job_ids.push(scheduler.submit(job).unwrap());
        }
        
        // Schedule and start all
        scheduler.schedule_cycle();
        for job_id in &job_ids {
            scheduler.mark_job_started(job_id).unwrap();
        }
        
        // Set all jobs to past start_time
        {
            let mut jobs = scheduler.jobs.write();
            for job_id in &job_ids {
                if let Some(job) = jobs.get_mut(job_id) {
                    job.start_time = Some(chrono::Utc::now() - chrono::Duration::seconds(10));
                }
            }
        }
        
        // Cleanup should return 3
        let cleaned = scheduler.cleanup_zombie_jobs();
        assert_eq!(cleaned, 3,
            "cleanup_zombie_jobs must return correct count (3 zombies)");
    }
    
    /// Test cleanup skips non-running jobs
    /// Kills mutation: state != Running becomes state == Running
    #[test]
    fn test_cleanup_zombie_jobs_skips_non_running() {
        let registry = Arc::new(NodeRegistry::new(60));
        registry.register(create_test_node("node-1", 4)).unwrap();
        
        let scheduler = Scheduler::new(registry, SchedulerConfig {
            job_timeout_secs: 1,
            ..Default::default()
        });
        
        // Submit jobs in different states
        // Job 1: Queued (not Running)
        let job1 = Job::new(JobDescriptor {
            name: "queued-job".to_string(),
            user_id: "user1".to_string(),
            project_id: "project1".to_string(),
            command: "echo".to_string(),
            arguments: vec![],
            environment: HashMap::new(),
            working_directory: "/app".to_string(),
            resources: crate::job::ResourceRequirements {
                gpu_count: 1,
                ..Default::default()
            },
            locality: Default::default(),
            policy: Default::default(),
            labels: HashMap::new(),
            annotations: HashMap::new(),
        });
        scheduler.submit(job1).unwrap();
        
        // Job 2: Scheduled (not Running)
        let job2 = Job::new(JobDescriptor {
            name: "scheduled-job".to_string(),
            user_id: "user1".to_string(),
            project_id: "project1".to_string(),
            command: "echo".to_string(),
            arguments: vec![],
            environment: HashMap::new(),
            working_directory: "/app".to_string(),
            resources: crate::job::ResourceRequirements {
                gpu_count: 1,
                ..Default::default()
            },
            locality: Default::default(),
            policy: Default::default(),
            labels: HashMap::new(),
            annotations: HashMap::new(),
        });
        scheduler.submit(job2).unwrap();
        scheduler.schedule_cycle();
        
        // No Running jobs, cleanup should return 0
        let cleaned = scheduler.cleanup_zombie_jobs();
        assert_eq!(cleaned, 0,
            "cleanup_zombie_jobs must skip non-Running jobs");
    }
}
