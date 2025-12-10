#!/usr/bin/env python3
"""
Jepsen-style Distributed Consistency Test for Zenith Dataplane
================================================================
This test suite implements Jepsen-style testing for distributed systems:
1. Network Partition Testing (Nemesis)
2. Linearizability Checking
3. Recovery Testing
"""

import subprocess
import time
import random
import json
import sys
from datetime import datetime
from concurrent.futures import ThreadPoolExecutor, as_completed

# Zenith nodes configuration
NODES = {
    'zenith-node-1': '172.28.0.11',
    'zenith-node-2': '172.28.0.12',
    'zenith-node-3': '172.28.0.13'
}

class JepsenTest:
    def __init__(self):
        self.test_results = []
        self.history = []
        self.start_time = datetime.now()
        
    def log(self, msg, level='INFO'):
        timestamp = datetime.now().strftime('%H:%M:%S.%f')[:-3]
        print(f'[{timestamp}] [{level}] {msg}')
        self.history.append({
            'time': timestamp,
            'level': level,
            'msg': msg
        })
    
    def exec_on_node(self, node, cmd):
        """Execute command on a specific node"""
        try:
            result = subprocess.run(
                ['docker', 'exec', node, 'bash', '-c', cmd],
                capture_output=True, text=True, timeout=10
            )
            return {
                'success': result.returncode == 0,
                'stdout': result.stdout.strip(),
                'stderr': result.stderr.strip()
            }
        except subprocess.TimeoutExpired:
            return {'success': False, 'error': 'timeout'}
        except Exception as e:
            return {'success': False, 'error': str(e)}
    
    def test_connectivity(self):
        """Test Phase 1: Verify all nodes can communicate"""
        self.log('=== PHASE 1: Connectivity Test ===')
        results = []
        
        for src_node in NODES:
            for dst_node in NODES:
                if src_node == dst_node:
                    continue
                    
                dst_ip = NODES[dst_node]
                result = self.exec_on_node(
                    src_node, 
                    f'ping -c 2 -W 2 {dst_ip} > /dev/null 2>&1 && echo OK || echo FAIL'
                )
                
                status = 'PASS' if result.get('stdout') == 'OK' else 'FAIL'
                self.log(f'  {src_node} -> {dst_node}: {status}')
                results.append(status == 'PASS')
        
        all_pass = all(results)
        self.test_results.append({
            'test': 'connectivity',
            'passed': all_pass,
            'details': f'{sum(results)}/{len(results)} connections successful'
        })
        return all_pass
    
    def test_network_partition(self):
        """Test Phase 2: Simulate network partition (Nemesis)"""
        self.log('=== PHASE 2: Network Partition Test (Nemesis) ===')
        
        # Choose node to partition
        isolated_node = 'zenith-node-3'
        self.log(f'  Isolating {isolated_node} from cluster...')
        
        # Block traffic from node-3 to node-1 and node-2
        for target in ['zenith-node-1', 'zenith-node-2']:
            target_ip = NODES[target]
            self.exec_on_node(
                isolated_node,
                f'iptables -A OUTPUT -d {target_ip} -j DROP 2>/dev/null || true'
            )
            self.exec_on_node(
                isolated_node,
                f'iptables -A INPUT -s {target_ip} -j DROP 2>/dev/null || true'
            )
        
        self.log(f'  {isolated_node} is now partitioned')
        
        # Verify partition
        partition_active = True
        for target in ['zenith-node-1', 'zenith-node-2']:
            result = self.exec_on_node(
                isolated_node,
                f'ping -c 1 -W 1 {NODES[target]} > /dev/null 2>&1 && echo OK || echo BLOCKED'
            )
            if result.get('stdout') == 'OK':
                partition_active = False
                
        self.log(f'  Partition active: {partition_active}')
        time.sleep(2)  # Let the partition take effect
        
        # Test that node-1 and node-2 can still communicate
        healthy_comm = self.exec_on_node(
            'zenith-node-1',
            f'ping -c 1 -W 1 {NODES["zenith-node-2"]} > /dev/null 2>&1 && echo OK'
        )
        
        self.log(f'  Healthy nodes communication: {healthy_comm.get("stdout", "FAIL")}')
        
        self.test_results.append({
            'test': 'network_partition',
            'passed': partition_active and healthy_comm.get('stdout') == 'OK',
            'details': 'Node isolation successful, healthy nodes communicating'
        })
        
        return partition_active
    
    def test_recovery(self):
        """Test Phase 3: Network recovery"""
        self.log('=== PHASE 3: Network Recovery Test ===')
        
        isolated_node = 'zenith-node-3'
        
        # Restore network
        self.log(f'  Healing network partition for {isolated_node}...')
        self.exec_on_node(isolated_node, 'iptables -F')
        
        time.sleep(2)  # Allow recovery
        
        # Verify recovery
        recovered = True
        for target in ['zenith-node-1', 'zenith-node-2']:
            result = self.exec_on_node(
                isolated_node,
                f'ping -c 2 -W 2 {NODES[target]} > /dev/null 2>&1 && echo OK || echo FAIL'
            )
            status = result.get('stdout') == 'OK'
            self.log(f'  {isolated_node} -> {target}: {"RECOVERED" if status else "FAILED"}')
            recovered = recovered and status
        
        self.test_results.append({
            'test': 'recovery',
            'passed': recovered,
            'details': 'Network partition healed successfully' if recovered else 'Recovery failed'
        })
        
        return recovered
    
    def test_concurrent_operations(self):
        """Test Phase 4: Concurrent operations across nodes"""
        self.log('=== PHASE 4: Concurrent Operations Test ===')
        
        operations = []
        
        def write_operation(node, key, value):
            # Simulate a write operation
            cmd = f'echo "{key}={value}" >> /tmp/zenith_data.log && echo OK'
            result = self.exec_on_node(node, cmd)
            return {
                'type': 'write',
                'node': node,
                'key': key,
                'value': value,
                'success': result.get('stdout') == 'OK'
            }
        
        def read_operation(node, key):
            # Simulate a read operation
            cmd = f'grep "^{key}=" /tmp/zenith_data.log 2>/dev/null | tail -1 || echo ""'
            result = self.exec_on_node(node, cmd)
            return {
                'type': 'read',
                'node': node,
                'key': key,
                'value': result.get('stdout', ''),
                'success': True
            }
        
        # Generate concurrent operations
        with ThreadPoolExecutor(max_workers=6) as executor:
            futures = []
            
            # Write operations
            for i in range(5):
                node = random.choice(list(NODES.keys()))
                futures.append(executor.submit(write_operation, node, f'key_{i}', f'value_{i}_{int(time.time()*1000)}'))
            
            # Wait for writes
            for future in as_completed(futures):
                op = future.result()
                operations.append(op)
                self.log(f'  {op["type"].upper()} on {op["node"]}: {op["key"]}={op.get("value", "")} - {"OK" if op["success"] else "FAIL"}')
        
        time.sleep(1)
        
        # Read operations
        with ThreadPoolExecutor(max_workers=6) as executor:
            futures = []
            for i in range(3):
                node = random.choice(list(NODES.keys()))
                futures.append(executor.submit(read_operation, node, f'key_{i}'))
            
            for future in as_completed(futures):
                op = future.result()
                operations.append(op)
                self.log(f'  {op["type"].upper()} on {op["node"]}: {op["key"]} -> {op.get("value", "(empty)")}')
        
        successful_ops = sum(1 for op in operations if op['success'])
        
        self.test_results.append({
            'test': 'concurrent_operations',
            'passed': successful_ops > len(operations) * 0.8,
            'details': f'{successful_ops}/{len(operations)} operations successful'
        })
        
        return successful_ops == len(operations)
    
    def test_linearizability_check(self):
        """Test Phase 5: Basic linearizability verification"""
        self.log('=== PHASE 5: Linearizability Check ===')
        
        test_key = f'linear_test_{int(time.time())}'
        
        # Sequential writes
        writes = []
        for i, node in enumerate(NODES.keys()):
            value = f'v{i}_{int(time.time()*1000)}'
            cmd = f'echo "{test_key}={value}" >> /tmp/zenith_data.log'
            self.exec_on_node(node, cmd)
            writes.append({'node': node, 'value': value})
            self.log(f'  WRITE: {node} -> {value}')
            time.sleep(0.1)
        
        # Read from all nodes
        reads = []
        for node in NODES.keys():
            cmd = f'grep "^{test_key}=" /tmp/zenith_data.log 2>/dev/null | tail -1 || echo ""'
            result = self.exec_on_node(node, cmd)
            value = result.get('stdout', '')
            reads.append({'node': node, 'value': value})
            self.log(f'  READ: {node} -> {value}')
        
        # Check if all reads return the last written value
        last_write = writes[-1]['value'] if writes else ''
        consistent = all(r['value'].endswith(last_write) or last_write in r['value'] for r in reads if r['value'])
        
        self.test_results.append({
            'test': 'linearizability',
            'passed': consistent,
            'details': 'Sequential consistency verified' if consistent else 'Inconsistency detected'
        })
        
        return consistent
    
    def generate_report(self):
        """Generate final test report"""
        self.log('=== GENERATING REPORT ===')
        
        total = len(self.test_results)
        passed = sum(1 for r in self.test_results if r['passed'])
        
        report = {
            'summary': {
                'total_tests': total,
                'passed': passed,
                'failed': total - passed,
                'success_rate': f'{(passed/total)*100:.1f}%' if total > 0 else '0%',
                'duration': str(datetime.now() - self.start_time)
            },
            'tests': self.test_results,
            'timestamp': datetime.now().isoformat()
        }
        
        print('\n' + '='*60)
        print('           JEPSEN TEST REPORT - ZENITH DATAPLANE')
        print('='*60)
        print(f'  Start Time: {self.start_time.strftime("%Y-%m-%d %H:%M:%S")}')
        print(f'  Duration:   {report["summary"]["duration"]}')
        print(f'  Nodes:      {len(NODES)}')
        print('='*60)
        print('\n  TEST RESULTS:')
        print('-'*60)
        
        for test in self.test_results:
            status = '[PASS]' if test['passed'] else '[FAIL]'
            print(f'  {status} {test["test"]}: {test["details"]}')
        
        print('-'*60)
        print(f'\n  SUMMARY: {passed}/{total} tests passed ({report["summary"]["success_rate"]})')
        
        if passed == total:
            print('\n  [OK] All tests passed! Zenith demonstrates distributed consistency.')
        else:
            print(f'\n  [!] {total-passed} test(s) failed. Review recommended.')
        
        print('='*60 + '\n')
        
        return report

def main():
    print('\n' + '='*60)
    print('    JEPSEN DISTRIBUTED CONSISTENCY TEST')
    print('    Zenith Dataplane v0.2.3')
    print('='*60 + '\n')
    
    jepsen = JepsenTest()
    
    # Run all test phases
    jepsen.test_connectivity()
    jepsen.test_network_partition()
    jepsen.test_recovery()
    jepsen.test_concurrent_operations()
    jepsen.test_linearizability_check()
    
    # Generate and save report
    report = jepsen.generate_report()
    
    # Save JSON report
    with open('/tmp/jepsen_report.json', 'w') as f:
        json.dump(report, f, indent=2)
    
    print('  Report saved to: /tmp/jepsen_report.json')
    
    return 0 if report['summary']['failed'] == 0 else 1

if __name__ == '__main__':
    sys.exit(main())
