#!/bin/bash
# Zenith Plugin Builder
# Builds WASM plugins with proper target

set -e

usage() {
    echo "Usage: $0 <plugin_directory>"
    echo "Example: $0 plugins/simple_filter"
    exit 1
}

if [ $# -lt 1 ]; then
    usage
fi

PLUGIN_DIR=$1

if [ ! -d "$PLUGIN_DIR" ]; then
    echo "Error: Directory $PLUGIN_DIR does not exist"
    exit 1
fi

if [ ! -f "$PLUGIN_DIR/Cargo.toml" ]; then
    echo "Error: No Cargo.toml found in $PLUGIN_DIR"
    exit 1
fi

echo "[BUILD] Building WASM plugin: $PLUGIN_DIR"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

# Ensure target is installed
if ! rustup target list | grep -q "wasm32-wasip1 (installed)"; then
    echo "[INSTALL] Installing wasm32-wasip1 target..."
    rustup target add wasm32-wasip1
fi

# Build
cd "$PLUGIN_DIR"
echo "[INFO] Compiling..."
cargo build --target wasm32-wasip1 --release

# Find output
PLUGIN_NAME=$(basename "$PLUGIN_DIR")
WASM_FILE="target/wasm32-wasip1/release/${PLUGIN_NAME}.wasm"

if [ -f "$WASM_FILE" ]; then
    SIZE=$(du -h "$WASM_FILE" | cut -f1)
    echo "[OK] Build successful!"
    echo "[INSTALL] Output: $WASM_FILE ($SIZE)"
    echo ""
    echo "To use:"
    echo "  zenith-cli load-plugin $WASM_FILE"
else
    echo "[FAIL] Build failed: Output file not found"
    exit 1
fi
