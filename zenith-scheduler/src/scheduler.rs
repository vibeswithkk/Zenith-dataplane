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
        
        let mut processed = 0;
        let mut to_remove = vec![];
        
        // Process jobs in priority order
        for (job_id, _priority) in queue.iter() {
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
            
            processed += 1;
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
    }
}
