# Zenith Infrastructure - Tutorial Lengkap

**Panduan Komprehensif untuk Memahami dan Menggunakan Zenith**

---

## Daftar Isi

1. [Pendahuluan](#1-pendahuluan)
2. [Cara Kerja Zenith](#2-cara-kerja-zenith)
3. [Instalasi](#3-instalasi)
4. [Penggunaan Dasar](#4-penggunaan-dasar)
5. [Arsitektur Internal](#5-arsitektur-internal)
6. [Pengembangan Plugin](#6-pengembangan-plugin)
7. [Deployment](#7-deployment)
8. [Troubleshooting](#8-troubleshooting)

---

## 1. Pendahuluan

### 1.1 Apa itu Zenith?

Zenith adalah ekosistem infrastruktur performa tinggi yang dirancang untuk mempercepat training dan inference AI/ML. Zenith bukan pengganti PyTorch atau TensorFlow, melainkan **layer performa** yang bekerja di bawah framework ML Anda.

### 1.2 Masalah yang Diselesaikan

**Masalah Utama:** GPU mahal seperti H100 sering "menganggur" menunggu data dari CPU.

```
Tanpa Zenith:
CPU (Python) --slow--> GPU
                       |
                   GPU: [IDLE] "Menunggu data..."
                       |
                   Utilisasi: 60%

Dengan Zenith:
Zenith (Rust) --fast--> GPU
                        |
                    GPU: [ACTIVE] "Data terus mengalir!"
                        |
                    Utilisasi: 95%+
```

### 1.3 Komponen Utama

1. **Zenith CPU Engine** - Runtime CPU ultra-cepat dengan NUMA awareness
2. **Zenith GPU Runtime** - Optimasi GPU dengan kernel selection dan memory offload
3. **Zenith Scheduler** - Scheduler ringan mirip Slurm dengan gang scheduling

---

## 2. Cara Kerja Zenith

### 2.1 CPU Engine

#### NUMA-Aware Memory

```
Sistem 2-Socket:

Socket 0 (NUMA Node 0)          Socket 1 (NUMA Node 1)
┌─────────────────────┐         ┌─────────────────────┐
│ CPU 0-15            │         │ CPU 16-31           │
│ RAM: 256GB          │───QPI───│ RAM: 256GB          │
│ GPU 0, GPU 1        │ (slow)  │ GPU 2, GPU 3        │
└─────────────────────┘         └─────────────────────┘

Strategi Zenith:
- Alokasikan memori di NUMA node yang sama dengan GPU
- Pin thread ke core yang dekat dengan memori
- Gunakan hugepages untuk mengurangi TLB miss
```

#### Lock-Free Ring Buffer

```rust
// Producer (Thread 1)          // Consumer (Thread 2)
buffer.try_push(data);          let data = buffer.try_pop();

// Tidak ada lock!
// Tidak ada contention!
// Throughput: 22 juta ops/detik
```

### 2.2 GPU Runtime

#### Kernel Selection

```
Operasi: MatMul(A, B) dimana A: 4096x4096, B: 4096x4096

Zenith melakukan:
1. Cek cache benchmark
2. Jika miss, jalankan micro-benchmark:
   - CUDA kernel: 1.2ms
   - Triton kernel: 1.5ms
   - TVM kernel: 1.4ms
   - CPU fallback: 150ms
3. Pilih CUDA kernel (tercepat)
4. Cache hasil untuk penggunaan selanjutnya
```

#### ZeRO Memory Offload

```
GPU VRAM (80GB)
├── Aktivasi (hot) - 40GB
├── Gradien (hot) - 20GB
└── Parameter (mixed) - 20GB
    ├── Layer 1-10 (hot) - di GPU
    └── Layer 11-100 (cold) - di CPU RAM

CPU RAM (512GB)
├── Parameter cold - 50GB
└── Prefetch buffer - 10GB

NVMe (4TB)
└── Checkpoints & optimizer states
```

### 2.3 Job Scheduler

#### Gang Scheduling

```
Job A: Butuh 8 GPU untuk distributed training

Tanpa Gang Scheduling:
- GPU 0-3 dialokasikan di Node 1
- GPU 4-7 menunggu di queue
- Job tidak bisa mulai!
- Waste time: 30 menit

Dengan Gang Scheduling:
- Tunggu sampai semua 8 GPU tersedia
- Alokasikan sekaligus
- Job langsung mulai!
- Waktu tunggu: 5 menit
```

---

## 3. Instalasi

### 3.1 Python SDK (Paling Mudah)

```bash
pip install zenith-ai
```

### 3.2 Dari Source Code

```bash
# Clone repository
git clone https://github.com/vibeswithkk/Zenith-dataplane.git
cd Zenith-dataplane

# Build semua crate
cargo build --release

# Jalankan tests
cargo test --all

# Build Python wheel
cd sdk-python
pip install maturin
maturin develop
```

### 3.3 Verifikasi Instalasi

```python
import zenith

print(f"Zenith version: {zenith.__version__}")
print(f"Native extension: {zenith.is_available()}")
```

---

## 4. Penggunaan Dasar

### 4.1 Data Loading dengan PyTorch

```python
import zenith.torch as zt
import torch

# Buat DataLoader high-performance
loader = zt.DataLoader(
    source="path/to/training_data",
    batch_size=64,
    shuffle=True,
    num_workers=4,
    pin_memory=True,
    prefetch_factor=2
)

# Training loop
model = torch.nn.Linear(100, 10).cuda()
optimizer = torch.optim.Adam(model.parameters())

for epoch in range(10):
    for batch in loader:  # Data mengalir cepat!
        inputs, labels = batch
        outputs = model(inputs.cuda())
        loss = torch.nn.functional.cross_entropy(outputs, labels.cuda())
        
        optimizer.zero_grad()
        loss.backward()
        optimizer.step()
```

### 4.2 Menggunakan WASM Plugins

```python
import zenith

# Inisialisasi engine
engine = zenith.Engine(buffer_size=4096)

# Load preprocessing plugin
engine.load_plugin("image_resize.wasm")

# Proses data dengan plugin
with engine:
    for batch in data:
        processed = engine.process(batch)
        # processed sudah di-resize oleh plugin!
```

### 4.3 TensorFlow Integration

```python
import zenith.tensorflow as ztf
import tensorflow as tf

dataset = ztf.ZenithDataset(
    source="path/to/data",
    batch_size=32
)

# Gunakan dengan tf.data pipeline
dataset = dataset.prefetch(tf.data.AUTOTUNE)

model.fit(dataset, epochs=10)
```

---

## 5. Arsitektur Internal

### 5.1 Struktur Direktori

```
zenith-dataplane/
├── zenith-runtime-cpu/     # CPU Engine
│   └── src/
│       ├── lib.rs          # Main entry point
│       ├── buffer.rs       # Ring buffers
│       ├── numa.rs         # NUMA discovery
│       ├── allocator.rs    # Memory allocator
│       ├── thread.rs       # Thread pinning
│       ├── io.rs           # io_uring
│       ├── telemetry.rs    # Metrics
│       ├── config.rs       # Configuration
│       └── engine.rs       # Main engine
│
├── zenith-runtime-gpu/     # GPU Runtime
│   └── src/
│       ├── lib.rs
│       ├── device.rs       # GPU discovery
│       ├── kernel.rs       # Kernel manager
│       ├── memory.rs       # Memory offload
│       ├── collective.rs   # NCCL
│       └── config.rs
│
├── zenith-scheduler/       # Job Scheduler
│   └── src/
│       ├── lib.rs
│       ├── job.rs          # Job definitions
│       ├── node.rs         # Node registry
│       ├── scheduler.rs    # Gang scheduling
│       └── api/            # gRPC/REST
│
├── zenith-proto/           # Protocol definitions
│   └── zenith.proto
│
├── zenith-bench/           # Benchmarks
│
└── sdk-python/             # Python SDK
    └── zenith/
        ├── __init__.py
        ├── engine.py
        ├── loader.py
        ├── torch/
        └── tensorflow/
```

### 5.2 Alur Data

```
┌─────────────┐
│    User     │
│   Code      │
└──────┬──────┘
       │
       ▼
┌──────────────┐     ┌──────────────┐
│  Python SDK  │────►│  Rust Core   │
│  (zenith-ai) │     │  (PyO3)      │
└──────────────┘     └──────┬───────┘
                            │
       ┌────────────────────┼────────────────────┐
       │                    │                    │
       ▼                    ▼                    ▼
┌────────────┐      ┌────────────┐      ┌────────────┐
│ CPU Engine │      │GPU Runtime │      │ Scheduler  │
│            │      │            │      │            │
│ • NUMA     │      │ • CUDA     │      │ • Gang     │
│ • io_uring │      │ • NCCL     │      │ • Priority │
│ • Buffers  │      │ • Offload  │      │ • Topology │
└────────────┘      └────────────┘      └────────────┘
```

---

## 6. Pengembangan Plugin

### 6.1 Membuat Plugin WASM

```rust
// plugin/src/lib.rs
#[no_mangle]
pub extern "C" fn process(ptr: *mut u8, len: usize) -> i32 {
    let data = unsafe { std::slice::from_raw_parts_mut(ptr, len) };
    
    // Lakukan preprocessing
    for byte in data.iter_mut() {
        *byte = byte.saturating_add(1);
    }
    
    0 // Success
}
```

### 6.2 Build Plugin

```bash
# Install target WASM
rustup target add wasm32-unknown-unknown

# Build
cargo build --target wasm32-unknown-unknown --release

# Output: target/wasm32-unknown-unknown/release/plugin.wasm
```

### 6.3 Gunakan Plugin

```python
import zenith

engine = zenith.Engine()
engine.load_plugin("plugin.wasm")

result = engine.process(data)
```

---

## 7. Deployment

### 7.1 Single Node

```bash
# Jalankan benchmark
cargo run -p zenith-bench --release -- full

# Jalankan dengan logging
RUST_LOG=debug cargo run -p zenith-scheduler --release
```

### 7.2 Docker

```dockerfile
FROM rust:1.75 AS builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
COPY --from=builder /app/target/release/zenith-scheduler /usr/local/bin/
CMD ["zenith-scheduler"]
```

### 7.3 Kubernetes

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: zenith-scheduler
spec:
  replicas: 1
  template:
    spec:
      containers:
      - name: scheduler
        image: zenith/scheduler:latest
        ports:
        - containerPort: 50051
        - containerPort: 8080
```

---

## 8. Troubleshooting

### 8.1 Common Issues

**Issue: Python import gagal**
```
ImportError: cannot import name '_core' from 'zenith'
```
**Solusi:** Native extension tidak terinstall. Install dengan:
```bash
pip install zenith-ai --force-reinstall
```

**Issue: NUMA tidak terdeteksi**
```
NUMA not available on this system
```
**Solusi:** Ini normal untuk sistem single-socket. Zenith akan menggunakan fallback.

**Issue: Performance tidak sesuai harapan**
```
Throughput lebih rendah dari benchmark
```
**Solusi:**
1. Pastikan hugepages diaktifkan: `cat /proc/meminfo | grep Huge`
2. Pastikan CPU frequency scaling dinonaktifkan
3. Jalankan dengan `taskset` untuk pin ke core spesifik

### 8.2 Debug Mode

```bash
# Enable debug logging
RUST_LOG=zenith=debug cargo run -p zenith-bench --release

# Profile dengan perf
perf record cargo run -p zenith-bench --release -- full
perf report
```

---

## Penutup

Selamat! Anda sekarang memahami cara kerja Zenith dan cara menggunakannya. Untuk pertanyaan lebih lanjut:

- GitHub Issues: https://github.com/vibeswithkk/Zenith-dataplane/issues
- Dokumentasi: https://github.com/vibeswithkk/Zenith-dataplane/tree/main/docs

---

**Dibuat oleh:** Wahyu Ardiansyah (Indonesia)  
**License:** Apache 2.0
