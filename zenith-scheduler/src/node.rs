//! Node registry and management

use crate::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use parking_lot::RwLock;
use chrono::{DateTime, Utc};

/// GPU device information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuDevice {
    /// Device ID (e.g., "cuda:0")
    pub device_id: String,
    /// Device name (e.g., "NVIDIA A100-SXM4-80GB")
    pub device_name: String,
    /// UUID
    pub uuid: String,
    /// Total memory in bytes
    pub total_memory: u64,
    /// Free memory in bytes
    pub free_memory: u64,
    /// GPU utilization (0.0-1.0)
    pub utilization: f32,
    /// Temperature in Celsius
    pub temperature: i32,
    /// Is currently allocated to a job
    pub allocated: bool,
    /// Job ID if allocated
    pub allocated_job_id: Option<String>,
}

/// Node topology information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeTopology {
    /// GPU devices
    pub gpus: Vec<GpuDevice>,
    /// Number of CPU cores
    pub cpu_cores: u32,
    /// Total CPU memory in bytes
    pub cpu_memory: u64,
    /// Free CPU memory in bytes
    pub cpu_memory_free: u64,
    /// NUMA node count
    pub numa_nodes: u32,
    /// NVLink present
    pub nvlink_present: bool,
    /// NVSwitch present
    pub nvswitch_present: bool,
    /// RDMA capable
    pub rdma_capable: bool,
}

/// Node health status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeHealth {
    /// Node is healthy
    Healthy,
    /// Node has warnings
    Warning,
    /// Node is unhealthy
    Unhealthy,
    /// Node is unreachable
    Unreachable,
}

/// A compute node in the cluster
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    /// Unique node ID
    pub id: String,
    /// Hostname
    pub hostname: String,
    /// IP address
    pub ip_address: String,
    /// Node topology
    pub topology: NodeTopology,
    /// Health status
    pub health: NodeHealth,
    /// Health message
    pub health_message: String,
    /// Registration time
    pub registered_at: DateTime<Utc>,
    /// Last heartbeat time
    pub last_heartbeat: DateTime<Utc>,
    /// Labels for filtering
    pub labels: HashMap<String, String>,
    /// Running job IDs
    pub running_jobs: Vec<String>,
}

impl Node {
    /// Create a new node
    pub fn new(id: String, hostname: String, ip_address: String, topology: NodeTopology) -> Self {
        let now = Utc::now();
        Self {
            id,
            hostname,
            ip_address,
            topology,
            health: NodeHealth::Healthy,
            health_message: "OK".to_string(),
            registered_at: now,
            last_heartbeat: now,
            labels: HashMap::new(),
            running_jobs: vec![],
        }
    }
    
    /// Get available GPU count
    pub fn available_gpus(&self) -> usize {
        self.topology.gpus.iter()
            .filter(|g| !g.allocated)
            .count()
    }
    
    /// Get total GPU count
    pub fn total_gpus(&self) -> usize {
        self.topology.gpus.len()
    }
    
    /// Update heartbeat
    pub fn heartbeat(&mut self) {
        self.last_heartbeat = Utc::now();
    }
    
    /// Check if node is stale (no heartbeat recently)
    pub fn is_stale(&self, max_age_seconds: i64) -> bool {
        let age = Utc::now() - self.last_heartbeat;
        age.num_seconds() > max_age_seconds
    }
    
    /// Allocate GPUs to a job
    pub fn allocate_gpus(&mut self, job_id: &str, count: usize) -> Result<Vec<String>> {
        let available: Vec<&mut GpuDevice> = self.topology.gpus.iter_mut()
            .filter(|g| !g.allocated)
            .take(count)
            .collect();
        
        if available.len() < count {
            return Err(Error::Node(format!(
                "Not enough GPUs: requested {}, available {}",
                count, available.len()
            )));
        }
        
        let mut allocated_ids = vec![];
        for gpu in self.topology.gpus.iter_mut().filter(|g| !g.allocated).take(count) {
            gpu.allocated = true;
            gpu.allocated_job_id = Some(job_id.to_string());
            allocated_ids.push(gpu.device_id.clone());
        }
        
        self.running_jobs.push(job_id.to_string());
        Ok(allocated_ids)
    }
    
    /// Release GPUs from a job
    pub fn release_gpus(&mut self, job_id: &str) {
        for gpu in &mut self.topology.gpus {
            if gpu.allocated_job_id.as_deref() == Some(job_id) {
                gpu.allocated = false;
                gpu.allocated_job_id = None;
            }
        }
        self.running_jobs.retain(|id| id != job_id);
    }
}

/// Node registry - manages all nodes in the cluster
pub struct NodeRegistry {
    nodes: RwLock<HashMap<String, Node>>,
    heartbeat_timeout_seconds: i64,
}

impl NodeRegistry {
    /// Create a new node registry
    pub fn new(heartbeat_timeout_seconds: i64) -> Self {
        Self {
            nodes: RwLock::new(HashMap::new()),
            heartbeat_timeout_seconds,
        }
    }
    
