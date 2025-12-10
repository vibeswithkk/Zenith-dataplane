//! State Persistence for Scheduler
//!
//! Provides durable storage for job and node state.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::fs;
use parking_lot::RwLock;

use crate::job::{Job, JobState};
use crate::{Error, Result};

/// State store configuration
#[derive(Debug, Clone)]
pub struct StateStoreConfig {
    /// Data directory
    pub data_dir: PathBuf,
    /// Enable WAL
    pub enable_wal: bool,
    /// Sync writes to disk
    pub sync_writes: bool,
    /// Checkpoint interval in seconds
    pub checkpoint_interval_secs: u64,
}

impl Default for StateStoreConfig {
    fn default() -> Self {
        Self {
            data_dir: PathBuf::from("/var/lib/zenith/scheduler"),
            enable_wal: true,
            sync_writes: true,
            checkpoint_interval_secs: 60,
        }
    }
}

/// Persistent state store
pub struct StateStore {
    config: StateStoreConfig,
    jobs: RwLock<HashMap<String, Job>>,
    nodes: RwLock<HashMap<String, NodeState>>,
}

/// Persisted node state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeState {
    /// Node ID
    pub id: String,
    /// Last heartbeat timestamp
    pub last_heartbeat: i64,
    /// Registration time
    pub registered_at: i64,
    /// Allocated jobs
    pub allocated_jobs: Vec<String>,
}

impl StateStore {
    /// Create a new state store
    pub fn new(config: StateStoreConfig) -> Result<Self> {
        // Create data directory if needed
        if !config.data_dir.exists() {
            fs::create_dir_all(&config.data_dir)
                .map_err(Error::Io)?;
        }
        
        let store = Self {
            config,
            jobs: RwLock::new(HashMap::new()),
            nodes: RwLock::new(HashMap::new()),
        };
        
        // Load existing state
        store.load()?;
        
        Ok(store)
    }
    
    /// Load state from disk
    fn load(&self) -> Result<()> {
        let jobs_path = self.config.data_dir.join("jobs.json");
        let nodes_path = self.config.data_dir.join("nodes.json");
        
        // Load jobs
        if jobs_path.exists() {
            let data = fs::read_to_string(&jobs_path)
                .map_err(Error::Io)?;
            let jobs: HashMap<String, Job> = serde_json::from_str(&data)
                .map_err(|e| Error::Serialization(e.to_string()))?;
            *self.jobs.write() = jobs;
        }
        
        // Load nodes
        if nodes_path.exists() {
            let data = fs::read_to_string(&nodes_path)
                .map_err(Error::Io)?;
            let nodes: HashMap<String, NodeState> = serde_json::from_str(&data)
                .map_err(|e| Error::Serialization(e.to_string()))?;
            *self.nodes.write() = nodes;
        }
        
        Ok(())
    }
    
    /// Save state to disk
    pub fn save(&self) -> Result<()> {
        let jobs_path = self.config.data_dir.join("jobs.json");
        let nodes_path = self.config.data_dir.join("nodes.json");
        
        // Save jobs
        let jobs = self.jobs.read();
        let jobs_data = serde_json::to_string_pretty(&*jobs)
            .map_err(|e| Error::Serialization(e.to_string()))?;
        fs::write(&jobs_path, jobs_data)
            .map_err(Error::Io)?;
        
        // Save nodes
        let nodes = self.nodes.read();
        let nodes_data = serde_json::to_string_pretty(&*nodes)
            .map_err(|e| Error::Serialization(e.to_string()))?;
        fs::write(&nodes_path, nodes_data)
            .map_err(Error::Io)?;
        
        Ok(())
    }
    
    /// Store a job
    pub fn store_job(&self, job: &Job) -> Result<()> {
        self.jobs.write().insert(job.id.to_string(), job.clone());
        
        if self.config.sync_writes {
            self.save()?;
        }
        
        Ok(())
    }
    
    /// Get a job
    pub fn get_job(&self, job_id: &str) -> Option<Job> {
        self.jobs.read().get(job_id).cloned()
    }
    
    /// Update job state
    pub fn update_job_state(&self, job_id: &str, state: JobState, message: &str) -> Result<()> {
        let mut jobs = self.jobs.write();
        
        if let Some(job) = jobs.get_mut(job_id) {
            job.transition(state, message);
            
            if self.config.sync_writes {
                drop(jobs);
                self.save()?;
            }
            
            Ok(())
        } else {
            Err(Error::Job(format!("Job not found: {}", job_id)))
        }
    }
    
