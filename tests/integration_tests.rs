use order_book::{Decimal, MarketDepthCache, Order, OrderBook, Side};
use parking_lot::RwLock;
use std::sync::Arc;

#[test]
/// Test the order of insertion and the computation of spread by running a workflow.
fn test_order_insertion_and_spread_computation() {
    let mut order_book = OrderBook::new();
    let market_depth_cache = MarketDepthCache::new();

    // Test 1: Insert a Bid (Buy) order
    let order = Order::new(99.50, 10, Side::Bid);
    let event = order_book.insert_order(order);
    market_depth_cache.process_order_event(event);
    let (best_bid, best_ask, _) = order_book.compute_spread();
    assert_eq!(
        best_bid,
        Some(Decimal::try_from(99.50).unwrap().normalize()),
        "Best bid should be 99.50"
    );
    assert!(
        best_ask.is_none(),
        "Ask should be `None` (no sell orders yet)"
    );

    // Insert another Bid at a lower price
    let order = Order::new(99.00, 5, Side::Bid);
    let event = order_book.insert_order(order);
    market_depth_cache.process_order_event(event);
    let (best_bid, _, _) = order_book.compute_spread();
    assert_eq!(
        best_bid,
        Some(Decimal::try_from(99.50).unwrap().normalize()),
        "Best bid should still be 99.50 (price priority)"
    );

    // Insert an ask (sell) order
    let order = Order::new(100.25, 20, Side::Ask);
    let event = order_book.insert_order(order);
    market_depth_cache.process_order_event(event);
    let (best_bid, best_ask, _) = order_book.compute_spread();
    assert_eq!(
        best_bid,
        Some(Decimal::try_from(99.50).unwrap().normalize()),
        "Best bid should be 99.50"
    );
    assert_eq!(
        best_ask,
        Some(Decimal::try_from(100.25).unwrap().normalize()),
        "Best ask should be 100.25"
    );

    // Insert another ask at a lower price (becomes new best ask)
    let order = Order::new(100.10, 30, Side::Ask);
    let event = order_book.insert_order(order);
    market_depth_cache.process_order_event(event);
    let (_, best_ask, _) = order_book.compute_spread();
    assert_eq!(
        best_ask,
        Some(Decimal::try_from(100.10).unwrap().normalize()),
        "New best ask should be 100.10"
    );

    // Verify time priority within a price level
    assert_eq!(
        order_book.orders_at_exact_price_level(Decimal::try_from(100.25).unwrap().normalize(), Side::Ask),
        1,
        "Should have one order at 100.25"
    );
}

#[test]
/// Test market depth aggregation logic by inserting orders at different price levels.
fn test_market_depth_aggregation_logic() {
    let mut order_book = OrderBook::new();
    let market_depth_cache = MarketDepthCache::new();

    // Aggregating binds at level 99
    // Insert order at 99.50 (aggregates to 99)
    for (price, quantity) in [(99.50, 10), (99.01, 5)] {
        let order = Order::new(price, quantity, Side::Bid);
        let event = order_book.insert_order(order);
        market_depth_cache.process_order_event(event);
    }
    // Total bid level 99 should be 10 + 5 = 15

    // Aggregating asks at level 100
    for (price, quantity) in [(100.25, 20), (100.99, 3)] {
        let order = Order::new(price, quantity, Side::Ask);
        let event = order_book.insert_order(order);
        market_depth_cache.process_order_event(event);
    }
    // Total ask level 100 should be 20 + 3 = 23

    // Running a cross-level check at level 101
    let order = Order::new(101.00, 50, Side::Ask);
    let event = order_book.insert_order(order);
    market_depth_cache.process_order_event(event);

    let (bid_depth, ask_depth) = market_depth_cache.get_aggregated_market_depth();

    // Check Bid Depth
    assert_eq!(
        *bid_depth
            .get(&Decimal::try_from(99.0).unwrap().normalize())
            .unwrap(),
        15,
        "Bid depth at 99.0 should be 15"
    );
    assert!(
        bid_depth
            .get(&Decimal::try_from(100.0).unwrap().normalize())
            .is_none(),
        "No bids should be aggregated at level 100"
    );

    // Check Ask Depth
    assert_eq!(
        *ask_depth
            .get(&Decimal::try_from(100.0).unwrap().normalize())
            .unwrap(),
        23,
        "Ask depth at 100.0 should be 23"
    );
    assert_eq!(
        *ask_depth
            .get(&Decimal::try_from(101.0).unwrap().normalize())
            .unwrap(),
        50,
        "Ask depth at 101.0 should be 50"
    );

    // Ensure no other unexpected levels exist
    assert!(bid_depth
        .get(&Decimal::try_from(98.0).unwrap().normalize())
        .is_none());
}

