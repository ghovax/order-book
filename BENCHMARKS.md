# Order Book Benchmarks

This document describes the comprehensive benchmark suite for the order book implementation.

## Running Benchmarks

To run all benchmarks:

```bash
cargo bench
```

To run a specific benchmark group:

```bash
cargo bench --bench order_book_benchmarks -- <benchmark_name>
```

For example:

```bash
cargo bench --bench order_book_benchmarks -- order_insertion
cargo bench --bench order_book_benchmarks -- concurrent_spread_reads
```

To run benchmarks with verbose output:

```bash
cargo bench -- --verbose
```

## Benchmark Results Location

After running benchmarks, Criterion generates detailed HTML reports in:

```
target/criterion/
```

Open `target/criterion/report/index.html` in a browser to view interactive performance graphs and statistical analysis.

## Benchmark Suite Overview

### 1. Single Order Insertion (`order_insertion`)

**Purpose**: Measures the raw performance of inserting orders into the core order book without cache updates.

**Operations Measured**:

- `insert_single_bid_order`: Inserting buy orders
- `insert_single_ask_order`: Inserting sell orders

**What This Tests**:

- BTreeMap insertion performance (O(log N))
- Lock acquisition overhead
- Memory allocation for new price levels

**Expected Performance**: Should be very fast (microseconds) as it's just a BTreeMap operation.

---

### 2. Order Insertion with Cache Update (`order_insertion_with_cache`)

**Purpose**: Measures the complete order insertion pipeline including cache updates.

**Operations Measured**:

- `insert_and_update_cache`: Full workflow of insert → event generation → cache update

**What This Tests**:

- End-to-end latency for the complete order processing pipeline
- Combined overhead of book insertion + cache aggregation

**Expected Performance**: Should be slightly slower than single insertion but still very fast.

---

### 3. Spread Computation (`spread_computation`)

**Purpose**: Measures the performance of computing the best bid/ask spread at various order book sizes.

**Test Sizes**: 100, 1,000, 10,000, 100,000 orders

**What This Tests**:

- O(1) access time for best bid/ask (BTreeMap first/last key)
- Whether performance degrades with larger order books (it shouldn't)

**Expected Performance**: Should be constant time O(1) regardless of book size.

---

### 4. Market Depth Retrieval (`market_depth_retrieval`)

**Purpose**: Measures the performance of retrieving the complete aggregated market depth snapshot.

**Test Sizes**: 100, 1,000, 10,000, 100,000 aggregated levels

**What This Tests**:

- BTreeMap cloning performance (O(N))
- Read lock acquisition
- Memory allocation for the snapshot

**Expected Performance**: Should scale linearly with the number of aggregated price levels.

---

### 5. Concurrent Spread Reads (`concurrent_spread_reads`)

**Purpose**: Measures the scalability of concurrent read operations on the order book.

**Thread Counts**: 1, 2, 4, 8 concurrent readers

**What This Tests**:

- RwLock read concurrency (readers should not block each other)
- Cache line contention
- Scalability with multiple CPU cores

**Expected Performance**: Should scale near-linearly with thread count (read operations are concurrent).

---

### 6. Concurrent Depth Reads (`concurrent_depth_reads`)

**Purpose**: Measures the scalability of concurrent market depth queries.

**Thread Counts**: 1, 2, 4, 8 concurrent readers

**What This Tests**:

- Cache RwLock read concurrency
- Memory bandwidth for cloning depth maps
- Independence of book and cache locks

**Expected Performance**: Should scale well with thread count, demonstrating the benefit of separate locks.

---

### 7. Mixed Workload (`mixed_workload`)

**Purpose**: Simulates realistic trading scenarios with both readers and writers.

**Scenarios**:

- `write_heavy_workload_90_10`: 90% writes, 10% reads (order entry heavy)
- `read_heavy_workload_10_90`: 10% writes, 90% reads (market data query heavy)

**What This Tests**:

- Lock contention under realistic conditions
- Write/read balance impact on throughput
- Whether the external cache pattern reduces contention

**Expected Performance**:

- Write-heavy: Should handle high order insertion rates
- Read-heavy: Should demonstrate excellent read scalability due to separate locks

---

### 8. Cache Event Processing (`cache_event_processing`)

**Purpose**: Measures the throughput of processing order events through the cache.

**Event Counts**: 100, 1,000, 10,000 events

**What This Tests**:

- End-to-end system throughput
- Combined book + cache performance
- Aggregation logic efficiency

**Expected Performance**: Should scale linearly with the number of events.

---

## Performance Characteristics Summary

| Operation          | Time Complexity | Expected Latency  |
| ------------------ | --------------- | ----------------- |
| Order Insertion    | O(log N)        | < 1 µs            |
| Cache Update       | O(log M)        | < 1 µs            |
| Spread Computation | O(1)            | < 100 ns          |
| Depth Retrieval    | O(M)            | < 10 µs (small M) |
| Concurrent Reads   | O(1) per thread | Scales linearly   |

Where:

- N = number of distinct price levels in the order book
- M = number of aggregated price levels in the cache

## Architecture Benefits Demonstrated

These benchmarks demonstrate the key benefits of the external cache / observer pattern:

1. **Minimal Lock Contention**: Separate locks for book and cache allow concurrent operations
2. **Fast Writes**: Order insertion is O(log N) and doesn't block on cache aggregation
3. **Scalable Reads**: Multiple readers can query book and cache simultaneously
4. **Predictable Performance**: Performance scales predictably with data size

## Interpreting Results

When analyzing benchmark results, look for:

1. **Consistency**: Low variance across iterations indicates predictable performance
2. **Scalability**: Linear or better scaling with increased concurrency
3. **Constant Time Operations**: Spread computation should not vary with book size
4. **Throughput**: Events/second should be high for all operations

## Comparison with Alternative Architectures

The external cache pattern should show:

- **Better read scalability** compared to a single-lock design
- **Lower write latency** compared to inline cache updates
- **Higher throughput** in mixed workloads compared to synchronized designs

## Future Benchmark Ideas

Potential additions to the benchmark suite:

1. Order cancellation performance
2. Order modification (price/quantity updates)
3. Memory usage profiling
4. Cache invalidation scenarios
5. Large order batch processing
6. Realistic market data replay

## Hardware Considerations

Benchmark results will vary based on:

- CPU core count (affects concurrency benchmarks)
- CPU cache size (affects large book benchmarks)
- Memory bandwidth (affects depth retrieval)
- CPU frequency (affects single-threaded operations)

Run benchmarks on production-like hardware for accurate performance projections.
