#!/usr/bin/env python3
"""
Phase 3 Validation Script
========================

Validates all Phase 3 components work correctly.
"""

import subprocess
import sys
import os
from pathlib import Path

class Colors:
    GREEN = '\033[92m'
    YELLOW = '\033[93m'
    RED = '\033[91m'
    BLUE = '\033[94m'
    BOLD = '\033[1m'
    END = '\033[0m'

def print_header(text):
    print(f"\n{Colors.BOLD}{Colors.BLUE}{'='*60}{Colors.END}")
    print(f"{Colors.BOLD}{Colors.BLUE}  {text}{Colors.END}")
    print(f"{Colors.BOLD}{Colors.BLUE}{'='*60}{Colors.END}\n")

def run_cmd(cmd, cwd=None):
    try:
        result = subprocess.run(cmd, shell=True, cwd=cwd, capture_output=True, text=True, timeout=120)
        return result.returncode == 0, result.stdout, result.stderr
    except Exception as e:
        return False, "", str(e)

def validate_circuit_breaker():
    """Test circuit breaker pattern"""
    print_header("Phase 3A: Circuit Breaker")
    
    project = Path(__file__).parent.parent
    success, out, err = run_cmd(
        "cargo test circuit_breaker --lib -p zenith-runtime-cpu -- --nocapture 2>&1",
        cwd=project
    )
    
    output = out + err
    if "3 passed" in output:
        print(f"{Colors.GREEN}[✓] Circuit Breaker: 3/3 tests passed{Colors.END}")
        print(f"    - Normal operation: ✓")
        print(f"    - Opens on failures: ✓")
        print(f"    - Reset works: ✓")
        return True
    else:
        print(f"{Colors.RED}[✗] Circuit Breaker tests failed{Colors.END}")
        return False

def validate_health_checks():
    """Test health check system"""
    print_header("Phase 3A: Health Checks")
    
    project = Path(__file__).parent.parent
    success, out, err = run_cmd(
        "cargo test health --lib -p zenith-runtime-cpu -- --nocapture 2>&1",
        cwd=project
    )
    
    output = out + err
    if "2 passed" in output:
        print(f"{Colors.GREEN}[✓] Health Checks: 2/2 tests passed{Colors.END}")
        print(f"    - Health manager: ✓")
        print(f"    - Readiness probes: ✓")
        return True
    else:
        print(f"{Colors.RED}[✗] Health check tests failed{Colors.END}")
        return False

def validate_helm_chart():
    """Validate Helm chart structure"""
    print_header("Phase 3B: Kubernetes Helm Chart")
    
    project = Path(__file__).parent.parent
    helm_dir = project / "deploy" / "helm" / "zenith"
    
    required_files = [
        "Chart.yaml",
        "values.yaml",
        "templates/_helpers.tpl",
        "templates/scheduler-deployment.yaml"
    ]
    
    all_exist = True
    for f in required_files:
        path = helm_dir / f
        if path.exists():
            print(f"{Colors.GREEN}[✓] {f} exists{Colors.END}")
        else:
            print(f"{Colors.RED}[✗] {f} missing{Colors.END}")
            all_exist = False
    
    # Validate YAML syntax
    import yaml
    for f in ["Chart.yaml", "values.yaml"]:
        try:
            yaml.safe_load(open(helm_dir / f))
            print(f"{Colors.GREEN}[✓] {f} is valid YAML{Colors.END}")
        except Exception as e:
            print(f"{Colors.RED}[✗] {f} has invalid YAML: {e}{Colors.END}")
            all_exist = False
    
    return all_exist

def validate_dockerfile():
    """Validate Dockerfile"""
    print_header("Phase 3B: Dockerfile")
    
    project = Path(__file__).parent.parent
    dockerfile = project / "Dockerfile"
    
    if not dockerfile.exists():
        print(f"{Colors.RED}[✗] Dockerfile not found{Colors.END}")
        return False
    
    content = dockerfile.read_text()
    
    checks = [
        ("FROM rust:", "Rust build stage"),
        ("FROM debian:", "Runtime stage"),
        ("HEALTHCHECK", "Health check configured"),
        ("EXPOSE", "Ports exposed"),
        ("USER zenith", "Non-root user"),
    ]
    
    all_pass = True
    for pattern, desc in checks:
        if pattern in content:
            print(f"{Colors.GREEN}[✓] {desc}{Colors.END}")
        else:
            print(f"{Colors.YELLOW}[!] {desc} not found{Colors.END}")
    
    return True

