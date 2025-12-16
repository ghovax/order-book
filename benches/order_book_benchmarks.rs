use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use order_book::{Decimal, MarketDepthCache, Order, OrderBook, Side};
use parking_lot::RwLock;
use std::sync::Arc;

/// Benchmark the performance of inserting a single order into the core order book. No cache update.
fn benchmark_single_order_insertion(criterion: &mut Criterion) {
    let mut benchmark_group = criterion.benchmark_group("order_insertion");

    benchmark_group.bench_function("insert_single_bid_order", |bencher| {
        let mut order_book = OrderBook::new();
        let mut price_counter = 100.0;

        bencher.iter(|| {
            let order = Order::new(price_counter, 100, Side::Bid);
            let event = order_book.insert_order(order);
            black_box(event);
            price_counter += 0.01; // Ensure unique prices
        });
    });

    benchmark_group.bench_function("insert_single_ask_order", |bencher| {
        let mut order_book = OrderBook::new();
        let mut price_counter = 100.0;

        bencher.iter(|| {
            let order = Order::new(price_counter, 100, Side::Ask);
            let event = order_book.insert_order(order);
            black_box(event);
            price_counter += 0.01;
        });
    });

    benchmark_group.finish();
}

/// Benchmark the performance of inserting a single order into the core order book and updating the cache.
fn benchmark_order_insertion_with_cache(criterion: &mut Criterion) {
    let mut benchmark_group = criterion.benchmark_group("order_insertion_with_cache");

    benchmark_group.bench_function("insert_and_update_cache", |bencher| {
        let mut order_book = OrderBook::new();
        let market_depth_cache = MarketDepthCache::new();
        let mut price_counter = 100.0;

        bencher.iter(|| {
            let order = Order::new(price_counter, 100, Side::Bid);
            let event = order_book.insert_order(order);
            market_depth_cache.process_order_event(event);
            price_counter += 0.01;
        });
    });

    benchmark_group.finish();
}

/// Benchmark the performance of computing the spread at various book sizes.
fn benchmark_spread_computation(criterion: &mut Criterion) {
    let mut benchmark_group = criterion.benchmark_group("spread_computation");

    for book_size in [100, 1_000, 10_000, 100_000] {
        benchmark_group.throughput(Throughput::Elements(1));

        // Pre-populate the order book
        let mut order_book = OrderBook::new();
        for i in 0..book_size {
            let bid_price = 100.0 - (i as f64 * 0.01);
            let ask_price = 101.0 + (i as f64 * 0.01);
            order_book.insert_order(Order::new(bid_price, 100, Side::Bid));
            order_book.insert_order(Order::new(ask_price, 100, Side::Ask));
        }

        benchmark_group.bench_with_input(
            BenchmarkId::new("compute_spread", book_size),
            &order_book,
            |bencher, book| {
                bencher.iter(|| {
                    let spread = book.compute_spread();
                    black_box(spread);
                });
            },
        );
    }

    benchmark_group.finish();
}

/// Benchmark the performance of retrieving market depth from the cache.
fn benchmark_market_depth_retrieval(criterion: &mut Criterion) {
    let mut benchmark_group = criterion.benchmark_group("market_depth_retrieval");

    for cache_size in [100, 1_000, 10_000, 100_000] {
        benchmark_group.throughput(Throughput::Elements(1));

        // Pre-populate the cache
        let mut order_book = OrderBook::new();
        let market_depth_cache = MarketDepthCache::new();

        for i in 0..cache_size {
            let bid_price = 100.0 - (i as f64 * 0.01);
            let ask_price = 101.0 + (i as f64 * 0.01);

            let bid_event = order_book.insert_order(Order::new(bid_price, 100, Side::Bid));
            let ask_event = order_book.insert_order(Order::new(ask_price, 100, Side::Ask));

            market_depth_cache.process_order_event(bid_event);
            market_depth_cache.process_order_event(ask_event);
        }

        benchmark_group.bench_with_input(
            BenchmarkId::new("get_market_depth", cache_size),
            &market_depth_cache,
            |bencher, cache| {
                bencher.iter(|| {
                    let depth = cache.get_aggregated_market_depth();
                    black_box(depth);
                });
            },
        );
    }

    benchmark_group.finish();
}

