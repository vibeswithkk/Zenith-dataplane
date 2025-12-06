# Contributing to Zenith AI

Thank you for your interest in contributing to Zenith AI! We're building high-performance data infrastructure for the AI era, and we welcome contributions from the community.

## Getting Started

### Prerequisites

- **Rust**: 1.75+ (for core engine)
- **Python**: 3.10+ (for SDK)
- **Maturin**: For Python packaging (`pip install maturin`)
- **WASM target**: `rustup target add wasm32-wasip1`

### Development Setup

```bash
# Clone the repository
git clone https://github.com/vibeswithkk/Zenith-dataplane.git
cd Zenith-dataplane

# Build Rust components
cargo build --release

# Set up Python environment
python3 -m venv .venv
source .venv/bin/activate
pip install -e sdk-python[dev]

# Verify installation
python3 -c "import zenith; print(zenith.__version__)"
```

## Development Workflow

### 1. Create a Feature Branch

```bash
git checkout -b feature/my-awesome-feature
```

### 2. Make Your Changes

- **Rust code**: Located in `core/`, `runtime/`, `host-api/`, etc.
- **Python SDK**: Located in `sdk-python/zenith/`
- **WASM Plugins**: Located in `plugins/`
- **Documentation**: Located in `docs/`

### 3. Run Tests

```bash
# Rust tests
cargo test

# Python tests
cd sdk-python
pytest

# Integration tests
python3 examples/demo_app.py
```

### 4. Format and Lint

```bash
# Rust
cargo fmt
cargo clippy

# Python
pip install black pylint
black sdk-python/
pylint sdk-python/zenith/
```

### 5. Commit with Conventional Commits

We follow [Conventional Commits](https://www.conventionalcommits.org/):

```bash
# Features
git commit -m "feat(torch): add distributed training support"

# Bug fixes
git commit -m "fix(engine): resolve memory leak in ring buffer"

# Documentation
git commit -m "docs: update PyTorch integration guide"

# Performance
git commit -m "perf(loader): optimize batch prefetching"
```

### 6. Submit a Pull Request

Push your branch and create a PR against `main`. Please include:
- Clear description of changes
- Any related issue numbers
- Screenshots/benchmarks if applicable

## Project Structure

```
zenith-dataplane/
├── core/               # Rust core engine
├── runtime/            # WASM runtime and plugin execution
├── host-api/           # HTTP/REST API server
├── control-plane/      # Cluster management
├── sdk-python/         # Python SDK (zenith-ai package)
│   ├── zenith/         # Main package
│   │   ├── torch/      # PyTorch adapter
│   │   └── tensorflow/ # TensorFlow adapter
│   └── pyproject.toml  # Maturin config
├── plugins/            # WASM preprocessing plugins
│   └── ai_preprocessing/
├── examples/           # Usage examples
├── docs/               # Documentation
└── tools/              # Development utilities
```

## Contribution Areas

### High Impact Contributions

1. **Framework Adapters**
   - JAX integration (`sdk-python/zenith/jax/`)
   - HuggingFace Datasets integration
   - Ray Data integration

2. **Preprocessing Plugins**
   - Audio preprocessing (WASM)
   - Video frame extraction
   - Advanced tokenizers

3. **Performance Optimization**
   - GPU Direct Storage support
   - RDMA/InfiniBand networking
   - Memory-mapped I/O improvements

4. **Cloud Integrations**
   - S3/GCS streaming
   - Azure Blob support
   - Kafka consumer

### Good First Issues

Look for issues labeled `good-first-issue` in our GitHub Issues.

## Code Style

### Rust

- Follow Rust API Guidelines
- Use `cargo fmt` for formatting
- Keep `unwrap()` to a minimum; prefer proper error handling
- Document public APIs with rustdoc

### Python

- Follow PEP 8
- Use type hints (Python 3.10+)
- Document with docstrings (Google style)
- Keep functions focused and testable

## Testing Guidelines

### Unit Tests

- Place tests in `src/` alongside the code (Rust)
- Use `pytest` for Python tests
- Aim for high coverage on critical paths

### Integration Tests

- Located in `tests/` directory
- Test end-to-end workflows
- Include performance regression tests

### Benchmark Tests

- Use `criterion` for Rust benchmarks
- Use `pytest-benchmark` for Python
- Compare against baseline before PR

## Documentation

- Update relevant docs when changing functionality
- Include docstrings for all public APIs
- Add examples for new features
- Keep README.md up to date

## Release Process

1. Version bump in `Cargo.toml` and `pyproject.toml`
2. Update CHANGELOG.md
3. Create GitHub Release
4. CI publishes to PyPI and crates.io

## Community

- **Issues**: Report bugs and request features
- **Discussions**: Ask questions and share ideas
- **Pull Requests**: Contribute code

## License

By contributing, you agree that your contributions will be licensed under the Apache License 2.0.

---

Thank you for helping make Zenith AI better for everyone!
