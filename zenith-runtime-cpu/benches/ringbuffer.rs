//! CPU Runtime Benchmarks
//!
//! Criterion-based benchmarks for CPU runtime components.

use criterion::{criterion_group, criterion_main, Criterion, BenchmarkId, Throughput};
use std::sync::Arc;
use std::thread;

fn bench_spsc_ringbuffer(c: &mut Criterion) {
    use zenith_runtime_cpu::buffer::{SpscRingBuffer, RingBuffer};
    
    let mut group = c.benchmark_group("spsc_ringbuffer");
    
    for size in [1024, 4096, 16384, 65536].iter() {
        group.throughput(Throughput::Elements(*size as u64));
        
        group.bench_with_input(
            BenchmarkId::new("push_pop", size),
            size,
            |b, &size| {
                let buffer = SpscRingBuffer::<u64>::new(size);
                
                b.iter(|| {
                    for i in 0..size {
                        let _ = buffer.try_push(i as u64);
                    }
                    for _ in 0..size {
                        let _ = buffer.try_pop();
                    }
                });
            },
        );
    }
    
    group.finish();
}

fn bench_spsc_concurrent(c: &mut Criterion) {
    use zenith_runtime_cpu::buffer::{SpscRingBuffer, RingBuffer};
    
    let mut group = c.benchmark_group("spsc_concurrent");
    
    for items in [10_000, 100_000, 1_000_000].iter() {
        group.throughput(Throughput::Elements(*items as u64));
        
        group.bench_with_input(
            BenchmarkId::new("producer_consumer", items),
            items,
            |b, &items| {
                b.iter(|| {
                    let buffer = Arc::new(SpscRingBuffer::<u64>::new(65536));
                    let buf_producer = Arc::clone(&buffer);
                    let buf_consumer = Arc::clone(&buffer);
                    
                    let producer = thread::spawn(move || {
                        for i in 0..items {
                            while buf_producer.try_push(i as u64).is_err() {
                                std::hint::spin_loop();
                            }
                        }
                    });
                    
                    let consumer = thread::spawn(move || {
                        let mut received = 0;
                        while received < items {
                            if buf_consumer.try_pop().is_some() {
                                received += 1;
                            } else {
                                std::hint::spin_loop();
                            }
                        }
                    });
                    
                    producer.join().unwrap();
                    consumer.join().unwrap();
                });
            },
        );
    }
    
    group.finish();
}

fn bench_memory_pool(c: &mut Criterion) {
    use zenith_runtime_cpu::pool::{MemoryPool, PoolConfig};
    
    let mut group = c.benchmark_group("memory_pool");
    
    for slab_size in [4096, 16384, 65536].iter() {
        group.bench_with_input(
            BenchmarkId::new("allocate_deallocate", slab_size),
            slab_size,
            |b, &slab_size| {
                let config = PoolConfig {
                    slab_size,
                    initial_slabs: 64,
                    max_slabs: 256,
                    alignment: 64,
                };
                let pool = MemoryPool::new(config).unwrap();
                
                b.iter(|| {
                    let buf = pool.allocate().unwrap();
                    pool.deallocate(buf);
                });
            },
        );
    }
    
    group.finish();
}

fn bench_telemetry(c: &mut Criterion) {
    use zenith_runtime_cpu::telemetry::TelemetryCollector;
    
    let mut group = c.benchmark_group("telemetry");
    
    group.bench_function("record_event", |b| {
        let collector = TelemetryCollector::new(1000);
        
        b.iter(|| {
            collector.record_event(1024);
        });
    });
    
    group.bench_function("record_latency", |b| {
        let collector = TelemetryCollector::new(1000);
        
        b.iter(|| {
            collector.record_latency(50);
        });
    });
    
    group.bench_function("snapshot", |b| {
        let collector = TelemetryCollector::new(1000);
        for _ in 0..1000 {
            collector.record_event(1024);
        }
        
        b.iter(|| {
            collector.snapshot()
        });
    });
    
    group.finish();
}

criterion_group!(
    benches,
    bench_spsc_ringbuffer,
    bench_spsc_concurrent,
    bench_memory_pool,
    bench_telemetry,
);

criterion_main!(benches);