    /// Delete a job
    pub fn delete_job(&self, job_id: &str) -> Result<()> {
        self.jobs.write().remove(job_id);
        
        if self.config.sync_writes {
            self.save()?;
        }
        
        Ok(())
    }
    
    /// List jobs by state
    pub fn list_jobs_by_state(&self, state: JobState) -> Vec<Job> {
        self.jobs.read()
            .values()
            .filter(|j| j.state == state)
            .cloned()
            .collect()
    }
    
    /// List all jobs
    pub fn list_all_jobs(&self) -> Vec<Job> {
        self.jobs.read().values().cloned().collect()
    }
    
    /// Store node state
    pub fn store_node(&self, node_state: NodeState) -> Result<()> {
        self.nodes.write().insert(node_state.id.clone(), node_state);
        
        if self.config.sync_writes {
            self.save()?;
        }
        
        Ok(())
    }
    
    /// Get node state
    pub fn get_node(&self, node_id: &str) -> Option<NodeState> {
        self.nodes.read().get(node_id).cloned()
    }
    
    /// List all nodes
    pub fn list_nodes(&self) -> Vec<NodeState> {
        self.nodes.read().values().cloned().collect()
    }
    
    /// Get job counts by state
    pub fn job_counts(&self) -> HashMap<JobState, usize> {
        let jobs = self.jobs.read();
        let mut counts = HashMap::new();
        
        for job in jobs.values() {
            *counts.entry(job.state).or_insert(0) += 1;
        }
        
        counts
    }
    
    /// Cleanup completed/failed jobs older than given seconds
    pub fn cleanup_old_jobs(&self, max_age_secs: i64) -> Result<usize> {
        let now = chrono::Utc::now().timestamp();
        let mut jobs = self.jobs.write();
        
        let to_remove: Vec<String> = jobs.iter()
            .filter(|(_, job)| {
                matches!(job.state, JobState::Completed | JobState::Failed | JobState::Cancelled) &&
                job.end_time.map(|t| now - t.timestamp() > max_age_secs).unwrap_or(false)
            })
            .map(|(id, _)| id.clone())
            .collect();
        
        let count = to_remove.len();
        for id in to_remove {
            jobs.remove(&id);
        }
        
        drop(jobs);
        
        if count > 0 && self.config.sync_writes {
            self.save()?;
        }
        
        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::job::{JobDescriptor, ResourceRequirements, LocalityPreferences, SchedulingPolicy};
    use tempfile::TempDir;
    
    fn create_test_job() -> Job {
        let descriptor = JobDescriptor {
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
        };
        
        Job::new(descriptor)
    }
    
    #[test]
    fn test_state_store() {
        let temp_dir = TempDir::new().unwrap();
        
        let config = StateStoreConfig {
            data_dir: temp_dir.path().to_path_buf(),
            enable_wal: false,
            sync_writes: true,
            checkpoint_interval_secs: 60,
        };
        
        let store = StateStore::new(config).unwrap();
        
        // Store a job
        let job = create_test_job();
        let job_id = job.id.to_string();
        store.store_job(&job).unwrap();
        
        // Retrieve job
        let retrieved = store.get_job(&job_id).unwrap();
        assert_eq!(retrieved.descriptor.name, "test-job");
        
        // Update state
        store.update_job_state(&job_id, JobState::Running, "Started").unwrap();
        
        let updated = store.get_job(&job_id).unwrap();
        assert_eq!(updated.state, JobState::Running);
    }
    
    #[test]
    fn test_job_counts() {
        let temp_dir = TempDir::new().unwrap();
        
        let config = StateStoreConfig {
            data_dir: temp_dir.path().to_path_buf(),
            sync_writes: false,
            ..Default::default()
        };
        
        let store = StateStore::new(config).unwrap();
        
        let job1 = create_test_job();
        let job2 = create_test_job();
        let mut job3 = create_test_job();
        job3.state = JobState::Running;
        
        store.store_job(&job1).unwrap();
        store.store_job(&job2).unwrap();
        store.store_job(&job3).unwrap();
        
        let counts = store.job_counts();
        assert_eq!(counts.get(&JobState::Pending), Some(&2));
        assert_eq!(counts.get(&JobState::Running), Some(&1));
    }
}
