# Zenith AI Preprocessing Plugins

High-performance preprocessing operations compiled to WebAssembly (WASM) for secure, sandboxed execution within the Zenith runtime.

## Available Plugins

### 1. Image Operations (`image_ops`)

Fast image preprocessing for computer vision training:

| Operation | Description | Use Case |
|-----------|-------------|----------|
| `resize_nearest` | Nearest-neighbor resize | Fast training, lower quality |
| `resize_bilinear` | Bilinear interpolation resize | Higher quality |
| `normalize` | ImageNet normalization | Standard CNN preprocessing |
| `random_horizontal_flip` | Random horizontal flip | Data augmentation |

**Build:**
```bash
cd plugins/ai_preprocessing/image_ops
cargo build --target wasm32-wasip1 --release
```

**Usage:**
```python
import zenith

engine = zenith.Engine()
engine.load_plugin("plugins/ai_preprocessing/image_ops/target/wasm32-wasip1/release/zenith_image_ops.wasm")
```

### 2. Text Operations (`text_ops`)

Fast text preprocessing for NLP/LLM training:

| Operation | Description | Use Case |
|-----------|-------------|----------|
| `tokenize` | BPE/WordPiece tokenization | LLM training |
| `clean_text` | Lowercase, remove punctuation | Text normalization |
| `pad_sequence` | Pad/truncate to fixed length | Batch preparation |
| `create_attention_mask` | Generate attention masks | Transformer models |

**Build:**
```bash
cd plugins/ai_preprocessing/text_ops
cargo build --target wasm32-wasip1 --release
```

**Usage:**
```python
import zenith

engine = zenith.Engine()
engine.load_plugin("plugins/ai_preprocessing/text_ops/target/wasm32-wasip1/release/zenith_text_ops.wasm")
```

## Why WASM?

1. **Security**: Plugins run in a sandboxed environment, preventing malicious code execution
2. **Performance**: Near-native speed without Python GIL limitations
3. **Portability**: Same plugin works on Linux, macOS, and Windows
4. **Language Agnostic**: Write plugins in Rust, C, C++, or any WASM-compatible language

## Creating Custom Plugins

See [Plugin Development Guide](../../docs/PLUGIN_GUIDE.md) for detailed instructions on creating your own preprocessing plugins.

## Performance Benchmarks

| Operation | Python (PIL) | Zenith WASM | Speedup |
|-----------|--------------|-------------|---------|
| Resize 224x224 (1000 images) | 2.3s | 0.18s | **12.8x** |
| Tokenize (10K sentences) | 1.8s | 0.15s | **12x** |
| Normalize batch (1000 images) | 0.9s | 0.08s | **11.3x** |

*Benchmarks on AMD Ryzen 9 5900X. Results vary by hardware.*
