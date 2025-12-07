//! NUMA Allocator Benchmarks

use criterion::{criterion_group, criterion_main, Criterion, BenchmarkId, Throughput};

fn bench_numa_topology(c: &mut Criterion) {
    use zenith_runtime_cpu::numa::NumaTopology;
    
    let mut group = c.benchmark_group("numa");
    
    group.bench_function("discover", |b| {
        b.iter(|| {
            NumaTopology::discover()
        });
    });
    
    group.bench_function("num_cpus", |b| {
        let topology = NumaTopology::discover().unwrap();
        b.iter(|| {
            topology.num_cpus()
        });
    });
    
    group.finish();
}

fn bench_allocator(c: &mut Criterion) {
    use zenith_runtime_cpu::allocator::NumaAllocator;
    
    let mut group = c.benchmark_group("allocator");
    
    for size in [4096, 65536, 1048576].iter() {
        group.throughput(Throughput::Bytes(*size as u64));
        
        group.bench_with_input(
            BenchmarkId::new("allocate_free", size),
            size,
            |b, &size| {
                let allocator = NumaAllocator::new();
                
                b.iter(|| {
                    if let Ok(ptr) = allocator.allocate(size, 64) {
                        unsafe { allocator.deallocate(ptr, size, 64) };
                    }
                });
            },
        );
    }
    
    group.finish();
}

criterion_group!(
    benches,
    bench_numa_topology,
    bench_allocator,
);

criterion_main!(benches);
