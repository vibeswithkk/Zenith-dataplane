# Zenith Dependencies - All FREE Software

This document lists all external dependencies required for Zenith's advanced features, 
confirming that **everything is available for FREE**.

## 🆓 Core Dependencies (All FREE)

| Dependency | License | Cost | Purpose |
|------------|---------|------|---------|
| Rust | MIT/Apache 2.0 | **FREE** | Programming language |
| Tokio | MIT | **FREE** | Async runtime |
| Crossbeam | MIT/Apache 2.0 | **FREE** | Lock-free data structures |
| Serde | MIT/Apache 2.0 | **FREE** | Serialization |
| io-uring | MIT | **FREE** | Async I/O |

## 🆓 ML Acceleration Dependencies (All FREE)

### ONNX Runtime

| Item | Details |
|------|---------|
| **License** | MIT (Open Source) |
| **Cost** | **$0 (FREE!)** |
| **Developer** | Microsoft |
| **Install** | `cargo add ort` or `pip install onnxruntime` |
| **GPU Support** | Free (requires CUDA) |
| **Source** | https://github.com/microsoft/onnxruntime |

```toml
# In Cargo.toml
ort = { version = "2.0.0-rc.10", optional = true }
```

```bash
# Python installation
pip install onnxruntime        # CPU (FREE)
pip install onnxruntime-gpu    # GPU (FREE, needs CUDA)
```

### TensorRT

| Item | Details |
|------|---------|
| **License** | NVIDIA Proprietary (Free to use) |
| **Cost** | **$0 (FREE!)** |
| **Developer** | NVIDIA |
| **Requires** | NVIDIA GPU (hardware cost only) |
| **Download** | https://developer.nvidia.com/tensorrt |

```bash
# Ubuntu/Debian
sudo apt install tensorrt

# Or via pip
pip install tensorrt
```

### CUDA Toolkit

| Item | Details |
|------|---------|
| **License** | NVIDIA EULA (Free to use) |
| **Cost** | **$0 (FREE!)** |
| **Developer** | NVIDIA |
| **Requires** | NVIDIA GPU |
| **Download** | https://developer.nvidia.com/cuda-downloads |

```bash
# Ubuntu
sudo apt install nvidia-cuda-toolkit
```

### cuDNN

| Item | Details |
|------|---------|
| **License** | NVIDIA EULA (Free to use) |
| **Cost** | **$0 (FREE!)** |
| **Developer** | NVIDIA |
| **Download** | https://developer.nvidia.com/cudnn |

## 💰 Hardware Costs (Only Expense)

The only costs are for hardware if you want GPU acceleration:

| GPU | Approximate Cost | Performance |
|-----|------------------|-------------|
| RTX 3060 (12GB) | ~$300 | 12.7 TFLOPS FP32 |
| RTX 4070 (12GB) | ~$600 | 29.1 TFLOPS FP32 |
| RTX 4090 (24GB) | ~$1,600 | 82.6 TFLOPS FP32 |
| A100 (40GB) | ~$10,000 | 156 TFLOPS FP32 |
| H100 (80GB) | ~$30,000 | 756 TFLOPS FP32 |

**Note:** CPU-only mode is 100% FREE with no hardware requirements.

## 🚀 Cloud Alternatives (Pay-per-Use)

If you don't want to buy hardware, use cloud GPU instances:

| Provider | GPU | Price (approx) |
|----------|-----|----------------|
| Google Colab | T4 | **FREE** (limited) |
| Kaggle | P100 | **FREE** (limited) |
| RunPod | RTX 4090 | ~$0.44/hour |
| Lambda Labs | A100 | ~$1.10/hour |
| AWS | A100 | ~$3.00/hour |

## 📋 Summary

| Category | Status |
|----------|--------|
| **Operating System** | Linux is FREE |
| **Programming Language** | Rust is FREE |
| **All Rust Crates** | FREE (open source) |
| **ONNX Runtime** | FREE |
| **TensorRT** | FREE |
| **CUDA Toolkit** | FREE |
| **cuDNN** | FREE |
| **GPU Hardware** | $300 - $30,000 (optional) |
| **Cloud GPU** | $0 - $3/hour |

---

## ✅ Conclusion

**All software used by Zenith is 100% FREE and open source.**

The only potential cost is GPU hardware if you want maximum performance, 
but Zenith works perfectly on CPU-only systems at no cost.

---

*Last updated: December 8, 2025*
