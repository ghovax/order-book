# Price-Priority Order Book Implementation Challenge

## 🎯 Objective

Implement a **price-priority limit order book** in **Rust**. This order book will manage limit orders and provide views of market data.

## 🛠️ Implementation Requirements

### 1. Data Structure: Order Book

The primary focus is the efficient data structure for the order book.

- **Order Definition:** An order must be defined by its **price**, **quantity**, and **side** (Buy/Bid or Sell/Ask).
- **Price Resolution:** The market price resolution is **0.01**.
- **Out of Scope:** Connectivity to external systems, user management, and timestamps are not required. Design the API with extensibility in mind.

### 2. Required Functionality (API)

Your implementation must support the following core behaviors:

- **Order Insertion:** Support append-only insertion of new limit orders.
- **Spread Computation:** Calculate and return the **best bid** (highest buy price) and the **best ask** (lowest sell price).
- **Market Depth View:** Return the market depth data aggregated by price level.

### 3. Market Depth Aggregation Logic

The market depth view must aggregate orders according to the following rule:

- **Aggregation Resolution:** Aggregate all orders within a resolution of **1**.
  - _Example:_ Orders at prices 100.01, 100.50, and 100.99 must all be aggregated into the price level **100**.

### 4. Test Suite

Include a comprehensive test suite to showcase and validate the expected behaviors:

- Test successful **order insertion**.
- Test the correct **spread computation**.
- Test the correct **market depth aggregation** based on the required logic.

### 5. Documentation (README.md)

Provide a `README.md` file that includes:

- An explanation of your **technical design choices**.
- A discussion of **potential improvements** for a real-world, high-performance scenario (e.g., handling high-throughput, latency, or concurrency).
