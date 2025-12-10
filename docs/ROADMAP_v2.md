# Zenith Dataplane - Development Roadmap

> **Status:** Private Development Phase
> **Last Updated:** 2025-12-09
## Vision
**"Making ML infrastructure faster, cheaper, and simpler — without replacing what works."**

Zenith is not a replacement for PyTorch, JAX, or TensorFlow. It's an **acceleration layer** that makes them run faster with less resources.
---
## Core Value Proposition

```python
import zenith
import torch
# That's it. Your ML infrastructure is now:
# [OK] 2-5x faster data loading
# [OK] 30-50% cost reduction
# [OK] Ultra-low latency
# [OK] Simple job scheduling (SLURM alternative)
```
---
## Development Phases
### Phase 1: Foundation (Current)
- [x] Core runtime architecture
- [x] Lock-free ring buffers (SPSC/MPMC)
- [x] Memory pool implementation
- [x] NUMA-aware allocation
- [ ] **Fix critical bugs (in progress)**
- [ ] Stabilize existing code
### Phase 2: Fast DataLoader
- [ ] PyTorch DataLoader drop-in replacement
- [ ] Zero-copy data transfers
- [ ] WASM preprocessing plugins
- [ ] Benchmarks vs PyTorch native
### Phase 3: Simple Scheduler
- [ ] `@zenith.job` decorator
- [ ] Local execution mode
- [ ] Cluster submission (`zenith.submit`)
- [ ] Job monitoring (`zenith.status`)
### Phase 4: Integration & Polish
- [ ] JAX integration
- [ ] TensorFlow integration
- [ ] Documentation & tutorials
- [ ] Public release
---
## Known Issues (Being Fixed)

| Issue | Status | Priority |
|-------|--------|----------|
| ~~`todo!()` in io_uring~~ | [OK] Fixed | Critical |
| Panic across FFI boundary | Pending | Critical |
| Zombie jobs in scheduler | Pending | High |
| GPU runtime placeholders | Planned | Medium |
---
## Target Metrics

| Metric | Target | Current |
|--------|--------|---------|
| Data loading speedup | 2-5x | TBD |
| Memory efficiency | +40% | TBD |
| Setup time | < 5 min | TBD |
| Test coverage | > 80% | ~30% |
---
## Architecture

```

 User Code 
 (PyTorch, JAX, TensorFlow) 

 ZENITH PYTHON SDK 
 
 DataLoader @job Transforms 
 API Decorator (WASM) 
 

 ZENITH CORE (Rust) 
 
 CPU Runtime Scheduler GPU Runtime 
 (buffer, (jobs, (CUDA, 
 pool, io) queues) TensorRT) 
 

 Hardware 
 (CPU, GPU, Storage, Network) 

```
---
## Contributing

This project is currently in private development. Once stabilized, we will open for contributions.
---
## License

Apache License 2.0
