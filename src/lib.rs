//! A high-performance tested and benchmarked price-priority order-book implementation
//! using an external cache to minimize lock contention and maximize concurrency.
//!
//! ## Architecture
//!
//! This library separates concerns into two independent services:
//!
//! 1. `OrderBook`: The core order book that maintains price-time priority
//! 2. `MarketDepthCache`: An external cache that aggregates market depth
//!
//! These services communicate via events (`OrderEvent`), allowing them to operate
//! with separate locks and enabling high concurrency for readers and writers.
//!
//! ## Example Usage
//!
//! ```rust
//! use order_book::{OrderBook, MarketDepthCache, Order, Side};
//! use rust_decimal::Decimal;
//! use parking_lot::RwLock;
//! use std::sync::Arc;
//!
//! // Create the order book and cache
//! let order_book = Arc::new(RwLock::new(OrderBook::new()));
//! let market_depth_cache = Arc::new(MarketDepthCache::new());
//!
//! // Insert an order
//! let order = Order::new(100.50, 100, Side::Bid);
//!
//! // 1. Acquire write lock briefly to insert order
//! let event = {
//!     let mut book = order_book.write();
//!     book.insert_order(order)
//! }; // Write lock released immediately
//!
//! // 2. Update cache (uses its own lock)
//! market_depth_cache.process_order_event(event);
//!
//! // 3. Query spread (read lock on order book)
//! let (best_bid, best_ask, spread) = order_book.read().compute_spread();
//!
//! // 4. Query market depth (read lock on cache)
//! let (bid_depth, ask_depth) = market_depth_cache.get_aggregated_market_depth();
//! ```
//!
//! The order-book and the cache use separate locks, which means that multiple
//! readers can access the book and cache simultaneously without blocking each other.
//!
//! Also, on the performance side, order insertions only hold the lock for a brief period,
//! that is a $O(\log{N})$, because we're relying on the `BTreeMap`'s efficient insertions.
//!
//! Lastly, the cache is updated asynchronously, which means that it does not block the order book.
//! This allows for high concurrency and responsiveness in the order book.

mod market_depth_cache;
mod order_book;
mod types;

// Re-export public API
pub use market_depth_cache::MarketDepthCache;
pub use order_book::OrderBook;
pub use types::{AggregatedDepthMap, Order, OrderEvent, ExactPriceLevelMap, Side};

// Re-export commonly used external dependencies
pub use parking_lot::RwLock;
pub use rust_decimal::Decimal;
