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
    
    // ===================== Config Tests =====================
    
    #[test]
    fn test_default_config() {
        let config = NodeAgentConfig::default();
        assert!(!config.node_id.is_empty());
        assert_eq!(config.heartbeat_interval_secs, 30);
        assert_eq!(config.gpu_monitor_interval_secs, 10);
        assert_eq!(config.scheduler_addr, "http://localhost:50051");
    }
    
    #[test]
    fn test_config_custom() {
        let config = NodeAgentConfig {
            node_id: "custom-node".to_string(),
            scheduler_addr: "http://scheduler:50051".to_string(),
            heartbeat_interval_secs: 60,
            gpu_monitor_interval_secs: 30,
        };
        
        assert_eq!(config.node_id, "custom-node");
        assert_eq!(config.scheduler_addr, "http://scheduler:50051");
        assert_eq!(config.heartbeat_interval_secs, 60);
        assert_eq!(config.gpu_monitor_interval_secs, 30);
    }
    
    #[test]
    fn test_config_clone() {
        let config = NodeAgentConfig::default();
        let cloned = config.clone();
        
        assert_eq!(config.node_id, cloned.node_id);
        assert_eq!(config.scheduler_addr, cloned.scheduler_addr);
        assert_eq!(config.heartbeat_interval_secs, cloned.heartbeat_interval_secs);
    }
    
    #[test]
    fn test_config_debug() {
        let config = NodeAgentConfig::default();
        let debug_str = format!("{:?}", config);
        assert!(debug_str.contains("NodeAgentConfig"));
        assert!(debug_str.contains("node_id"));
    }
    
    #[test]
    fn test_config_serialize() {
        let config = NodeAgentConfig {
            node_id: "test-node".to_string(),
            scheduler_addr: "http://localhost:50051".to_string(),
            heartbeat_interval_secs: 30,
            gpu_monitor_interval_secs: 10,
        };
        
        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("test-node"));
        assert!(json.contains("50051"));
    }
    
    #[test]
    fn test_config_deserialize() {
        let json = r#"{
            "node_id": "json-node",
            "scheduler_addr": "http://sched:8080",
            "heartbeat_interval_secs": 45,
            "gpu_monitor_interval_secs": 15
        }"#;
        
        let config: NodeAgentConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.node_id, "json-node");
        assert_eq!(config.scheduler_addr, "http://sched:8080");
        assert_eq!(config.heartbeat_interval_secs, 45);
    }
    
    // ===================== GPU Parsing Tests =====================
    
    #[test]
    fn test_parse_gpu_line_valid() {
        let line = "0, NVIDIA A100, GPU-abc123, 40960, 35000, 25, 45";
        let gpu = NodeAgent::parse_gpu_line(line);
        
        assert!(gpu.is_some());
        let gpu = gpu.unwrap();
        assert_eq!(gpu.device_id, "cuda:0");
        assert_eq!(gpu.device_name, "NVIDIA A100");
        assert_eq!(gpu.uuid, "GPU-abc123");
        assert_eq!(gpu.total_memory, 40960 * 1024 * 1024);
        assert_eq!(gpu.free_memory, 35000 * 1024 * 1024);
        assert!((gpu.utilization - 0.25).abs() < 0.01);
        assert_eq!(gpu.temperature, 45);
        assert!(!gpu.allocated);
        assert!(gpu.allocated_job_id.is_none());
    }
    
    #[test]
    fn test_parse_gpu_line_h100() {
        let line = "1, NVIDIA H100 80GB HBM3, GPU-h100-xyz, 81920, 80000, 5, 38";
        let gpu = NodeAgent::parse_gpu_line(line);
        
        assert!(gpu.is_some());
        let gpu = gpu.unwrap();
        assert_eq!(gpu.device_id, "cuda:1");
        assert!(gpu.device_name.contains("H100"));
    }
    
    #[test]
    fn test_parse_gpu_line_invalid_short() {
        let line = "0, NVIDIA A100";  // Too few fields
        let gpu = NodeAgent::parse_gpu_line(line);
        assert!(gpu.is_none());
    }
    
    #[test]
    fn test_parse_gpu_line_empty() {
        let line = "";
        let gpu = NodeAgent::parse_gpu_line(line);
        assert!(gpu.is_none());
    }
    
    #[test]
    fn test_parse_gpu_line_invalid_numbers() {
        let line = "0, GPU, uuid, invalid, 35000, 25, 45";
        let gpu = NodeAgent::parse_gpu_line(line);
        
        // Should still parse, but with 0 for invalid numeric fields
        assert!(gpu.is_some());
        let gpu = gpu.unwrap();
        assert_eq!(gpu.total_memory, 0);  // "invalid" parsed as 0
    }
    
    #[test]
    fn test_parse_gpu_line_whitespace() {
        let line = "  0  ,  NVIDIA A100  ,  GPU-123  ,  40960  ,  30000  ,  50  ,  60  ";
        let gpu = NodeAgent::parse_gpu_line(line);
        
        assert!(gpu.is_some());
        let gpu = gpu.unwrap();
        assert_eq!(gpu.device_id, "cuda:0");
        assert_eq!(gpu.device_name, "NVIDIA A100");
    }
    
    // ===================== Detection Tests =====================
    
    #[test]
    fn test_detect_numa_nodes() {
        // This will return at least 1 on any Linux system
        let numa_count = NodeAgent::detect_numa_nodes();
        assert!(numa_count >= 1);
    }
    
    #[test]
    fn test_detect_rdma() {
        // Just verify it doesn't panic
        let _rdma = NodeAgent::detect_rdma();
        // Most systems don't have RDMA, so we just check it returns a bool
    }
    
    #[test]
    fn test_get_ip_address() {
        let ip = NodeAgent::get_ip_address();
        // Should return a valid IP string
        assert!(!ip.is_empty());
        // Either loopback or a real IP
        assert!(ip == "127.0.0.1" || ip.contains('.') || ip.contains(':'));
    }
    
    #[test]
    fn test_discover_gpus() {
        // On systems without nvidia-smi, this should return empty
        let gpus = NodeAgent::discover_gpus();
        // Verify return type is correct (empty vector is valid on non-GPU systems)
        let _ = gpus.len();  // Ensures gpus is a valid Vec
    }
    
    // ===================== Node Agent Tests =====================
    
    #[test]
    fn test_node_agent_creation() {
        let config = NodeAgentConfig {
            node_id: "test-agent".to_string(),
            scheduler_addr: "http://localhost:50051".to_string(),
            heartbeat_interval_secs: 30,
            gpu_monitor_interval_secs: 10,
        };
        
        let agent = NodeAgent::new(config);
        assert!(agent.is_ok());
        
        let agent = agent.unwrap();
        let status = agent.status();
        assert_eq!(status.id, "test-agent");
    }
    
    #[test]
    fn test_node_agent_stop() {
        let config = NodeAgentConfig::default();
        let mut agent = NodeAgent::new(config).unwrap();
        
        // Initially not running
        assert!(!agent.running);
        
        // Stop (even when not started)
        agent.stop();
        assert!(!agent.running);
    }
    
    #[test]
    fn test_node_agent_status() {
        let config = NodeAgentConfig {
            node_id: "status-test".to_string(),
            scheduler_addr: "http://localhost:50051".to_string(),
            heartbeat_interval_secs: 30,
            gpu_monitor_interval_secs: 10,
        };
        
        let agent = NodeAgent::new(config).unwrap();
        let status = agent.status();
        
        assert_eq!(status.id, "status-test");
        assert!(status.topology.cpu_cores > 0);
        assert!(status.topology.cpu_memory > 0);
    }
    
    #[test]
    fn test_discover_topology() {
        let topology = NodeAgent::discover_topology();
        assert!(topology.is_ok());
        
        let topology = topology.unwrap();
        assert!(topology.cpu_cores > 0);
        assert!(topology.cpu_memory > 0);
        assert!(topology.numa_nodes >= 1);
    }
    
    // ===================== Integration Tests =====================
    
    #[test]
    fn test_multiple_gpu_lines() {
        let lines = vec![
            "0, NVIDIA A100, GPU-0, 40960, 40000, 10, 40",
            "1, NVIDIA A100, GPU-1, 40960, 35000, 30, 45",
            "2, NVIDIA A100, GPU-2, 40960, 30000, 50, 50",
            "3, NVIDIA A100, GPU-3, 40960, 25000, 70, 55",
        ];
        
        let gpus: Vec<GpuDevice> = lines.iter()
            .filter_map(|l| NodeAgent::parse_gpu_line(l))
            .collect();
        
        assert_eq!(gpus.len(), 4);
        
        for (i, gpu) in gpus.iter().enumerate() {
            assert_eq!(gpu.device_id, format!("cuda:{}", i));
        }
    }
    
    #[test]
    fn test_config_roundtrip() {
        let original = NodeAgentConfig {
            node_id: "roundtrip-node".to_string(),
            scheduler_addr: "http://sched:9090".to_string(),
            heartbeat_interval_secs: 120,
            gpu_monitor_interval_secs: 60,
        };
        
        let json = serde_json::to_string(&original).unwrap();
        let restored: NodeAgentConfig = serde_json::from_str(&json).unwrap();
        
        assert_eq!(original.node_id, restored.node_id);
        assert_eq!(original.scheduler_addr, restored.scheduler_addr);
        assert_eq!(original.heartbeat_interval_secs, restored.heartbeat_interval_secs);
        assert_eq!(original.gpu_monitor_interval_secs, restored.gpu_monitor_interval_secs);
    }
}

