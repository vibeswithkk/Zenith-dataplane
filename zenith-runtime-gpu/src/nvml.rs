//! NVML-like GPU Management Interface
//!
//! Abstraction layer for NVIDIA Management Library operations.

use serde::{Deserialize, Serialize};

use crate::{Error, Result};

/// GPU power state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PowerState {
    /// Maximum performance
    P0,
    /// High performance
    P1,
    /// Medium performance
    P2,
    /// Idle
    P8,
    /// Unknown
    Unknown,
}

/// GPU memory info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryInfo {
    /// Total memory in bytes
    pub total: u64,
    /// Used memory in bytes  
    pub used: u64,
    /// Free memory in bytes
    pub free: u64,
}

/// GPU utilization info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UtilizationInfo {
    /// GPU compute utilization (0-100)
    pub gpu: u32,
    /// Memory controller utilization (0-100)
    pub memory: u32,
    /// Encoder utilization (0-100)
    pub encoder: u32,
    /// Decoder utilization (0-100)
    pub decoder: u32,
}

/// GPU clock info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClockInfo {
    /// Graphics clock in MHz
    pub graphics: u32,
    /// SM clock in MHz
    pub sm: u32,
    /// Memory clock in MHz
    pub memory: u32,
    /// Video clock in MHz
    pub video: u32,
}

/// GPU temperature info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemperatureInfo {
    /// GPU temperature in Celsius
    pub gpu: i32,
    /// Memory temperature in Celsius (if available)
    pub memory: Option<i32>,
    /// Slowdown threshold
    pub slowdown_threshold: i32,
    /// Shutdown threshold
    pub shutdown_threshold: i32,
}

/// PCIe link info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PcieInfo {
    /// Current generation (1-5)
    pub generation: u32,
    /// Current link width (x1, x4, x8, x16)
    pub width: u32,
    /// Maximum generation
    pub max_generation: u32,
    /// Maximum width
    pub max_width: u32,
    /// TX throughput in KB/s
    pub tx_throughput: u64,
    /// RX throughput in KB/s
    pub rx_throughput: u64,
}

/// NVLink status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NvlinkStatus {
    /// NVLink version
    pub version: u32,
    /// Number of active links
    pub active_links: u32,
    /// Bandwidth per link in GB/s
    pub bandwidth_per_link: f32,
    /// Connected GPU indices
    pub connected_gpus: Vec<u32>,
}

/// ECC memory stats
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EccStats {
    /// ECC enabled
    pub enabled: bool,
    /// Single bit errors (correctable)
    pub single_bit_errors: u64,
    /// Double bit errors (uncorrectable)
    pub double_bit_errors: u64,
}

/// Comprehensive GPU device info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuInfo {
    /// Device index
    pub index: u32,
    /// Device name
    pub name: String,
    /// UUID
    pub uuid: String,
    /// Serial number
    pub serial: Option<String>,
    /// VBIOS version
    pub vbios_version: String,
    /// Driver version
    pub driver_version: String,
    /// CUDA compute capability
    pub compute_capability: (u32, u32),
    /// Number of SMs
    pub sm_count: u32,
    /// Power state
    pub power_state: PowerState,
    /// Power limit in watts
    pub power_limit: u32,
    /// Current power draw in watts
    pub power_draw: u32,
    /// Memory info
    pub memory: MemoryInfo,
    /// Utilization
    pub utilization: UtilizationInfo,
    /// Clocks
    pub clocks: ClockInfo,
    /// Temperature
    pub temperature: TemperatureInfo,
    /// PCIe info
    pub pcie: PcieInfo,
    /// NVLink status
    pub nvlink: Option<NvlinkStatus>,
    /// ECC stats
    pub ecc: EccStats,
}

/// NVML-like GPU management interface
pub struct NvmlManager {
    #[allow(dead_code)]
    initialized: bool,
    gpu_count: u32,
}

impl NvmlManager {
    /// Initialize NVML
    pub fn new() -> Result<Self> {
        // In production: Call nvmlInit()
        // For now, we'll detect GPUs via nvidia-smi
        
        let gpu_count = Self::detect_gpu_count();
        
        Ok(Self {
            initialized: true,
            gpu_count,
        })
    }
    
    /// Detect GPU count
    fn detect_gpu_count() -> u32 {
        match std::process::Command::new("nvidia-smi")
            .args(["--query-gpu=index", "--format=csv,noheader"])
            .output()
        {
            Ok(output) if output.status.success() => {
                String::from_utf8_lossy(&output.stdout)
                    .lines()
                    .count() as u32
            }
            _ => 0,
        }
    }
    
    /// Get GPU count
    pub fn gpu_count(&self) -> u32 {
        self.gpu_count
    }
    
