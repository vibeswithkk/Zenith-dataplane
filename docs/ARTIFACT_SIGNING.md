# Artifact Signing Guide
# Zenith Dataplane
**Version:** 1.0 
**Date:** 2025-12-10
---
## Overview

This document describes how to cryptographically sign Zenith build artifacts for supply chain security.
---
## 1. Signing Methods
### Option A: Sigstore/Cosign (Recommended)

Keyless signing using OIDC identity.

```bash
# Install cosign
curl -O -L https://github.com/sigstore/cosign/releases/latest/download/cosign-linux-amd64
chmod +x cosign-linux-amd64
sudo mv cosign-linux-amd64 /usr/local/bin/cosign
# Sign binary
cosign sign-blob --output-signature zenith.sig target/release/zenith
cosign sign-blob --output-certificate zenith.crt target/release/zenith
# Verify signature
cosign verify-blob --signature zenith.sig --certificate zenith.crt target/release/zenith
```
### Option B: GPG Signing

Traditional GPG signing.

```bash
# Generate GPG key (if not exists)
gpg --full-generate-key
# Sign binary
gpg --armor --detach-sign target/release/zenith
# Verify signature
gpg --verify target/release/zenith.asc target/release/zenith
```
### Option C: SHA256 Checksums

Minimal integrity verification.

```bash
# Generate checksums
sha256sum target/release/zenith > checksums.txt
sha256sum target/release/libzenith_core.so >> checksums.txt
# Verify checksums
sha256sum -c checksums.txt
```
---
## 2. CI/CD Integration
### GitHub Actions Workflow

```yaml
# .github/workflows/release.yml
name: Release with Signing

on:
 push:
 tags:
- 'v*'

jobs:
 build-and-sign:
 runs-on: ubuntu-latest
 permissions:
 id-token: write # For Sigstore OIDC
 contents: write
 
 steps:
- uses: actions/checkout@v4
- name: Build Release
 run: cargo build --release
- name: Install Cosign
 uses: sigstore/cosign-installer@v3
- name: Sign Artifacts
 run: |
 cosign sign-blob \
--yes \
--output-signature zenith.sig \
--output-certificate zenith.crt \
 target/release/zenith
- name: Generate SBOM
 run: |
 cargo sbom > SBOM.json
 cosign sign-blob --yes \
--output-signature SBOM.sig \
 target/release/SBOM.json
- name: Generate Checksums
 run: |
 sha256sum target/release/zenith > checksums.txt
 sha256sum target/release/*.so >> checksums.txt
- name: Upload Artifacts
 uses: actions/upload-artifact@v4
 with:
 name: zenith-signed
 path: |
 target/release/zenith
 zenith.sig
 zenith.crt
 checksums.txt
 SBOM.json
```
---
## 3. Verification Script
### verify_artifacts.sh

```bash
#!/bin/bash
# Zenith Artifact Verification Script

set -e

ARTIFACT="${1:-target/release/zenith}"
SIG="${ARTIFACT}.sig"
CRT="${ARTIFACT}.crt"

echo "=== Zenith Artifact Verification ==="
echo ""
# 1. Check file exists
if [ ! -f "$ARTIFACT" ]; then
 echo "[FAIL] Artifact not found: $ARTIFACT"
 exit 1
fi
echo "[PASS] Artifact exists"
# 2. Verify SHA256
if [ -f "checksums.txt" ]; then
 if sha256sum -c checksums.txt --quiet 2>/dev/null; then
 echo "[PASS] SHA256 checksum verified"
 else
 echo "[FAIL] SHA256 checksum mismatch"
 exit 1
 fi
fi
# 3. Verify Sigstore signature
if [ -f "$SIG" ] && [ -f "$CRT" ]; then
 if cosign verify-blob --signature "$SIG" --certificate "$CRT" "$ARTIFACT" 2>/dev/null; then
 echo "[PASS] Sigstore signature verified"
 else
 echo "[FAIL] Sigstore signature invalid"
 exit 1
 fi
else
 echo "[WARN] No Sigstore signature found"
fi
# 4. Verify GPG signature
if [ -f "${ARTIFACT}.asc" ]; then
 if gpg --verify "${ARTIFACT}.asc" "$ARTIFACT" 2>/dev/null; then
 echo "[PASS] GPG signature verified"
 else
 echo "[FAIL] GPG signature invalid"
 exit 1
 fi
fi

echo ""
echo "=== Verification Complete ==="
```
---
## 4. SLSA Provenance

For SLSA Level 3 compliance, generate provenance:

```yaml
# Add to GitHub Actions
- name: Generate SLSA Provenance
 uses: slsa-framework/slsa-github-generator/.github/workflows/generator_generic_slsa3.yml@v1.9.0
 with:
 base64-subjects: |
 ${{ steps.hash.outputs.hashes }}
```
---
## 5. Quick Start
### Sign Release Artifacts

```bash
# Build release
cargo build --release
# Generate checksums
cd target/release
sha256sum zenith libzenith_core.so > checksums.txt
# Sign with cosign (requires OIDC login)
cosign sign-blob --yes zenith --output-signature zenith.sig --output-certificate zenith.crt
# Create signed release package
tar -czvf zenith-v0.2.3-linux-x86_64-signed.tar.gz \
 zenith \
 libzenith_core.so \
 zenith.sig \
 zenith.crt \
 checksums.txt
```
### Verify Downloaded Artifacts

```bash
# Download and extract
tar -xzvf zenith-v0.2.3-linux-x86_64-signed.tar.gz
# Verify
sha256sum -c checksums.txt
cosign verify-blob --signature zenith.sig --certificate zenith.crt zenith
```
---
## 6. Key Management
### For Production

| Method | Key Storage | Rotation |
|--------|-------------|----------|
| Sigstore | OIDC (keyless) | Automatic |
| GPG | Hardware token (YubiKey) | Annual |
| KMS | AWS/GCP/Azure KMS | Per policy |
---
## 7. Compliance Checklist

| Item | Status |
|------|--------|
| SHA256 checksums | [OK] Available |
| Signing method documented | [OK] This document |
| CI/CD integration ready | [OK] Workflow provided |
| Verification script | [OK] verify_artifacts.sh |
| SLSA guidance | [OK] Documented |
---
**Document Version:** 1.0 
**Last Updated:** 2025-12-10
