# Known Issues & Technical Debt

> **For internal tracking only**
> **Last Updated:** 2025-12-09

---

## 🔴 Critical (Must Fix Before Release) — ALL FIXED ✅

### 1. ~~`todo!()` Macros in io_uring~~ ✅ FIXED
- **Location:** `zenith-runtime-cpu/src/io.rs`
- **Impact:** Runtime crash if io_uring feature enabled
- **Status:** Fixed - now returns `Error::NotImplemented`

### 2. ~~Panic Across FFI Boundary~~ ✅ FIXED
- **Location:** `core/src/lib.rs`
- **Impact:** Python process abort without traceback
- **Fix:** Added `std::panic::catch_unwind` at all FFI boundaries
- **Status:** ✅ Fixed - panics now caught and return error codes

### 3. ~~Zombie Jobs in Scheduler~~ ✅ FIXED
- **Location:** `zenith-scheduler/src/scheduler.rs`
- **Impact:** Jobs stuck in "Running" state forever if node dies
- **Fix:** Added `cleanup_zombie_jobs()` method with timeout + heartbeat detection
- **Status:** ✅ Fixed - scheduler now detects and cleans up zombie jobs

### 4. ~~Input Validation Missing~~ ✅ FIXED
- **Location:** `core/src/validation.rs`
- **Impact:** Potential security vulnerabilities
- **Fix:** Created comprehensive validation module
- **Status:** ✅ Fixed - validation utilities for job names, paths, commands, ranges

---

## 🟠 High Priority

### 5. `unwrap()` Usage
- **Location:** 139+ locations across codebase
- **Impact:** Panic on unexpected None/Err
- **Fix:** Replace with proper error handling
- **Status:** 📋 Planned

### 6. GPU Runtime Placeholders
- **Location:** `zenith-runtime-gpu/src/cuda.rs`, `tensorrt.rs`, `multigpu.rs`
- **Impact:** GPU features don't actually work
- **Fix:** Mark as experimental or implement
- **Status:** 📋 Planned

### 7. ~~Missing Heartbeat/Health Checks~~ ✅ FIXED (via #3)
- **Location:** Scheduler, Node Agent
- **Impact:** No detection of dead nodes
- **Fix:** Implemented in `cleanup_zombie_jobs()` and `is_node_healthy()`
- **Status:** ✅ Fixed

---

## 🟡 Medium Priority

### 8. Helm NetworkPolicy Disabled
- **Location:** `deploy/helm/zenith/values.yaml`
- **Impact:** Security risk in K8s deployments
- **Fix:** Enable by default
- **Status:** 📋 Planned

### 9. Missing ABI Versioning
- **Location:** Python wheel packaging
- **Impact:** Version mismatch crashes
- **Fix:** Add SONAME/ABI metadata
- **Status:** 📋 Planned

### 10. Limited Test Coverage
- **Current:** ~73 tests
- **Target:** 300+ tests for production
- **Fix:** Add comprehensive test suite
- **Status:** 📋 Planned

---

## 🟢 Low Priority / Nice to Have

### 11. Documentation Gaps
- Many functions missing rustdoc
- Tutorials incomplete
- **Status:** 📋 Planned

### 12. CI/CD Improvements
- No performance regression detection
- Missing multi-arch builds
- **Status:** 📋 Planned

### 13. Manylinux Wheel Builds
- Currently source-only distribution
- **Status:** 📋 Planned

---

## Technical Debt Summary

| Category | Count | Status |
|----------|-------|--------|
| Critical bugs | 4/4 | ✅ ALL FIXED |
| High priority | 1/3 | 🟠 In progress |
| Medium priority | 0/3 | 🟡 Planned |
| Low priority | 0/3 | 🟢 Planned |
| **Total Fixed** | **5/13** | — |

---

## Fix Progress

```
[█████░░░░░] 38% Complete

Fixed: 5/13
In Progress: 0/13
Planned: 8/13

CRITICAL BUGS: 100% COMPLETE ✅
```

---

## Notes

- ✅ All critical issues are now FIXED
- High priority issues should be fixed before beta release
- Medium/Low can be addressed iteratively
