use crate::types::{Order, OrderEvent, PriceLevelMap, Side};
use rust_decimal::Decimal;
use std::collections::BTreeMap;

/// The core order book structure that maintains price-time priority.
///
/// This structure is responsible only for:
/// 
/// - Storing orders at each price level
/// - Maintaining price priority (best bid/ask)
/// - Publishing events when orders are inserted
///
/// It does not maintain aggregated market depth, as that is handled by the external
/// `MarketDepthCache` service to minimize lock contention.
///
/// ### Thread Safety
///
/// This structure is designed to be wrapped in a `RwLock` for concurrent access.
/// The write lock should be held only briefly during order insertion.
#[derive(Debug)]
pub struct OrderBook {
    /// Ask side (sell orders): sorted by ascending price (lowest ask first)
    asks: PriceLevelMap,
    /// Bid side (buy orders): sorted by descending price (highest bid first)
    bids: PriceLevelMap,
}

impl OrderBook {
    /// Creates a new empty order book.
    ///
    /// ## Examples
    ///
    /// ```
    /// use order_book::OrderBook;
    ///
    /// let order_book = OrderBook::new();
    /// ```
    pub fn new() -> Self {
        OrderBook {
            asks: BTreeMap::new(),
            bids: BTreeMap::new(),
        }
    }

    /// Aggregates a precise price to its integer price level.
    ///
    /// This function truncates the decimal portion of the price, effectively
    /// grouping all orders with prices like 100.01, 100.25, 100.99 into the
    /// same aggregated level of 100.
    ///
    /// ## Arguments
    ///
    /// * `price`: The exact order price to aggregate
    ///
    /// ## Returns
    ///
    /// The truncated price level as a Decimal
    ///
    /// ## Examples
    ///
    /// ```
    /// use order_book::OrderBook;
    /// use rust_decimal::Decimal;
    ///
    /// let price = Decimal::new(10025, 2); // 100.25
    /// let aggregated = OrderBook::aggregate_price_to_level(price);
    /// assert_eq!(aggregated, Decimal::new(100, 0)); // 100
    /// ```
    pub fn aggregate_price_to_level(price: Decimal) -> Decimal {
        price.trunc()
    }

    /// Inserts a new order into the order book and returns an event.
    ///
    /// This method:
    /// 1. Adds the order to the appropriate price level (maintaining time priority)
    /// 2. Returns an `OrderEvent` that downstream services can use to update their state
    ///
    /// The write lock should be held only during this operation, which is $O(\log{N})$
    /// where $N$ is the number of distinct price levels.
    ///
    /// ## Arguments
    ///
    /// * `order`: The order to insert
    ///
    /// ## Returns
    ///
    /// An `OrderEvent` describing the change that occurred
    ///
    /// ## Examples
    ///
    /// ```
    /// use order_book::{OrderBook, Order, Side};
    /// use rust_decimal::Decimal;
    ///
    /// let mut order_book = OrderBook::new();
    /// let order = Order::new(100.50, 100, Side::Bid);
    ///
    /// let event = order_book.insert_order(order);
    /// assert_eq!(event.quantity_delta, 100);
    /// ```
    pub fn insert_order(&mut self, order: Order) -> OrderEvent {
        let order_price = order.price;
        let order_quantity = order.quantity;
        let order_side = order.side;

        // Select the appropriate price level map based on side
        let price_level_map = match order_side {
            Side::Bid => &mut self.bids,
            Side::Ask => &mut self.asks,
        };

        // Insert the order at its price level, maintaining time priority
        price_level_map
            .entry(order_price)
            .or_insert_with(Vec::new)
            .push(order);

        // Publish the event for downstream consumers
        OrderEvent {
            price: order_price,
            quantity_delta: order_quantity,
            side: order_side,
        }
    }

    /// Computes the current best bid and best ask prices.
    ///
    /// This operation acquires a read lock and is O(1) due to the BTreeMap structure:
    /// 
    /// - Best bid is the highest price in the bid map (last key)
    /// - Best ask is the lowest price in the ask map (first key)
    ///
    /// ## Returns
    ///
    /// A tuple of `(best_bid, best_ask)` where each is `Option<Decimal>`.
    /// Returns `None` if there are no orders on that side.
    ///
    /// ## Examples
    ///
    /// ```
    /// use order_book::{OrderBook, Order, Side};
    /// use rust_decimal::Decimal;
    ///
    /// let mut order_book = OrderBook::new();
    /// order_book.insert_order(Order::new(100.50, 100, Side::Bid));
    ///
    /// let (best_bid, best_ask) = order_book.compute_spread();
    /// assert_eq!(best_bid, Some(Decimal::new(10050, 2)));
    /// assert_eq!(best_ask, None);
    /// ```
    pub fn compute_spread(&self) -> (Option<Decimal>, Option<Decimal>) {
        // BTreeMap maintains sorted order:
        // - For bids: higher prices come last (use next_back to get highest)
        // - For asks: lower prices come first (use next to get lowest)
        let best_bid = self.bids.keys().next_back().copied();
        let best_ask = self.asks.keys().next().copied();

        (best_bid, best_ask)
    }

    /// Returns the number of distinct price levels on the bid side.
    ///
    /// ## Returns
    ///
    /// The count of unique bid price levels
    pub fn bid_levels_count(&self) -> usize {
        self.bids.len()
    }

    /// Returns the number of distinct price levels on the ask side.
    ///
    /// ## Returns
    ///
    /// The count of unique ask price levels
    pub fn ask_levels_count(&self) -> usize {
        self.asks.len()
    }

    /// Returns the total number of orders at a specific price level.
    ///
    /// ## Arguments
    ///
    /// * `price`: The price level to query
    /// * `side`: The side (bid or ask) to query
    ///
    /// ## Returns
    ///
    /// The number of orders at that price level, or 0 if no orders exist
    pub fn orders_at_price_level(&self, price: Decimal, side: Side) -> usize {
        let price_level_map = match side {
            Side::Bid => &self.bids,
            Side::Ask => &self.asks,
        };

        price_level_map
            .get(&price)
            .map(|orders| orders.len())
            .unwrap_or(0)
    }
}

impl Default for OrderBook {
    fn default() -> Self {
        Self::new()
    }
}
