//! Node Agent - Runs on compute nodes

use crate::node::{Node, NodeTopology, GpuDevice};
use crate::Result;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::time::interval;
use tracing::{info, warn, debug};

/// Node agent configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeAgentConfig {
    /// Node ID (usually hostname)
    pub node_id: String,
    /// Scheduler address
    pub scheduler_addr: String,
    /// Heartbeat interval in seconds
    pub heartbeat_interval_secs: u64,
    /// GPU monitoring interval in seconds
    pub gpu_monitor_interval_secs: u64,
}

impl Default for NodeAgentConfig {
    fn default() -> Self {
        let hostname = hostname::get()
            .map(|h| h.to_string_lossy().to_string())
            .unwrap_or_else(|_| "unknown".to_string());
        
        Self {
            node_id: hostname,
            scheduler_addr: "http://localhost:50051".to_string(),
            heartbeat_interval_secs: 30,
            gpu_monitor_interval_secs: 10,
        }
    }
}

/// Node agent that reports to the scheduler
pub struct NodeAgent {
    config: NodeAgentConfig,
    node: Node,
    running: bool,
}

impl NodeAgent {
    /// Create a new node agent
    pub fn new(config: NodeAgentConfig) -> Result<Self> {
        let topology = Self::discover_topology()?;
        let node = Node::new(
            config.node_id.clone(),
            config.node_id.clone(),
            Self::get_ip_address(),
            topology,
        );
        
        Ok(Self {
            config,
            node,
            running: false,
        })
    }
    
    /// Discover local GPU topology
    fn discover_topology() -> Result<NodeTopology> {
        // Try to discover real GPUs via nvidia-smi
        let gpus = Self::discover_gpus();
        
        let cpu_cores = num_cpus::get() as u32;
        let sys = sysinfo::System::new_all();
        let cpu_memory = sys.total_memory();
        let cpu_memory_free = sys.available_memory();
        
        // Detect NUMA nodes
        let numa_nodes = Self::detect_numa_nodes();
        
        Ok(NodeTopology {
            gpus,
            cpu_cores,
            cpu_memory,
            cpu_memory_free,
            numa_nodes,
            nvlink_present: false,  // Would need nvml to detect
            nvswitch_present: false,
            rdma_capable: Self::detect_rdma(),
        })
    }
    
    /// Discover GPUs via nvidia-smi
    fn discover_gpus() -> Vec<GpuDevice> {
        // Try running nvidia-smi
        match std::process::Command::new("nvidia-smi")
            .args(["--query-gpu=index,name,uuid,memory.total,memory.free,utilization.gpu,temperature.gpu", "--format=csv,noheader,nounits"])
            .output()
        {
            Ok(output) if output.status.success() => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                stdout.lines()
                    .filter_map(Self::parse_gpu_line)
                    .collect()
            }
            _ => {
                debug!("nvidia-smi not available, returning empty GPU list");
                vec![]
            }
        }
    }
    
    /// Parse nvidia-smi output line
    fn parse_gpu_line(line: &str) -> Option<GpuDevice> {
        let parts: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
        if parts.len() >= 7 {
            Some(GpuDevice {
                device_id: format!("cuda:{}", parts[0]),
                device_name: parts[1].to_string(),
                uuid: parts[2].to_string(),
                total_memory: parts[3].parse::<u64>().unwrap_or(0) * 1024 * 1024,
                free_memory: parts[4].parse::<u64>().unwrap_or(0) * 1024 * 1024,
                utilization: parts[5].parse::<f32>().unwrap_or(0.0) / 100.0,
                temperature: parts[6].parse::<i32>().unwrap_or(0),
                allocated: false,
                allocated_job_id: None,
            })
        } else {
            None
        }
    }
    
    /// Get local IP address
    fn get_ip_address() -> String {
        // Try to get non-loopback IP
        if let Ok(addrs) = local_ip_address::list_afinet_netifas() {
            for (_, ip) in addrs {
                if !ip.is_loopback() {
                    return ip.to_string();
                }
            }
        }
        "127.0.0.1".to_string()
    }
    
    /// Detect NUMA node count
    fn detect_numa_nodes() -> u32 {
        if let Ok(entries) = std::fs::read_dir("/sys/devices/system/node") {
            entries
                .filter_map(|e| e.ok())
                .filter(|e| e.file_name().to_string_lossy().starts_with("node"))
                .count() as u32
        } else {
            1
        }
    }
    
    /// Detect RDMA capability
    fn detect_rdma() -> bool {
        std::path::Path::new("/sys/class/infiniband").exists()
    }
    
    /// Start the node agent
    pub async fn start(&mut self) -> Result<()> {
        info!("Starting node agent: {}", self.config.node_id);
        self.running = true;
        
        // Register with scheduler
        self.register().await?;
        
        // Start heartbeat loop
        let mut heartbeat_interval = interval(
            Duration::from_secs(self.config.heartbeat_interval_secs)
        );
        
        while self.running {
            heartbeat_interval.tick().await;
            
            // Update topology
            if let Ok(topology) = Self::discover_topology() {
                self.node.topology = topology;
            }
            
            // Send heartbeat
            if let Err(e) = self.send_heartbeat().await {
                warn!("Failed to send heartbeat: {}", e);
            }
        }
        
        Ok(())
    }
    
    /// Stop the node agent
    pub fn stop(&mut self) {
        info!("Stopping node agent");
        self.running = false;
    }
    
    /// Register with scheduler
    async fn register(&self) -> Result<()> {
        info!("Registering with scheduler at {}", self.config.scheduler_addr);
        
        // In production: gRPC call to scheduler
        // For now, just log
        info!("Node registered: {} with {} GPUs", 
            self.node.id, 
            self.node.topology.gpus.len()
        );
        
        Ok(())
    }
    
    /// Send heartbeat to scheduler
    async fn send_heartbeat(&self) -> Result<()> {
        debug!("Sending heartbeat");
        
        // In production: gRPC call to scheduler
        // For now, just log
        
        Ok(())
    }
    
    /// Get current node status
    pub fn status(&self) -> &Node {
        &self.node
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_default_config() {
        let config = NodeAgentConfig::default();
        assert!(!config.node_id.is_empty());
        assert_eq!(config.heartbeat_interval_secs, 30);
    }
}