#[test]
/// Test that the market depth cache correctly aggregates orders at the same price level.
fn test_decimal_precision() {
    let mut order_book = OrderBook::new();
    let market_depth_cache = MarketDepthCache::new();

    // Test prices that might cause f64 issues but must be precise with `Decimal`
    for (price, quantity) in [(100.00, 1), (100.01, 2), (99.99, 3)] {
        let order = Order::new(price, quantity, Side::Bid);
        let event = order_book.insert_order(order);
        market_depth_cache.process_order_event(event);
    }

    let (best_bid, _, _) = order_book.compute_spread();
    // 100.01 is the highest price
    assert_eq!(
        best_bid,
        Some(Decimal::try_from(100.01).unwrap().normalize()),
        "Best Bid must be 100.01 due to `Decimal` precision"
    );

    // Check aggregation logic on these precise values
    let (bid_depth, _) = market_depth_cache.get_aggregated_market_depth();
    // 100.00 and 100.01 aggregate to 100.0, with depth 1 + 2 = 3
    // 99.99 aggregates to 99.0, with depth 3

    assert_eq!(
        *bid_depth
            .get(&Decimal::try_from(100.0).unwrap().normalize())
            .unwrap(),
        3,
        "Bid depth at 100.0 should be 3 (100.00 + 100.01)"
    );
    assert_eq!(
        *bid_depth
            .get(&Decimal::try_from(99.0).unwrap().normalize())
            .unwrap(),
        3,
        "Bid depth at 99.0 should be 3 (99.99)"
    );
}

#[test]
/// Test that the market depth cache correctly handles concurrent access.
fn test_concurrent_access_smoke_test() {
    use std::thread;

    // Wrap the book and cache in Arc for sharing across threads
    let order_book_arc = Arc::new(RwLock::new(OrderBook::new()));
    let market_depth_cache_arc = Arc::new(MarketDepthCache::new());

    let mut thread_handles = vec![];
    let orders_per_thread = 1000;
    let number_of_threads = 4;

    for thread_id in 0..number_of_threads {
        let book_clone = Arc::clone(&order_book_arc);
        let cache_clone = Arc::clone(&market_depth_cache_arc);

        thread_handles.push(thread::spawn(move || {
            for order_index in 0..orders_per_thread {
                let price = 100.00 + (thread_id as f64) * 0.01 + (order_index as f64) * 0.001;
                let quantity = 1;
                let side = if (thread_id + order_index) % 2 == 0 {
                    Side::Bid
                } else {
                    Side::Ask
                };

                // 1. Writer acquires book lock briefly
                let event = {
                    let mut book = book_clone.write();
                    let order = Order::new(price, quantity, side);
                    book.insert_order(order)
                }; // Book write lock released

                // 2. Writer acquires cache lock
                cache_clone.process_order_event(event); // Cache lock released

                // 3. Reader checks spread (acquires book read lock)
                let _spread = book_clone.read().compute_spread();

                // 4. Reader checks depth (acquires cache read lock)
                let _depth = cache_clone.get_aggregated_market_depth();
            }
        }));
    }

    // Wait for all threads to finish
    for thread_handle in thread_handles {
        thread_handle.join().unwrap();
    }

    // As a final validation, check the total quantity across all depths
    let total_inserted_quantity = (orders_per_thread * number_of_threads) as u64;
    let (bid_depth, ask_depth) = market_depth_cache_arc.get_aggregated_market_depth();

    let total_cached_quantity: u64 =
        bid_depth.values().sum::<u64>() + ask_depth.values().sum::<u64>();

    // This validates that every order was processed by the cache correctly
    assert_eq!(
        total_cached_quantity, total_inserted_quantity,
        "Total quantity in cache must match total inserted orders"
    );
}