    /// Register a new node
    pub fn register(&self, node: Node) -> Result<()> {
        let mut nodes = self.nodes.write();
        nodes.insert(node.id.clone(), node);
        Ok(())
    }
    
    /// Deregister a node
    pub fn deregister(&self, node_id: &str) -> Result<()> {
        let mut nodes = self.nodes.write();
        nodes.remove(node_id);
        Ok(())
    }
    
    /// Update node status
    pub fn update(&self, node_id: &str, topology: NodeTopology) -> Result<()> {
        let mut nodes = self.nodes.write();
        if let Some(node) = nodes.get_mut(node_id) {
            node.topology = topology;
            node.heartbeat();
            Ok(())
        } else {
            Err(Error::Node(format!("Node not found: {}", node_id)))
        }
    }
    
    /// Get a node by ID
    pub fn get(&self, node_id: &str) -> Option<Node> {
        self.nodes.read().get(node_id).cloned()
    }
    
    /// Get all healthy nodes
    pub fn healthy_nodes(&self) -> Vec<Node> {
        self.nodes.read()
            .values()
            .filter(|n| n.health == NodeHealth::Healthy && !n.is_stale(self.heartbeat_timeout_seconds))
            .cloned()
            .collect()
    }
    
    /// Get nodes with available GPUs
    pub fn nodes_with_available_gpus(&self, count: usize) -> Vec<Node> {
        self.healthy_nodes()
            .into_iter()
            .filter(|n| n.available_gpus() >= count)
            .collect()
    }
    
    /// Check if a specific node is healthy
    pub fn is_node_healthy(&self, node_id: &str) -> bool {
        if let Some(node) = self.nodes.read().get(node_id) {
            node.health == NodeHealth::Healthy && !node.is_stale(self.heartbeat_timeout_seconds)
        } else {
            false
        }
    }
    
    /// Get cluster summary
    pub fn summary(&self) -> ClusterSummary {
        let nodes = self.nodes.read();
        let healthy_nodes: Vec<_> = nodes.values()
            .filter(|n| n.health == NodeHealth::Healthy)
            .collect();
        
        ClusterSummary {
            total_nodes: nodes.len(),
            healthy_nodes: healthy_nodes.len(),
            total_gpus: nodes.values().map(|n| n.total_gpus()).sum(),
            available_gpus: nodes.values().map(|n| n.available_gpus()).sum(),
            running_jobs: nodes.values().map(|n| n.running_jobs.len()).sum(),
        }
    }
}

/// Cluster summary statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterSummary {
    /// Total number of nodes
    pub total_nodes: usize,
    /// Number of healthy nodes
    pub healthy_nodes: usize,
    /// Total GPUs in cluster
    pub total_gpus: usize,
    /// Available GPUs
    pub available_gpus: usize,
    /// Running jobs
    pub running_jobs: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    fn create_test_node() -> Node {
        let gpu = GpuDevice {
            device_id: "cuda:0".to_string(),
            device_name: "NVIDIA A100".to_string(),
            uuid: "GPU-12345".to_string(),
            total_memory: 80 * 1024 * 1024 * 1024,
            free_memory: 80 * 1024 * 1024 * 1024,
            utilization: 0.0,
            temperature: 40,
            allocated: false,
            allocated_job_id: None,
        };
        
        let topology = NodeTopology {
            gpus: vec![gpu],
            cpu_cores: 64,
            cpu_memory: 512 * 1024 * 1024 * 1024,
            cpu_memory_free: 500 * 1024 * 1024 * 1024,
            numa_nodes: 2,
            nvlink_present: true,
            nvswitch_present: false,
            rdma_capable: true,
        };
        
        Node::new(
            "node-1".to_string(),
            "gpu-node-1".to_string(),
            "192.168.1.1".to_string(),
            topology,
        )
    }
    
    #[test]
    fn test_node_creation() {
        let node = create_test_node();
        assert_eq!(node.available_gpus(), 1);
        assert_eq!(node.health, NodeHealth::Healthy);
    }
    
    #[test]
    fn test_gpu_allocation() {
        let mut node = create_test_node();
        
        let allocated = node.allocate_gpus("job-1", 1).unwrap();
        assert_eq!(allocated.len(), 1);
        assert_eq!(node.available_gpus(), 0);
        
        node.release_gpus("job-1");
        assert_eq!(node.available_gpus(), 1);
    }
    
    #[test]
    fn test_node_registry() {
        let registry = NodeRegistry::new(60);
        let node = create_test_node();
        
        registry.register(node.clone()).unwrap();
        
        let retrieved = registry.get("node-1").unwrap();
        assert_eq!(retrieved.hostname, "gpu-node-1");
        
        let summary = registry.summary();
        assert_eq!(summary.total_nodes, 1);
        assert_eq!(summary.total_gpus, 1);
    }
}
