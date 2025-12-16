use crate::order_book::OrderBook;
use crate::types::{AggregatedDepthMap, OrderEvent, Side};
use parking_lot::RwLock;
use std::collections::BTreeMap;

/// An external cache service that maintains aggregated market depth.
///
/// This structure is completely decoupled from the core `OrderBook` and operates
/// on events published by the order book. It maintains its own state and locks,
/// allowing for maximum concurrency:
///
/// - Readers can query market depth without blocking order insertion
/// - Order insertion doesn't need to wait for depth aggregation
/// - The cache can be updated asynchronously after the core book is modified
///
/// ## Architecture
///
/// This follows the Observer Pattern:
/// 
/// - The `OrderBook` is the publisher (subject)
/// - The `MarketDepthCache` is the subscriber (observer)
/// - `OrderEvent` is the message passed between them
///
/// ## Thread Safety
///
/// The bid and ask depth maps are protected by separate `RwLock`s, allowing
/// concurrent reads and serialized writes. This structure can be safely shared
/// across threads using `Arc<MarketDepthCache>`.
#[derive(Debug)]
pub struct MarketDepthCache {
    /// Aggregated bid depth: maps aggregated price levels to total quantities
    aggregated_bid_depth: RwLock<AggregatedDepthMap>,
    /// Aggregated ask depth: maps aggregated price levels to total quantities
    aggregated_ask_depth: RwLock<AggregatedDepthMap>,
}

impl MarketDepthCache {
    /// Creates a new empty market depth cache.
    ///
    /// ## Examples
    ///
    /// ```
    /// use order_book::MarketDepthCache;
    ///
    /// let cache = MarketDepthCache::new();
    /// ```
    pub fn new() -> Self {
        MarketDepthCache {
            aggregated_bid_depth: RwLock::new(BTreeMap::new()),
            aggregated_ask_depth: RwLock::new(BTreeMap::new()),
        }
    }

    /// Processes an order event and updates the aggregated market depth.
    ///
    /// This method is called after an order is inserted into the order book.
    /// It aggregates the order price to its level and updates the cached quantity.
    ///
    /// The operation is $O(\log{N})$ where $N$ is the number of aggregated price levels.
    /// The lock is held only for the duration of the `BTreeMap` update.
    ///
    /// ## Arguments
    ///
    /// * `event`: The order event to process
    ///
    /// ## Examples
    ///
    /// ```
    /// use order_book::{OrderBook, MarketDepthCache, Order, Side};
    /// use rust_decimal::Decimal;
    ///
    /// let mut order_book = OrderBook::new();
    /// let cache = MarketDepthCache::new();
    ///
    /// let order = Order::new(100.50, 100, Side::Bid);
    ///
    /// let event = order_book.insert_order(order);
    /// cache.process_order_event(event);
    /// ```
    pub fn process_order_event(&self, event: OrderEvent) {
        // Aggregate the price to its level using the core book's logic
        let aggregated_price_level = OrderBook::aggregate_price_to_level(event.price);

        // Select the appropriate depth map based on side
        let mut depth_write_lock = match event.side {
            Side::Bid => self.aggregated_bid_depth.write(),
            Side::Ask => self.aggregated_ask_depth.write(),
        };

        // Update the aggregated quantity at this level
        *depth_write_lock
            .entry(aggregated_price_level)
            .or_insert(0) += event.quantity_delta;

        // Lock is automatically released here
    }

    /// Retrieves a snapshot of the current aggregated market depth.
    ///
    /// This method clones the current depth maps to provide a consistent snapshot.
    /// Multiple readers can call this method concurrently without blocking each other
    /// or blocking order insertion.
    ///
    /// The operation is $O(N)$ where $N$ is the number of aggregated price levels,
    /// due to the `BTreeMap` clone.
    ///
    /// ## Returns
    ///
    /// A tuple of `(bid_depth, ask_depth)` where each is an `AggregatedDepthMap`
    /// mapping aggregated price levels to total quantities.
    ///
    /// ## Examples
    ///
    /// ```
    /// use order_book::{OrderBook, MarketDepthCache, Order, Side};
    /// use rust_decimal::Decimal;
    ///
    /// let mut order_book = OrderBook::new();
    /// let cache = MarketDepthCache::new();
    ///
    /// let order = Order::new(100.50, 100, Side::Bid);
    ///
    /// let event = order_book.insert_order(order);
    /// cache.process_order_event(event);
    ///
    /// let (bid_depth, ask_depth) = cache.get_aggregated_market_depth();
    /// assert_eq!(bid_depth.get(&Decimal::new(100, 0)), Some(&100));
    /// ```
    pub fn get_aggregated_market_depth(&self) -> (AggregatedDepthMap, AggregatedDepthMap) {
        // Acquire read locks and clone the maps
        let bid_depth_snapshot = self.aggregated_bid_depth.read().clone();
        let ask_depth_snapshot = self.aggregated_ask_depth.read().clone();

        // Read locks are automatically released here
        (bid_depth_snapshot, ask_depth_snapshot)
    }

    /// Returns the total quantity at a specific aggregated price level.
    ///
    /// ## Arguments
    ///
    /// * `aggregated_level`: The aggregated price level to query
    /// * `side`: The side (bid or ask) to query
    ///
    /// ## Returns
    ///
    /// The total quantity at that level, or 0 if no orders exist
    ///
    /// ## Examples
    ///
    /// ```
    /// use order_book::{OrderBook, MarketDepthCache, Order, Side};
    /// use rust_decimal::Decimal;
    ///
    /// let mut order_book = OrderBook::new();
    /// let cache = MarketDepthCache::new();
    ///
    /// let order = Order::new(100.50, 100, Side::Bid);
    ///
    /// let event = order_book.insert_order(order);
    /// cache.process_order_event(event);
    ///
    /// let quantity = cache.get_quantity_at_level(Decimal::new(100, 0), Side::Bid);
    /// assert_eq!(quantity, 100);
    /// ```
    pub fn get_quantity_at_level(&self, aggregated_level: rust_decimal::Decimal, side: Side) -> u64 {
        let depth_read_lock = match side {
            Side::Bid => self.aggregated_bid_depth.read(),
            Side::Ask => self.aggregated_ask_depth.read(),
        };

        depth_read_lock.get(&aggregated_level).copied().unwrap_or(0)
    }

    /// Returns the number of aggregated price levels on the bid side.
    pub fn bid_levels_count(&self) -> usize {
        self.aggregated_bid_depth.read().len()
    }

    /// Returns the number of aggregated price levels on the ask side.
    pub fn ask_levels_count(&self) -> usize {
        self.aggregated_ask_depth.read().len()
    }

    /// Clears all cached market depth data.
    ///
    /// This is useful for testing or resetting the cache state.
    pub fn clear(&self) {
        self.aggregated_bid_depth.write().clear();
        self.aggregated_ask_depth.write().clear();
    }
}

impl Default for MarketDepthCache {
    fn default() -> Self {
        Self::new()
    }
}