#[test]
/// Test what happens when the order book is empty.
fn test_empty_order_book() {
    let order_book = OrderBook::new();

    let (best_bid, best_ask, _) = order_book.compute_spread();
    assert!(
        best_bid.is_none(),
        "Best bid should be `None` for empty book"
    );
    assert!(
        best_ask.is_none(),
        "Best ask should be `None` for empty book"
    );

    assert_eq!(order_book.bid_levels_count(), 0);
    assert_eq!(order_book.ask_levels_count(), 0);
}

#[test]
/// Test different cache level queries results.
fn test_cache_level_queries() {
    let mut order_book = OrderBook::new();
    let market_depth_cache = MarketDepthCache::new();

    for (price, quantity, side) in [(99.50, 10, Side::Bid), (100.25, 20, Side::Ask)] {
        let order = Order::new(price, quantity, side);
        let event = order_book.insert_order(order);
        market_depth_cache.process_order_event(event);
    }

    // Test individual level queries
    assert_eq!(
        market_depth_cache
            .get_quantity_at_level(Decimal::try_from(99.0).unwrap().normalize(), Side::Bid),
        10,
        "Bid quantity at level 99 should be 10"
    );
    assert_eq!(
        market_depth_cache
            .get_quantity_at_level(Decimal::try_from(100.0).unwrap().normalize(), Side::Ask),
        20,
        "Ask quantity at level 100 should be 20"
    );
    assert_eq!(
        market_depth_cache
            .get_quantity_at_level(Decimal::try_from(98.0).unwrap().normalize(), Side::Bid),
        0,
        "Non-existent level should return 0"
    );
}

#[test]
/// Test when multiple orders are inserted at the same price level.
fn test_multiple_orders_same_price_level() {
    let mut order_book = OrderBook::new();
    let market_depth_cache = MarketDepthCache::new();

    // Insert multiple orders at the same price level
    for quantity in [10, 20, 30] {
        let order = Order::new(100.00, quantity, Side::Bid);
        let event = order_book.insert_order(order);
        market_depth_cache.process_order_event(event);
    }

    // Verify the order book maintains all orders
    assert_eq!(
        order_book.orders_at_exact_price_level(Decimal::try_from(100.00).unwrap().normalize(), Side::Bid),
        3,
        "Should have 3 orders at price level 100.00"
    );

    // Verify the cache aggregates correctly
    assert_eq!(
        market_depth_cache
            .get_quantity_at_level(Decimal::try_from(100.0).unwrap().normalize(), Side::Bid),
        60,
        "Aggregated quantity should be 10 + 20 + 30 = 60"
    );
}

#[test]
/// Test if clearing the cache works as expected.
fn test_cache_clear() {
    let mut order_book = OrderBook::new();
    let market_depth_cache = MarketDepthCache::new();

    for (price, quantity, side) in [(99.50, 10, Side::Bid), (100.25, 20, Side::Ask)] {
        let order = Order::new(price, quantity, side);
        let event = order_book.insert_order(order);
        market_depth_cache.process_order_event(event);
    }

    assert_eq!(market_depth_cache.bid_levels_count(), 1);
    assert_eq!(market_depth_cache.ask_levels_count(), 1);

    market_depth_cache.clear();

    assert_eq!(market_depth_cache.bid_levels_count(), 0);
    assert_eq!(market_depth_cache.ask_levels_count(), 0);
}

#[test]
/// Test the price aggregation by taking some boundary cases.
fn test_price_aggregation_boundary_cases() {
    let mut order_book = OrderBook::new();
    let market_depth_cache = MarketDepthCache::new();

    // Test boundary cases for aggregation
    for (price, quantity) in [(99.00, 1), (99.99, 2), (100.00, 3), (100.01, 4)] {
        let order = Order::new(price, quantity, Side::Bid);
        let event = order_book.insert_order(order);
        market_depth_cache.process_order_event(event);
    }

    let (bid_depth, _) = market_depth_cache.get_aggregated_market_depth();

    // 99.00 and 99.99 should aggregate to 99
    assert_eq!(
        *bid_depth
            .get(&Decimal::try_from(99.0).unwrap().normalize())
            .unwrap(),
        3,
        "Level 99 should have 1 + 2 = 3"
    );

    // 100.00 and 100.01 should aggregate to 100
    assert_eq!(
        *bid_depth
            .get(&Decimal::try_from(100.0).unwrap().normalize())
            .unwrap(),
        7,
        "Level 100 should have 3 + 4 = 7"
    );
}
