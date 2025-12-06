# Publishing Zenith AI to PyPI

This guide covers how to publish zenith-ai to the Python Package Index (PyPI).

## Prerequisites

1. **PyPI Account**: Register at https://pypi.org/account/register/
2. **2FA Enabled**: PyPI requires Two-Factor Authentication
3. **API Token**: Create at https://pypi.org/manage/account/token/

## Quick Publish (Recommended)

### Using Maturin (Easiest)

```bash
# Navigate to sdk-python directory
cd sdk-python

# Publish directly (will prompt for token)
maturin publish

# Or with token directly
maturin publish --token pypi-YOUR_API_TOKEN_HERE
```

### Using Twine (Alternative)

```bash
# 1. Build the wheel first
cd sdk-python
maturin build --release

# 2. Upload with twine
twine upload target/wheels/*.whl

# When prompted:
# Username: __token__
# Password: pypi-YOUR_API_TOKEN_HERE
```

## Step-by-Step Guide

### 1. Set Up Credentials

Create `~/.pypirc` for convenience:

```ini
[pypi]
username = __token__
password = pypi-YOUR_API_TOKEN_HERE
```

**Security Note**: Set permissions with `chmod 600 ~/.pypirc`

### 2. Test on TestPyPI First (Recommended)

Before publishing to real PyPI, test on TestPyPI:

```bash
# Register at https://test.pypi.org/account/register/
# Create API token at https://test.pypi.org/manage/account/token/

# Publish to TestPyPI
maturin publish --repository testpypi --token pypi-TEST_TOKEN

# Test installation
pip install --index-url https://test.pypi.org/simple/ zenith-ai
```

### 3. Publish to PyPI

Once tested, publish to real PyPI:

```bash
cd sdk-python
maturin publish --token pypi-YOUR_REAL_TOKEN
```

### 4. Verify Publication

```bash
# Wait a few minutes, then:
pip install zenith-ai

# Test
python -c "import zenith; print(zenith.__version__)"
```

## Building for Multiple Platforms

For maximum compatibility, build wheels for multiple platforms:

### Using GitHub Actions (Recommended)

Create `.github/workflows/release.yml`:

```yaml
name: Release to PyPI

on:
  release:
    types: [published]

jobs:
  build-wheels:
    runs-on: ${{ matrix.os }}
    strategy:
      matrix:
        os: [ubuntu-latest, macos-latest, windows-latest]
        python-version: ['3.10', '3.11', '3.12']
    
    steps:
      - uses: actions/checkout@v4
      
      - name: Set up Python
        uses: actions/setup-python@v5
        with:
          python-version: ${{ matrix.python-version }}
      
      - name: Install Rust
        uses: dtolnay/rust-action@stable
      
      - name: Install maturin
        run: pip install maturin
      
      - name: Build wheel
        run: |
          cd sdk-python
          maturin build --release
      
      - name: Upload wheel
        uses: actions/upload-artifact@v4
        with:
          name: wheel-${{ matrix.os }}-${{ matrix.python-version }}
          path: sdk-python/target/wheels/*.whl

  publish:
    needs: build-wheels
    runs-on: ubuntu-latest
    
    steps:
      - name: Download all wheels
        uses: actions/download-artifact@v4
        with:
          path: wheels
      
      - name: Publish to PyPI
        uses: pypa/gh-action-pypi-publish@release/v1
        with:
          packages-dir: wheels/
          password: ${{ secrets.PYPI_API_TOKEN }}
```

### Manual Multi-Platform Build

If you have access to different machines:

```bash
# On Linux
cd sdk-python && maturin build --release

# On macOS
cd sdk-python && maturin build --release

# On Windows
cd sdk-python && maturin build --release

# Collect all .whl files and upload together
twine upload *.whl
```

## Version Management

Before each release, update version in:

1. `sdk-python/Cargo.toml`:
   ```toml
   version = "0.2.0"
   ```

2. `sdk-python/pyproject.toml`:
   ```toml
   version = "0.2.0"
   ```

3. `sdk-python/zenith/__init__.py`:
   ```python
   __version__ = "0.2.0"
   ```

## Troubleshooting

### "File already exists"
- You cannot overwrite existing versions on PyPI
- Bump the version number and republish

### "Invalid token"
- Make sure you're using `__token__` as username
- Token should start with `pypi-`
- Check token hasn't expired

### "Package name taken"
- Choose a unique name
- Check availability at https://pypi.org/project/YOUR-NAME/

## Useful Commands

```bash
# Check what will be uploaded
twine check target/wheels/*.whl

# View package on PyPI
open https://pypi.org/project/zenith-ai/

# Install specific version
pip install zenith-ai==0.1.0
```

## Links

- PyPI: https://pypi.org/project/zenith-ai/
- TestPyPI: https://test.pypi.org/project/zenith-ai/
- Maturin Docs: https://www.maturin.rs/
