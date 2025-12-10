//! NUMA-Aware Memory Topology Discovery
//!
//! This module provides functionality to discover and query the NUMA
//! (Non-Uniform Memory Access) topology of the system.

use crate::{Error, Result};
use std::collections::HashMap;
use tracing::{debug, info, warn};

/// Represents a NUMA node in the system
#[derive(Debug, Clone)]
pub struct NumaNode {
    /// Node ID (0-indexed)
    pub node_id: u32,
    /// CPU cores belonging to this node
    pub cpu_cores: Vec<u32>,
    /// Total memory in bytes
    pub total_memory: u64,
    /// Free memory in bytes
    pub free_memory: u64,
    /// Whether hugepages are available
    pub hugepages_available: bool,
    /// Number of free hugepages
    pub hugepages_free: u64,
    /// Hugepage size in bytes
    pub hugepage_size: u64,
}

/// System NUMA topology
#[derive(Debug, Clone)]
pub struct NumaTopology {
    /// Map of node ID to node info
    nodes: HashMap<u32, NumaNode>,
    /// Total number of NUMA nodes
    num_nodes: u32,
    /// Total number of CPU cores
    num_cpus: u32,
    /// Whether NUMA is actually available
    numa_available: bool,
}

impl NumaTopology {
    /// Discover the system's NUMA topology
    pub fn discover() -> Result<Self> {
        info!("Discovering NUMA topology...");
        
        // Check if NUMA is available
        let numa_available = Self::check_numa_available();
        
        if !numa_available {
            warn!("NUMA not available on this system, using single-node fallback");
            return Ok(Self::single_node_fallback());
        }
        
        let nodes = Self::discover_nodes()?;
        let num_nodes = nodes.len() as u32;
        let num_cpus = nodes.values()
            .map(|n| n.cpu_cores.len() as u32)
            .sum();
        
        info!(
            "Discovered {} NUMA nodes with {} total CPUs",
            num_nodes, num_cpus
        );
        
        Ok(Self {
            nodes,
            num_nodes,
            num_cpus,
            numa_available,
        })
    }
    
    /// Check if NUMA is available on this system
    fn check_numa_available() -> bool {
        std::path::Path::new("/sys/devices/system/node/node0").exists()
    }
    
    /// Discover all NUMA nodes
    fn discover_nodes() -> Result<HashMap<u32, NumaNode>> {
        let mut nodes = HashMap::new();
        
        let node_base = std::path::Path::new("/sys/devices/system/node");
        
        for entry in std::fs::read_dir(node_base)
            .map_err(|e| Error::Numa(format!("Failed to read NUMA nodes: {}", e)))?
        {
            let entry = entry.map_err(|e| Error::Numa(e.to_string()))?;
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            
            if !name_str.starts_with("node") {
                continue;
            }
            
            if let Ok(node_id) = name_str.strip_prefix("node")
                .unwrap_or("")
                .parse::<u32>()
            {
                let node = Self::read_node_info(node_id, &entry.path())?;
                debug!("Discovered NUMA node {}: {:?}", node_id, node);
                nodes.insert(node_id, node);
            }
        }
        
        Ok(nodes)
    }
    
    /// Read detailed info about a NUMA node
    fn read_node_info(node_id: u32, node_path: &std::path::Path) -> Result<NumaNode> {
        // Read CPU list
        let cpulist_path = node_path.join("cpulist");
        let cpu_cores = if cpulist_path.exists() {
            Self::parse_cpulist(&std::fs::read_to_string(&cpulist_path)
                .unwrap_or_default())
        } else {
            vec![]
        };
        
        // Read memory info
        let meminfo_path = node_path.join("meminfo");
        let (total_memory, free_memory) = if meminfo_path.exists() {
            Self::parse_meminfo(&std::fs::read_to_string(&meminfo_path)
                .unwrap_or_default())
        } else {
            (0, 0)
        };
        
        // Read hugepage info
        let hugepages_path = node_path.join("hugepages/hugepages-2048kB");
        let (hugepages_available, hugepages_free, hugepage_size) = 
            if hugepages_path.exists() {
                let free = std::fs::read_to_string(hugepages_path.join("free_hugepages"))
                    .unwrap_or_default()
                    .trim()
                    .parse::<u64>()
                    .unwrap_or(0);
                (free > 0, free, 2 * 1024 * 1024) // 2MB hugepages
            } else {
                (false, 0, 0)
            };
        
        Ok(NumaNode {
            node_id,
            cpu_cores,
            total_memory,
            free_memory,
            hugepages_available,
            hugepages_free,
            hugepage_size,
        })
    }
    
