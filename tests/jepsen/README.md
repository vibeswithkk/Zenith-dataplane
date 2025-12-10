# Jepsen Distributed Consistency Testing

This directory contains Jepsen-style distributed consistency tests for Zenith Dataplane.

## Overview

Jepsen is a testing framework for distributed systems that helps verify correctness under various fault conditions. Our implementation tests:

1. **Network Connectivity** - Inter-node communication
2. **Network Partitions (Nemesis)** - Node isolation simulation
3. **Recovery** - Partition healing
4. **Concurrent Operations** - Multi-threaded read/writes
5. **Linearizability** - Sequential consistency

## Quick Start

### Prerequisites

- Docker and Docker Compose
- Python 3.8+
- sshpass (for remote testing)

### Local Testing

```bash
# Start the test cluster
docker-compose -f docker-compose.jepsen.yml up -d

# Wait for nodes to start
sleep 10

# Run tests
python3 jepsen_test.py
```

### Remote Testing

```bash
# Copy to remote server
scp -r . user@server:/path/to/jepsen

# SSH and run
ssh user@server "cd /path/to/jepsen && python3 jepsen_test.py"
```

## Test Phases

### Phase 1: Connectivity

Verifies all nodes can ping each other within the Docker network.

### Phase 2: Network Partition (Nemesis)

Simulates network failures using iptables:
```bash
# Isolate node-3
iptables -A OUTPUT -d 172.28.0.11 -j DROP
iptables -A INPUT -s 172.28.0.11 -j DROP
```

### Phase 3: Recovery

Heals the partition and verifies recovery:
```bash
# Restore connectivity
iptables -F
```

### Phase 4: Concurrent Operations

Executes concurrent read/write operations across all nodes using ThreadPoolExecutor.

### Phase 5: Linearizability Check

Performs sequential writes and verifies all nodes see the same value.

## Expected Results

| Test | Expected | Reason |
|------|----------|--------|
| Connectivity | PASS | Docker network works |
| Network Partition | PASS | iptables works |
| Recovery | PASS | Partition heals |
| Concurrent Ops | PASS | Thread-safe |
| Linearizability | FAIL | No replication layer |

**Note:** Linearizability failure is expected because Zenith nodes operate independently without cross-node replication.

## Configuration

### Node IPs

| Node | IP |
|------|-----|
| zenith-node-1 | 172.28.0.11 |
| zenith-node-2 | 172.28.0.12 |
| zenith-node-3 | 172.28.0.13 |
| jepsen-controller | 172.28.0.10 |

### Resource Limits

- Nodes: 256MB RAM, 0.3 CPU
- Controller: 512MB RAM, 0.5 CPU

## Output

Test results are saved to:
- Console output with timestamps
- `/tmp/jepsen_report.json` for programmatic access

### Sample Output

```
============================================================
           JEPSEN TEST REPORT - ZENITH DATAPLANE
============================================================
  Start Time: 2025-12-10 11:12:39
  Duration:   0:00:17.640284
  Nodes:      3
============================================================

  TEST RESULTS:
------------------------------------------------------------
  [PASS] connectivity: 6/6 connections successful
  [PASS] network_partition: Node isolation successful
  [PASS] recovery: Network partition healed successfully
  [PASS] concurrent_operations: 8/8 operations successful
  [FAIL] linearizability: Inconsistency detected
------------------------------------------------------------

  SUMMARY: 4/5 tests passed (80.0%)
============================================================
```

## Files

| File | Description |
|------|-------------|
| `jepsen_test.py` | Main test script |
| `docker-compose.jepsen.yml` | Container orchestration |
| `README.md` | This documentation |

## References

- [Jepsen.io](https://jepsen.io/) - Official Jepsen testing framework
- [Consistency Models](https://jepsen.io/consistency) - Understanding distributed consistency