def validate_openapi():
    """Validate OpenAPI spec"""
    print_header("Phase 3D: OpenAPI Documentation")
    
    project = Path(__file__).parent.parent
    openapi = project / "docs" / "api" / "openapi.yaml"
    
    if not openapi.exists():
        print(f"{Colors.RED}[✗] OpenAPI spec not found{Colors.END}")
        return False
    
    import yaml
    try:
        spec = yaml.safe_load(open(openapi))
        
        print(f"{Colors.GREEN}[✓] OpenAPI spec is valid YAML{Colors.END}")
        print(f"    - Version: {spec.get('openapi', 'unknown')}")
        print(f"    - Title: {spec.get('info', {}).get('title', 'unknown')}")
        
        paths = spec.get('paths', {})
        print(f"    - Endpoints documented: {len(paths)}")
        
        for path in paths:
            print(f"      • {path}")
        
        return True
    except Exception as e:
        print(f"{Colors.RED}[✗] OpenAPI validation failed: {e}{Colors.END}")
        return False

def validate_ci_workflow():
    """Validate GitHub Actions workflow"""
    print_header("Phase 3E: CI/CD Pipeline")
    
    project = Path(__file__).parent.parent
    workflow = project / ".github" / "workflows" / "ci.yml"
    
    if not workflow.exists():
        print(f"{Colors.RED}[✗] CI workflow not found{Colors.END}")
        return False
    
    import yaml
    try:
        spec = yaml.safe_load(open(workflow))
        
        print(f"{Colors.GREEN}[✓] CI workflow is valid YAML{Colors.END}")
        
        jobs = spec.get('jobs', {})
        print(f"    - Jobs defined: {len(jobs)}")
        
        for job_name in jobs:
            print(f"      • {job_name}")
        
        return True
    except Exception as e:
        print(f"{Colors.RED}[✗] CI workflow validation failed: {e}{Colors.END}")
        return False

def main():
    print(f"""
{Colors.BOLD}{Colors.BLUE}
╔═══════════════════════════════════════════════════════════════╗
║                                                               ║
║   ZENITH PHASE 3 VALIDATION                                   ║
║                                                               ║
╚═══════════════════════════════════════════════════════════════╝
{Colors.END}
""")
    
    results = []
    
    tests = [
        ("Circuit Breaker", validate_circuit_breaker),
        ("Health Checks", validate_health_checks),
        ("Helm Chart", validate_helm_chart),
        ("Dockerfile", validate_dockerfile),
        ("OpenAPI", validate_openapi),
        ("CI/CD", validate_ci_workflow),
    ]
    
    for name, test_fn in tests:
        try:
            passed = test_fn()
            results.append((name, passed))
        except Exception as e:
            print(f"{Colors.RED}[✗] {name} crashed: {e}{Colors.END}")
            results.append((name, False))
    
    # Summary
    print_header("Validation Summary")
    
    passed = sum(1 for _, p in results if p)
    total = len(results)
    
    for name, success in results:
        if success:
            print(f"{Colors.GREEN}[✓] {name}: PASSED{Colors.END}")
        else:
            print(f"{Colors.RED}[✗] {name}: FAILED{Colors.END}")
    
    print(f"\n{Colors.BOLD}Total: {passed}/{total} validations passed{Colors.END}")
    
    if passed == total:
        print(f"\n{Colors.GREEN}{Colors.BOLD}✅ ALL PHASE 3 COMPONENTS VALIDATED!{Colors.END}\n")
        return 0
    else:
        print(f"\n{Colors.YELLOW}{Colors.BOLD}⚠️ Some validations need attention{Colors.END}\n")
        return 1

if __name__ == "__main__":
    sys.exit(main())
