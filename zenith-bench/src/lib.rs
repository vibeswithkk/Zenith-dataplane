//! # Zenith Benchmark Harness
//!
//! MLPerf-style benchmarks for Zenith infrastructure.
//!
//! Copyright 2025 Wahyu Ardiansyah and Zenith AI Contributors

pub mod cpu;
pub mod synthetic;
pub mod report;

use clap::{Parser, Subcommand};
use std::time::Duration;

/// Zenith Benchmark Harness
#[derive(Parser)]
#[command(name = "zenith-bench")]
#[command(about = "Benchmark harness for Zenith infrastructure")]
pub struct Args {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Run CPU runtime benchmarks
    Cpu {
        /// Number of iterations
        #[arg(short, long, default_value = "1000")]
        iterations: usize,
    },
    /// Run ring buffer benchmarks
    RingBuffer {
        /// Buffer size
        #[arg(short, long, default_value = "65536")]
        size: usize,
    },
    /// Run full benchmark suite
    Full {
        /// Output file for results
        #[arg(short, long, default_value = "benchmark_results.json")]
        output: String,
    },
}

/// Benchmark result
#[derive(Debug, Clone, serde::Serialize)]
pub struct BenchmarkResult {
    pub name: String,
    pub iterations: usize,
    pub total_time_ms: f64,
    pub avg_time_us: f64,
    pub min_time_us: f64,
    pub max_time_us: f64,
    pub p50_time_us: f64,
    pub p95_time_us: f64,
    pub p99_time_us: f64,
    pub throughput_ops_sec: f64,
}

impl BenchmarkResult {
    /// Calculate from timings
    pub fn from_timings(name: &str, timings: &mut Vec<Duration>) -> Self {
        timings.sort();
        
        let iterations = timings.len();
        let total: Duration = timings.iter().sum();
        let total_ms = total.as_secs_f64() * 1000.0;
        
        let to_us = |d: Duration| d.as_secs_f64() * 1_000_000.0;
        
        let min = timings.first().map(|d| to_us(*d)).unwrap_or(0.0);
        let max = timings.last().map(|d| to_us(*d)).unwrap_or(0.0);
        let avg = total_ms * 1000.0 / iterations as f64;
        
        let p50 = to_us(timings[iterations / 2]);
        let p95 = to_us(timings[iterations * 95 / 100]);
        let p99 = to_us(timings[iterations * 99 / 100]);
        
        let throughput = (iterations as f64 * 1000.0) / total_ms;
        
        Self {
            name: name.to_string(),
            iterations,
            total_time_ms: total_ms,
            avg_time_us: avg,
            min_time_us: min,
            max_time_us: max,
            p50_time_us: p50,
            p95_time_us: p95,
            p99_time_us: p99,
            throughput_ops_sec: throughput,
        }
    }
    
    /// Print result
    pub fn print(&self) {
        println!("\n📊 {} Benchmark Results:", self.name);
        println!("  Iterations:     {:>12}", self.iterations);
        println!("  Total time:     {:>12.2} ms", self.total_time_ms);
        println!("  Avg latency:    {:>12.2} µs", self.avg_time_us);
        println!("  Min latency:    {:>12.2} µs", self.min_time_us);
        println!("  Max latency:    {:>12.2} µs", self.max_time_us);
        println!("  P50 latency:    {:>12.2} µs", self.p50_time_us);
        println!("  P95 latency:    {:>12.2} µs", self.p95_time_us);
        println!("  P99 latency:    {:>12.2} µs", self.p99_time_us);
        println!("  Throughput:     {:>12.0} ops/sec", self.throughput_ops_sec);
    }
}