    /// Parse a CPU list string like "0-3,8-11"
    fn parse_cpulist(list: &str) -> Vec<u32> {
        let mut cpus = Vec::new();
        
        for part in list.trim().split(',') {
            if part.contains('-') {
                let bounds: Vec<&str> = part.split('-').collect();
                if bounds.len() == 2 {
                    if let (Ok(start), Ok(end)) = (
                        bounds[0].parse::<u32>(),
                        bounds[1].parse::<u32>(),
                    ) {
                        cpus.extend(start..=end);
                    }
                }
            } else if let Ok(cpu) = part.parse::<u32>() {
                cpus.push(cpu);
            }
        }
        
        cpus
    }
    
    /// Parse memory info from NUMA node
    fn parse_meminfo(content: &str) -> (u64, u64) {
        let mut total = 0u64;
        let mut free = 0u64;
        
        for line in content.lines() {
            if line.contains("MemTotal:") {
                if let Some(value) = Self::extract_kb_value(line) {
                    total = value * 1024;
                }
            } else if line.contains("MemFree:") {
                if let Some(value) = Self::extract_kb_value(line) {
                    free = value * 1024;
                }
            }
        }
        
        (total, free)
    }
    
    /// Extract KB value from a line like "Node 0 MemTotal:    12345678 kB"
    fn extract_kb_value(line: &str) -> Option<u64> {
        line.split_whitespace().find(|s| s.chars().all(|c| c.is_ascii_digit()))
            .and_then(|s| s.parse().ok())
    }
    
    /// Create a single-node fallback for non-NUMA systems
    fn single_node_fallback() -> Self {
        let sys_info = sysinfo::System::new_all();
        let num_cpus = sys_info.cpus().len() as u32;
        
        let node = NumaNode {
            node_id: 0,
            cpu_cores: (0..num_cpus).collect(),
            total_memory: sys_info.total_memory(),
            free_memory: sys_info.available_memory(),
            hugepages_available: false,
            hugepages_free: 0,
            hugepage_size: 0,
        };
        
        let mut nodes = HashMap::new();
        nodes.insert(0, node);
        
        Self {
            nodes,
            num_nodes: 1,
            num_cpus,
            numa_available: false,
        }
    }
    
    // Public API
    
    /// Get the number of NUMA nodes
    pub fn num_nodes(&self) -> u32 {
        self.num_nodes
    }
    
    /// Get the total number of CPUs
    pub fn num_cpus(&self) -> u32 {
        self.num_cpus
    }
    
    /// Check if NUMA is available
    pub fn is_numa_available(&self) -> bool {
        self.numa_available
    }
    
    /// Get a specific NUMA node
    pub fn get_node(&self, node_id: u32) -> Option<&NumaNode> {
        self.nodes.get(&node_id)
    }
    
    /// Get all NUMA nodes
    pub fn nodes(&self) -> impl Iterator<Item = &NumaNode> {
        self.nodes.values()
    }
    
    /// Get CPUs for a specific NUMA node
    pub fn cpus_for_node(&self, node_id: u32) -> Option<&[u32]> {
        self.nodes.get(&node_id).map(|n| n.cpu_cores.as_slice())
    }
    
    /// Find the NUMA node for a given CPU
    pub fn node_for_cpu(&self, cpu: u32) -> Option<u32> {
        for (node_id, node) in &self.nodes {
            if node.cpu_cores.contains(&cpu) {
                return Some(*node_id);
            }
        }
        None
    }
    
    /// Get the node with the most free memory
    pub fn node_with_most_free_memory(&self) -> Option<u32> {
        self.nodes.iter()
            .max_by_key(|(_, node)| node.free_memory)
            .map(|(id, _)| *id)
    }
    
    /// Get total system memory
    pub fn total_memory(&self) -> u64 {
        self.nodes.values().map(|n| n.total_memory).sum()
    }
    
    /// Get total free memory
    pub fn total_free_memory(&self) -> u64 {
        self.nodes.values().map(|n| n.free_memory).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_cpulist() {
        assert_eq!(
            NumaTopology::parse_cpulist("0-3"),
            vec![0, 1, 2, 3]
        );
        
        assert_eq!(
            NumaTopology::parse_cpulist("0-3,8-11"),
            vec![0, 1, 2, 3, 8, 9, 10, 11]
        );
        
        assert_eq!(
            NumaTopology::parse_cpulist("0,2,4,6"),
            vec![0, 2, 4, 6]
        );
    }
    
    #[test]
    fn test_topology_discovery() {
        // This will use fallback on most development machines
        let topology = NumaTopology::discover().unwrap();
        assert!(topology.num_cpus() > 0);
        assert!(topology.num_nodes() >= 1);
    }
}
