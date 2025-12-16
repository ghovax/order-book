use rust_decimal::Decimal;
use std::collections::BTreeMap;

/// Represents the side of an order in the order book.
///
/// - `Bid` represents buy orders (demand side)
/// - `Ask` represents sell orders (supply side)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Side {
    /// Buy side: traders willing to purchase at a given price
    Bid,
    /// Sell side: traders willing to sell at a given price
    Ask,
}

/// Represents a single order in the order book.
///
/// Each order contains a price, quantity, and side (bid or ask).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Order {
    /// The price level at which this order is placed (using fixed-point arithmetic)
    pub price: Decimal,
    /// The quantity of the asset being bought or sold
    pub quantity: u64,
    /// Whether this is a buy (`Bid`) or sell (`Ask`) order
    pub side: Side,
}

impl Order {
    /// Creates a new order with the given price, quantity, and side.
    pub fn new(price: f64, quantity: u64, side: Side) -> Self {
        Self {
            price: Decimal::try_from(price).unwrap(),
            quantity,
            side,
        }
    }
}

/// Represents an event published by the `OrderBook` when its state changes.
///
/// This event is consumed by downstream services (like `MarketDepthCache`) to update
/// their own state without blocking the core order book operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrderEvent {
    /// The exact price level where the change occurred
    pub price: Decimal,
    /// The change in quantity at this price level (positive for additions)
    pub quantity_delta: u64,
    /// Whether this event affects the bid or ask side
    pub side: Side,
}

/// Type alias for a price level in the order book.
///
/// Maps each price (`Decimal`) to a list of orders at that price.
/// Orders within a price level maintain time priority (FIFO).
pub type PriceLevelMap = BTreeMap<Decimal, Vec<Order>>;

/// Type alias for aggregated market depth cache.
///
/// Maps each aggregated price level (Decimal) to the total quantity (`u64`)
/// available at that level across all individual orders.
pub type AggregatedDepthMap = BTreeMap<Decimal, u64>;