    /// Get GPU info for a specific device
    pub fn get_gpu_info(&self, index: u32) -> Result<GpuInfo> {
        if index >= self.gpu_count {
            return Err(Error::Gpu(format!("Invalid GPU index: {}", index)));
        }
        
        // Query nvidia-smi for detailed info
        let output = std::process::Command::new("nvidia-smi")
            .args([
                "--query-gpu=index,name,uuid,vbios_version,driver_version,pstate,power.limit,power.draw,memory.total,memory.used,memory.free,utilization.gpu,utilization.memory,clocks.gr,clocks.sm,clocks.mem,clocks.video,temperature.gpu,pcie.link.gen.current,pcie.link.width.current,pcie.link.gen.max,pcie.link.width.max",
                "--format=csv,noheader,nounits",
                &format!("--id={}", index),
            ])
            .output()
            .map_err(|e| Error::Gpu(format!("Failed to run nvidia-smi: {}", e)))?;
        
        if !output.status.success() {
            return Err(Error::Gpu("nvidia-smi failed".to_string()));
        }
        
        let line = String::from_utf8_lossy(&output.stdout);
        let parts: Vec<&str> = line.trim().split(',').map(|s| s.trim()).collect();
        
        if parts.len() < 22 {
            return Err(Error::Gpu("Unexpected nvidia-smi output".to_string()));
        }
        
        let power_state = match parts[5] {
            "P0" => PowerState::P0,
            "P1" => PowerState::P1,
            "P2" => PowerState::P2,
            "P8" => PowerState::P8,
            _ => PowerState::Unknown,
        };
        
        Ok(GpuInfo {
            index,
            name: parts[1].to_string(),
            uuid: parts[2].to_string(),
            serial: None,
            vbios_version: parts[3].to_string(),
            driver_version: parts[4].to_string(),
            compute_capability: (8, 0), // Would need CUDA API
            sm_count: 108, // Would need CUDA API
            power_state,
            power_limit: parts[6].parse().unwrap_or(0),
            power_draw: parts[7].parse::<f32>().unwrap_or(0.0) as u32,
            memory: MemoryInfo {
                total: parts[8].parse::<u64>().unwrap_or(0) * 1024 * 1024,
                used: parts[9].parse::<u64>().unwrap_or(0) * 1024 * 1024,
                free: parts[10].parse::<u64>().unwrap_or(0) * 1024 * 1024,
            },
            utilization: UtilizationInfo {
                gpu: parts[11].parse().unwrap_or(0),
                memory: parts[12].parse().unwrap_or(0),
                encoder: 0,
                decoder: 0,
            },
            clocks: ClockInfo {
                graphics: parts[13].parse().unwrap_or(0),
                sm: parts[14].parse().unwrap_or(0),
                memory: parts[15].parse().unwrap_or(0),
                video: parts[16].parse().unwrap_or(0),
            },
            temperature: TemperatureInfo {
                gpu: parts[17].parse().unwrap_or(0),
                memory: None,
                slowdown_threshold: 83,
                shutdown_threshold: 90,
            },
            pcie: PcieInfo {
                generation: parts[18].parse().unwrap_or(0),
                width: parts[19].parse().unwrap_or(0),
                max_generation: parts[20].parse().unwrap_or(0),
                max_width: parts[21].parse().unwrap_or(0),
                tx_throughput: 0,
                rx_throughput: 0,
            },
            nvlink: None, // Would need NVML
            ecc: EccStats {
                enabled: false,
                single_bit_errors: 0,
                double_bit_errors: 0,
            },
        })
    }
    
    /// Get all GPU info
    pub fn get_all_gpus(&self) -> Vec<GpuInfo> {
        (0..self.gpu_count)
            .filter_map(|i| self.get_gpu_info(i).ok())
            .collect()
    }
    
    /// Set power limit for a GPU
    pub fn set_power_limit(&self, index: u32, watts: u32) -> Result<()> {
        if index >= self.gpu_count {
            return Err(Error::Gpu(format!("Invalid GPU index: {}", index)));
        }
        
        let status = std::process::Command::new("nvidia-smi")
            .args([
                &format!("--id={}", index),
                &format!("--power-limit={}", watts),
            ])
            .status()
            .map_err(|e| Error::Gpu(format!("Failed to set power limit: {}", e)))?;
        
        if status.success() {
            Ok(())
        } else {
            Err(Error::Gpu("Failed to set power limit".to_string()))
        }
    }
    
    /// Reset GPU
    pub fn reset_gpu(&self, index: u32) -> Result<()> {
        if index >= self.gpu_count {
            return Err(Error::Gpu(format!("Invalid GPU index: {}", index)));
        }
        
        let status = std::process::Command::new("nvidia-smi")
            .args([
                &format!("--id={}", index),
                "--gpu-reset",
            ])
            .status()
            .map_err(|e| Error::Gpu(format!("Failed to reset GPU: {}", e)))?;
        
        if status.success() {
            Ok(())
        } else {
            Err(Error::Gpu("Failed to reset GPU".to_string()))
        }
    }
}

impl Default for NvmlManager {
    fn default() -> Self {
        Self::new().unwrap_or(Self {
            initialized: false,
            gpu_count: 0,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_nvml_manager() {
        let manager = NvmlManager::default();
        // GPU count depends on hardware
        println!("Detected {} GPUs", manager.gpu_count());
    }
    
    #[test]
    fn test_power_state() {
        assert_eq!(PowerState::P0, PowerState::P0);
        assert_ne!(PowerState::P0, PowerState::P8);
    }
}
