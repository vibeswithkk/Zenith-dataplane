//! Zenith Benchmark - Main Entry Point

use clap::Parser;
use zenith_bench::{Args, BenchmarkResult, Commands};
use zenith_runtime_cpu::buffer::{RingBuffer, SpscRingBuffer};
use std::time::Instant;

fn main() -> anyhow::Result<()> {
 tracing_subscriber::fmt::init();
 
 let args = Args::parse();
 
 println!("");
 println!(" ZENITH BENCHMARK HARNESS ");
 println!("");
 println!(" Version: {}", zenith_runtime_cpu::VERSION);
 println!(" Date: {}", chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC"));
 println!("");
 
 match args.command {
 Commands::Cpu { iterations } => {
 run_cpu_benchmarks(iterations)?;
 }
 Commands::RingBuffer { size } => {
 run_ringbuffer_benchmarks(size)?;
 }
 Commands::Full { output } => {
 let results = run_full_suite()?;
 
 // Save results
 let json = serde_json::to_string_pretty(&results)?;
 std::fs::write(&output, &json)?;
 println!("\n Results saved to: {}", output);
 }
 }
 
 println!("\n");
 println!(" BENCHMARK COMPLETE ");
 println!("");
 
 Ok(())
}

fn run_cpu_benchmarks(iterations: usize) -> anyhow::Result<()> {
 println!("\n Running CPU Runtime Benchmarks...\n");
 
 // NUMA topology discovery benchmark
 let mut timings = Vec::with_capacity(iterations);
 for _ in 0..iterations {
 let start = Instant::now();
 let _ = zenith_runtime_cpu::NumaTopology::discover();
 timings.push(start.elapsed());
 }
 
 let result = BenchmarkResult::from_timings("NUMA Topology Discovery", &mut timings);
 result.print();
 
 Ok(())
}

fn run_ringbuffer_benchmarks(size: usize) -> anyhow::Result<()> {
 println!("\n Running Ring Buffer Benchmarks...\n");
 
 let buffer = SpscRingBuffer::<u64>::new(size);
 let iterations = size * 10;
 
 // Push benchmark
 let mut timings = Vec::with_capacity(iterations);
 for i in 0..iterations {
 let start = Instant::now();
 let _ = buffer.try_push(i as u64);
 timings.push(start.elapsed());
 
 // Pop to prevent full buffer
 if i % 2 == 1 {
 let _ = buffer.try_pop();
 }
 }
 
 let result = BenchmarkResult::from_timings("SPSC Ring Buffer Push", &mut timings);
 result.print();
 
 // Pop benchmark
 let mut timings = Vec::with_capacity(iterations);
 for _ in 0..iterations {
 if buffer.try_push(42).is_err() {
 let _ = buffer.try_pop();
 }
 
 let start = Instant::now();
 let _ = buffer.try_pop();
 timings.push(start.elapsed());
 }
 
 let result = BenchmarkResult::from_timings("SPSC Ring Buffer Pop", &mut timings);
 result.print();
 
 Ok(())
}

fn run_full_suite() -> anyhow::Result<Vec<BenchmarkResult>> {
 println!("\n Running Full Benchmark Suite...\n");
 
 let results = Vec::new();
 
 // CPU benchmarks
 run_cpu_benchmarks(1000)?;
 
 // Ring buffer benchmarks
 run_ringbuffer_benchmarks(65536)?;
 
 Ok(results)
}
