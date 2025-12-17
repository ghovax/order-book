# Price-Priority Order Book

The purpose of this library is to implement a price-priority order book, and the design choices reflect priorities of modularity and scalability, aiming to avoid future rewrites by relying on a solid data structure architecture.

The order book is a ledger in which orders are recorded. Every order must include a price, a positive integer quantity, and a side indicating buy (bid) or sell (ask). For a buy order, the price is the maximum amount the buyer is willing to pay per unit; for a sell order, the price is the minimum amount the seller is willing to accept per unit. From the information on the ledger, we can calculate two quantities: the market spread and market depth. The spread is the difference between the best bid and the best ask, where the best bid is the maximum price any buyer is willing to pay and the best ask is the minimum price any seller is willing to accept; it is a global quantity derived from the orders present in the ledger. Market depth is a more granular quantity that represents how much it would cost to execute a larger trade—for example, to buy 150 units available in the order book, we need to sum the quantities available at successive prices until we reach 150 units. Because individual prices can be very precise, we aggregate them by defining price levels: each order is associated with the integer part of its price, so orders at 100.50 and 100.99 are grouped into price level 100; the aggregated market depth thus represents quantities grouped by the truncated price.

Thus, we have two sides of an order book: one focused on precise data for managing exact orders, and another that provides aggregated data for market analysis. Because we apply programming principles such as proper division of roles, it is important to recognize the separation between these different components of the problem, which in turn needs to be reflected in the code architecture.

I would say the core element of an order book is an individual order, which has a price, a quantity, and a side. This is reflected in `types.rs`, where an enum is used for the `Side` to maintain type safety, and an `Order` is defined as an individual struct with all members public so its implementation is transparent. It is intended to be treated as a single element we manipulate that aggregates essential data; for this, I defined a `new` function as an accessibility tool for creating `Order`s from floating point numbers. The `price` in the order struct is represented not by a binary floating point but by a fixed-point decimal (`Decimal`) to ensure accurate representation of quantities and to avoid rounding errors during calculations, which I believe is the standard approach in market and financial code.

In the `order_book.rs` file, I defined and implemented the `OrderBook` class. It contains two `ExactPriceLevelMap`s, one for bids and one for asks. Each exact-price-level map is a binary tree map that uses fixed-point decimals as keys for prices and associates each price with a vector of orders, so multiple orders with the same exact price are represented. The usage of this data structure is smart because, upon insertion of an entry in the map, it is automatically sorted by key, which means the exact prices for each order (the key) are maintained in sorted order for both bids and asks. 

The `OrderBook` class provides methods for being initialized (`new`), aggregating prices to the nearest level (`aggregate_price_to_level`), adding an order to the ledger (`insert_order`), as well as for retrieving the best bid and ask prices and computing the spread simultaneously (`compute_spread`) and the orders at a given exact-price-level (`orders_at_exact_price_level`).

One question that naturally arises is: where we calculate market depth? To address this, we use an external cache that tracks aggregated market depth; this structure is decoupled from the order book and maintains its own state. When an order is published by the order book (basically once an order is created) it is inserted into the cache for processing. Although the cache could become temporarily inconsistent with the order book, preventing such invalid states is the user's responsibility; the architecture itself is fully compatible with the order book. The publisher-subscriber design is intentional for several reasons: it maximizes concurrency because readers can query market depth from the cache without blocking order insertions, which require high throughput; and order insertion does not need to wait for depth aggregation, since aggregation can be performed asynchronously from the core order book rather than as a blocking operation.

Technically speaking, the `MarketDepthCache` is a subscriber (observer) that receives `OrderEvent`s from the `OrderBook` (publisher) and updates its state accordingly. Additionally, the `MarketDepthCache` is designed to be thread-safe, allowing concurrent reads and serialized writes (see `parking_lot::RwLock` implementation of fairness for more details) to the aggregated bid and ask depth maps. 

The `MarketDepthCache` works in a similar way to the `OrderBook`, but specifically tracking aggregated market depth individually for each side (bid or ask) by storing the quantity at each price level via two separate `AggregatedDepthMap` (`BTreeMap<Decimal, u64>`, where the quantity is a `u64`) instances. 

It works in the following way: first, a new order is added to the order book, then the market depth cache, which can be extended to compute other data if needed, registers the order event. For the market depth calculation, the aggregated price level for the order is computed, and the lock for the associated aggregated market depth map (`AggregatedDepthMap`) is acquired. Finally, the given quantity is inserted at the aggregated price level and stored for later retrieval. This operation is opaque to the library's end user, who is only concerned with registering the event to the subscriber, meaning the market depth cache.

If the user wants to retrieve the aggregated market depth, they will obtain a snapshot of the bids and asks individually, which they can then query directly via the `get_aggregated_market_depth` method. As a utility method, the user can also call `get_quantity_at_level` directly, which simplifies this operation.

```rust
use order_book::{OrderBook, MarketDepthCache, Order, Side};
use rust_decimal::Decimal;

let mut order_book = OrderBook::new();
let cache = MarketDepthCache::new();

let order = Order::new(100.50, 100, Side::Bid);

let event = order_book.insert_order(order);
cache.process_order_event(event);

let quantity = cache.get_quantity_at_level(Decimal::new(100, 0), Side::Bid);
assert_eq!(quantity, 100);
```

Other methods are also present, but they are utilities, such as retrieving the count of bid and ask levels individually (`bid_levels_count` and `ask_levels_count`) or clearing the cache (`clear`).

Of course, no library is complete without a unit-test suite that thoroughly tests the implementations, which I have placed in the `tests/integration_tests.rs` file. I have also implemented benchmarks in the `benches/order_book_benchmarks.rs` file so that any future changes to the implementation can be tested to detect performance regressions.

Lastly, I would like to add final considerations on the thread safety of the classes I implemented. The `OrderBook` is `Send` but not `Sync`: it can be transferred between threads, but it is not safe for concurrent access because the binary tree map likely does not implement internal synchronization, so multiple threads could modify it simultaneously. Therefore, the `OrderBook` class in this library is intended to be used behind a read-write lock (`RwLock`). To access it from multiple threads, as shown in the test files, create an `Arc` that wraps the `RwLock`; the lock regulates reading and writing to the order book, while the atomic reference count provides shared ownership.

The market depth cache uses an internal lock for each of the two aggregated market depth, one for bids and one for asks. To allow access from multiple threads and make it `Send` and `Sync`, the cache must use an `Arc`. We are using external locking for the order book because it is simple to implement and flexible: if needed later, we can wrap it in an `Arc` plus a `RwLock` to make it `Send` and `Sync`. The market depth cache can retain internal locking since its implementation will be opaque, and it only requires independent bid and ask access. All in all, both choices are possible for both systems, but this design decision makes the architecture more flexible for the future and clarifies the distinct responsibilities of each component.