/// Benchmark the performance of computing the spread concurrently.
fn benchmark_concurrent_spread_reads(criterion: &mut Criterion) {
    let mut benchmark_group = criterion.benchmark_group("concurrent_spread_reads");

    // Pre-populate a large order book
    let mut order_book = OrderBook::new();
    for i in 0..10_000 {
        let bid_price = 100.0 - (i as f64 * 0.01);
        let ask_price = 101.0 + (i as f64 * 0.01);
        order_book.insert_order(Order::new(bid_price, 100, Side::Bid));
        order_book.insert_order(Order::new(ask_price, 100, Side::Ask));
    }

    let order_book_arc = Arc::new(RwLock::new(order_book));

    for threads_count in [1, 2, 4, 8] {
        benchmark_group.bench_with_input(
            BenchmarkId::new("concurrent_reads", threads_count),
            &threads_count,
            |bencher, &thread_count| {
                bencher.iter(|| {
                    let mut thread_handles = vec![];

                    for _ in 0..thread_count {
                        let book_clone = Arc::clone(&order_book_arc);
                        thread_handles.push(std::thread::spawn(move || {
                            for _ in 0..100 {
                                let spread = book_clone.read().compute_spread();
                                black_box(spread);
                            }
                        }));
                    }

                    for handle in thread_handles {
                        handle.join().unwrap();
                    }
                });
            },
        );
    }

    benchmark_group.finish();
}

/// Benchmark the performance of retrieving market depth concurrently.
fn benchmark_concurrent_depth_reads(criterion: &mut Criterion) {
    let mut benchmark_group = criterion.benchmark_group("concurrent_depth_reads");

    // Pre-populate cache
    let mut order_book = OrderBook::new();
    let market_depth_cache = Arc::new(MarketDepthCache::new());

    for i in 0..10_000 {
        let bid_price = 100.0 - (i as f64 * 0.01);
        let ask_price = 101.0 + (i as f64 * 0.01);

        let bid_event = order_book.insert_order(Order::new(bid_price, 100, Side::Bid));
        let ask_event = order_book.insert_order(Order::new(ask_price, 100, Side::Ask));

        market_depth_cache.process_order_event(bid_event);
        market_depth_cache.process_order_event(ask_event);
    }

    for threads_count in [1, 2, 4, 8] {
        benchmark_group.bench_with_input(
            BenchmarkId::new("concurrent_depth_reads", threads_count),
            &threads_count,
            |bencher, &thread_count| {
                bencher.iter(|| {
                    let mut thread_handles = vec![];

                    for _ in 0..thread_count {
                        let cache_clone = Arc::clone(&market_depth_cache);
                        thread_handles.push(std::thread::spawn(move || {
                            for _ in 0..100 {
                                let depth = cache_clone.get_aggregated_market_depth();
                                black_box(depth);
                            }
                        }));
                    }

                    for handle in thread_handles {
                        handle.join().unwrap();
                    }
                });
            },
        );
    }

    benchmark_group.finish();
}

