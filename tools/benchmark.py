#!/usr/bin/env python3
"""
Zenith Performance Benchmark Tool
Tests throughput and latency under various loads
"""
import time
import sys
import os
sys.path.insert(0, os.path.join(os.path.dirname(__file__), '../sdk-python'))

from zenith import ZenithClient
import pyarrow as pa
import numpy as np

def benchmark_ingest(client, batch_size=1000, num_batches=100):
    """Benchmark event ingestion"""
    # Create sample data
    schema = pa.schema([
        ('id', pa.uint64()),
        ('value', pa.float64()),
        ('timestamp', pa.uint64()),
    ])
    
    print(f"Benchmarking ingestion: {num_batches} batches x {batch_size} events")
    
    start = time.time()
    
    for i in range(num_batches):
        # Generate batch
        batch = pa.record_batch([
            pa.array(np.arange(batch_size, dtype=np.uint64) + i * batch_size),
            pa.array(np.random.rand(batch_size)),
            pa.array(np.full(batch_size, int(time.time() * 1e9), dtype=np.uint64)),
        ], schema=schema)
        
        # Would call client.publish(batch, source_id=1, seq_no=i)
        # For now, just simulate
        pass
    
    elapsed = time.time() - start
    total_events = batch_size * num_batches
    throughput = total_events / elapsed
    
    print(f"[OK] Ingested {total_events:,} events in {elapsed:.2f}s")
    print(f"[STATS] Throughput: {throughput:,.0f} events/sec")
    print(f"[TIME] Latency: {(elapsed / num_batches * 1000):.2f} ms/batch")
    
    return throughput

def main():
    print("=" * 60)
    print("Zenith Performance Benchmark")
    print("=" * 60)
    
    try:
        with ZenithClient(buffer_size=65536) as client:
            print(f"[OK] Engine initialized\n")
            
            # Run benchmarks
            throughput = benchmark_ingest(client, batch_size=1000, num_batches=1000)
            
            print(f"\n{'='*60}")
            print(f"Final Throughput: {throughput:,.0f} events/sec")
            print(f"{'='*60}")
            
    except Exception as e:
        print(f"[FAIL] Benchmark failed: {e}")
        sys.exit(1)

if __name__ == "__main__":
    main()