/// Benchmark the performance of a mixed workload of writes and reads.
fn benchmark_mixed_workload(criterion: &mut Criterion) {
    let mut benchmark_group = criterion.benchmark_group("mixed_workload");

    // First, run a full write-heavy workload...
    benchmark_group.bench_function("write_heavy_workload_90_10", |bencher| {
        let order_book_arc = Arc::new(RwLock::new(OrderBook::new()));
        let market_depth_cache_arc = Arc::new(MarketDepthCache::new());

        bencher.iter(|| {
            let book_clone = Arc::clone(&order_book_arc);
            let cache_clone = Arc::clone(&market_depth_cache_arc);

            let mut thread_handles = vec![];

            // Writer thread (90% of operations)
            for _ in 0..9 {
                let book = Arc::clone(&book_clone);
                let cache = Arc::clone(&cache_clone);
                thread_handles.push(std::thread::spawn(move || {
                    for i in 0..10 {
                        let price = 100.0 + (i as f64 * 0.01);
                        let event = {
                            let mut book_lock = book.write();
                            book_lock.insert_order(Order::new(price, 100, Side::Bid))
                        };
                        cache.process_order_event(event);
                    }
                }));
            }

            // Reader thread (10% of operations)
            for _ in 0..1 {
                let book = Arc::clone(&book_clone);
                let cache = Arc::clone(&cache_clone);
                thread_handles.push(std::thread::spawn(move || {
                    for _ in 0..10 {
                        let spread = book.read().compute_spread();
                        let depth = cache.get_aggregated_market_depth();
                        black_box((spread, depth));
                    }
                }));
            }

            for handle in thread_handles {
                handle.join().unwrap();
            }
        });
    });

    // ...then run a full read-heavy workload
    benchmark_group.bench_function("read_heavy_workload_10_90", |bencher| {
        // Pre-populate with data
        let order_book_arc = Arc::new(RwLock::new(OrderBook::new()));
        let market_depth_cache_arc = Arc::new(MarketDepthCache::new());

        {
            let mut book = order_book_arc.write();
            for i in 0..1000 {
                let price = 100.0 + (i as f64 * 0.01);
                let event = book.insert_order(Order::new(price, 100, Side::Bid));
                market_depth_cache_arc.process_order_event(event);
            }
        }

        bencher.iter(|| {
            let book_clone = Arc::clone(&order_book_arc);
            let cache_clone = Arc::clone(&market_depth_cache_arc);

            let mut thread_handles = vec![];

            // Writer threads (10% of operations)
            for _ in 0..1 {
                let book = Arc::clone(&book_clone);
                let cache = Arc::clone(&cache_clone);
                thread_handles.push(std::thread::spawn(move || {
                    for i in 0..10 {
                        let price = 200.0 + (i as f64 * 0.01);
                        let event = {
                            let mut book_lock = book.write();
                            book_lock.insert_order(Order::new(price, 100, Side::Bid))
                        };
                        cache.process_order_event(event);
                    }
                }));
            }

            // Reader threads (90% of operations)
            for _ in 0..9 {
                let book = Arc::clone(&book_clone);
                let cache = Arc::clone(&cache_clone);
                thread_handles.push(std::thread::spawn(move || {
                    for _ in 0..10 {
                        let spread = book.read().compute_spread();
                        let depth = cache.get_aggregated_market_depth();
                        black_box((spread, depth));
                    }
                }));
            }

            for handle in thread_handles {
                handle.join().unwrap();
            }
        });
    });

    benchmark_group.finish();
}

/// Benchmark the performance of processing order events in the cache.
fn benchmark_cache_event_processing(criterion: &mut Criterion) {
    let mut benchmark_group = criterion.benchmark_group("cache_event_processing");

    // Iterate over a range of event counts
    for event_count in [100, 1_000, 10_000] {
        benchmark_group.throughput(Throughput::Elements(event_count));

        benchmark_group.bench_with_input(
            BenchmarkId::new("process_events", event_count),
            &event_count,
            |bencher, &event_count| {
                bencher.iter(|| {
                    let mut order_book = OrderBook::new();
                    let market_depth_cache = MarketDepthCache::new();

                    for i in 0..event_count {
                        let price = 100.0 + (i as f64 * 0.01);
                        let side = if i % 2 == 0 { Side::Bid } else { Side::Ask };
                        let event = order_book.insert_order(Order::new(price, 100, side));
                        market_depth_cache.process_order_event(event);
                    }

                    black_box(market_depth_cache);
                });
            },
        );
    }

    benchmark_group.finish();
}

// Define the benchmarks group to generate the reports automatically
criterion_group!(
    benches,
    benchmark_single_order_insertion,
    benchmark_order_insertion_with_cache,
    benchmark_spread_computation,
    benchmark_market_depth_retrieval,
    benchmark_concurrent_spread_reads,
    benchmark_concurrent_depth_reads,
    benchmark_mixed_workload,
    benchmark_cache_event_processing,
);

criterion_main!(benches);
