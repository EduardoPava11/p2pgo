// SPDX-License-Identifier: MIT OR Apache-2.0

//! Decentralized Exchange (DEX) using Relay Network
//!
//! Uses the relay mesh network as computation/execution layer
//! for a decentralized order book.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, BTreeMap};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Decentralized order book using relays for execution
pub struct RelayDEX {
    /// Order books for different pairs
    pub books: Arc<RwLock<HashMap<TradingPair, OrderBook>>>,

    /// Relay assignments for order matching
    pub relay_assignments: Arc<RwLock<HashMap<String, RelayRole>>>,

    /// Execution queue
    pub execution_queue: Arc<RwLock<Vec<PendingExecution>>>,

    /// Settlement layer
    pub settlements: Arc<RwLock<Vec<Settlement>>>,
}

#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct TradingPair {
    pub base: String,  // e.g., "DJED"
    pub quote: String, // e.g., "FUEL" (relay credits)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderBook {
    /// Buy orders sorted by price (highest first)
    pub bids: BTreeMap<OrderKey, Order>,

    /// Sell orders sorted by price (lowest first)
    pub asks: BTreeMap<OrderKey, Order>,

    /// Last trade price
    pub last_price: Option<f64>,

    /// 24h volume
    pub volume_24h: f64,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct OrderKey {
    /// Price in quote currency (multiplied by 1e8 for precision)
    pub price_e8: u64,

    /// Timestamp for FIFO ordering at same price
    pub timestamp: u64,

    /// Unique order ID
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Order {
    pub id: String,
    pub trader: String,
    pub side: OrderSide,
    pub price: f64,
    pub amount: f64,
    pub remaining: f64,
    pub timestamp: u64,
    pub relay_id: String, // Relay that submitted this order
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum OrderSide {
    Buy,
    Sell,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RelayRole {
    /// Maintains order book for specific pairs
    OrderBookKeeper { pairs: Vec<TradingPair> },

    /// Matches orders and creates executions
    Matcher { pairs: Vec<TradingPair> },

    /// Validates and settles trades
    Settler,

    /// Archives historical data
    Archiver,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingExecution {
    pub id: String,
    pub buy_order: String,
    pub sell_order: String,
    pub price: f64,
    pub amount: f64,
    pub matcher_relay: String,
    pub status: ExecutionStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExecutionStatus {
    Pending,
    Validating,
    Settling,
    Completed,
    Failed(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settlement {
    pub execution_id: String,
    pub buyer: String,
    pub seller: String,
    pub pair: TradingPair,
    pub price: f64,
    pub amount: f64,
    pub buyer_fee: f64,
    pub seller_fee: f64,
    pub settler_relay: String,
    pub timestamp: u64,
    pub block_height: u64, // For finality
}

impl RelayDEX {
    pub fn new() -> Self {
        Self {
            books: Arc::new(RwLock::new(HashMap::new())),
            relay_assignments: Arc::new(RwLock::new(HashMap::new())),
            execution_queue: Arc::new(RwLock::new(Vec::new())),
            settlements: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Submit a new order through a relay
    pub async fn submit_order(
        &self,
        order: Order,
        pair: TradingPair,
    ) -> Result<String, &'static str> {
        let mut books = self.books.write().await;
        let book = books.entry(pair).or_insert_with(OrderBook::new);

        let key = OrderKey {
            price_e8: (order.price * 1e8) as u64,
            timestamp: order.timestamp,
            id: order.id.clone(),
        };

        match order.side {
            OrderSide::Buy => {
                book.bids.insert(key, order.clone());
            }
            OrderSide::Sell => {
                book.asks.insert(key, order.clone());
            }
        }

        Ok(order.id)
    }

    /// Match orders for a trading pair (called by matcher relays)
    pub async fn match_orders(&self, pair: &TradingPair) -> Vec<PendingExecution> {
        let mut books = self.books.write().await;
        let book = match books.get_mut(pair) {
            Some(b) => b,
            None => return Vec::new(),
        };

        let mut executions = Vec::new();

        // Get best bid and ask
        loop {
            // Clone keys to avoid borrow issues
            let bid_entry = book.bids.iter().next().map(|(k, v)| (k.clone(), v.clone()));
            let ask_entry = book.asks.iter().next().map(|(k, v)| (k.clone(), v.clone()));

            if let (Some((bid_key, bid)), Some((ask_key, ask))) = (bid_entry, ask_entry) {
                // Check if orders cross
                if bid.price >= ask.price {
                    let trade_amount = bid.remaining.min(ask.remaining);
                    let trade_price = (bid.price + ask.price) / 2.0; // Midpoint

                    let execution = PendingExecution {
                        id: format!("{}-{}-{}", bid.id, ask.id, bid.timestamp),
                        buy_order: bid.id.clone(),
                        sell_order: ask.id.clone(),
                        price: trade_price,
                        amount: trade_amount,
                        matcher_relay: "current_relay".to_string(), // Would be actual relay ID
                        status: ExecutionStatus::Pending,
                    };

                    executions.push(execution);

                    // Update order amounts
                    let mut updated_bid = bid.clone();
                    let mut updated_ask = ask.clone();
                    updated_bid.remaining -= trade_amount;
                    updated_ask.remaining -= trade_amount;

                    // Remove filled orders
                    if updated_bid.remaining == 0.0 {
                        book.bids.remove(&bid_key);
                    } else {
                        book.bids.insert(bid_key, updated_bid);
                    }

                    if updated_ask.remaining == 0.0 {
                        book.asks.remove(&ask_key);
                    } else {
                        book.asks.insert(ask_key, updated_ask);
                    }

                    // Update last price
                    book.last_price = Some(trade_price);
                    book.volume_24h += trade_amount * trade_price;
                } else {
                    break; // No more crossing orders
                }
            } else {
                break; // No orders to match
            }
        }

        // Add to execution queue
        if !executions.is_empty() {
            let mut queue = self.execution_queue.write().await;
            queue.extend(executions.clone());
        }

        executions
    }

    /// Validate and settle executions (called by settler relays)
    pub async fn settle_execution(
        &self,
        execution_id: &str,
        settler_relay: &str,
    ) -> Result<Settlement, &'static str> {
        let mut queue = self.execution_queue.write().await;

        // Find execution
        let exec_pos = queue.iter().position(|e| e.id == execution_id)
            .ok_or("Execution not found")?;

        let mut execution = queue[exec_pos].clone();

        // Validate execution
        execution.status = ExecutionStatus::Validating;

        // In production, would verify:
        // - Both traders have sufficient balances
        // - Orders haven't been cancelled
        // - Price is fair market value

        // Create settlement
        let settlement = Settlement {
            execution_id: execution_id.to_string(),
            buyer: execution.buy_order.clone(),
            seller: execution.sell_order.clone(),
            pair: TradingPair {
                base: "DJED".to_string(),
                quote: "FUEL".to_string()
            },
            price: execution.price,
            amount: execution.amount,
            buyer_fee: execution.amount * 0.001, // 0.1% fee
            seller_fee: execution.amount * 0.001,
            settler_relay: settler_relay.to_string(),
            timestamp: 0, // Would use actual
            block_height: 0, // Would use actual
        };

        // Update status
        execution.status = ExecutionStatus::Completed;
        queue[exec_pos] = execution;

        // Store settlement
        let mut settlements = self.settlements.write().await;
        settlements.push(settlement.clone());

        Ok(settlement)
    }

    /// Assign roles to relays based on their capabilities
    pub async fn assign_relay_role(
        &self,
        relay_id: String,
        role: RelayRole,
    ) {
        let mut assignments = self.relay_assignments.write().await;
        assignments.insert(relay_id, role);
    }

    /// Get market depth for a pair
    pub async fn get_market_depth(
        &self,
        pair: &TradingPair,
        levels: usize,
    ) -> (Vec<(f64, f64)>, Vec<(f64, f64)>) {
        let books = self.books.read().await;
        let book = match books.get(pair) {
            Some(b) => b,
            None => return (Vec::new(), Vec::new()),
        };

        let mut bids = Vec::new();
        let mut asks = Vec::new();

        // Aggregate bids by price level
        let mut bid_levels: BTreeMap<u64, f64> = BTreeMap::new();
        for (key, order) in book.bids.iter().take(levels * 10) {
            *bid_levels.entry(key.price_e8).or_insert(0.0) += order.remaining;
        }

        for (price_e8, amount) in bid_levels.iter().rev().take(levels) {
            bids.push((*price_e8 as f64 / 1e8, *amount));
        }

        // Aggregate asks by price level
        let mut ask_levels: BTreeMap<u64, f64> = BTreeMap::new();
        for (key, order) in book.asks.iter().take(levels * 10) {
            *ask_levels.entry(key.price_e8).or_insert(0.0) += order.remaining;
        }

        for (price_e8, amount) in ask_levels.iter().take(levels) {
            asks.push((*price_e8 as f64 / 1e8, *amount));
        }

        (bids, asks)
    }
}

impl OrderBook {
    pub fn new() -> Self {
        Self {
            bids: BTreeMap::new(),
            asks: BTreeMap::new(),
            last_price: None,
            volume_24h: 0.0,
        }
    }
}

/// Message types for relay-to-relay DEX communication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DexMessage {
    /// New order submission
    SubmitOrder {
        order: Order,
        pair: TradingPair,
    },

    /// Order cancellation
    CancelOrder {
        order_id: String,
        trader: String,
    },

    /// Execution broadcast
    ExecutionBroadcast {
        execution: PendingExecution,
    },

    /// Settlement confirmation
    SettlementConfirm {
        settlement: Settlement,
    },

    /// Market data request
    MarketDataRequest {
        pair: TradingPair,
        depth_levels: usize,
    },

    /// Market data response
    MarketDataResponse {
        pair: TradingPair,
        bids: Vec<(f64, f64)>,
        asks: Vec<(f64, f64)>,
        last_price: Option<f64>,
        volume_24h: f64,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_order_matching() {
        let dex = RelayDEX::new();
        let pair = TradingPair {
            base: "DJED".to_string(),
            quote: "FUEL".to_string(),
        };

        // Submit buy order
        let buy_order = Order {
            id: "buy1".to_string(),
            trader: "alice".to_string(),
            side: OrderSide::Buy,
            price: 10.5,
            amount: 100.0,
            remaining: 100.0,
            timestamp: 1000,
            relay_id: "relay1".to_string(),
        };

        dex.submit_order(buy_order, pair.clone()).await.unwrap();

        // Submit sell order that crosses
        let sell_order = Order {
            id: "sell1".to_string(),
            trader: "bob".to_string(),
            side: OrderSide::Sell,
            price: 10.0,
            amount: 50.0,
            remaining: 50.0,
            timestamp: 1001,
            relay_id: "relay2".to_string(),
        };

        dex.submit_order(sell_order, pair.clone()).await.unwrap();

        // Match orders
        let executions = dex.match_orders(&pair).await;
        assert_eq!(executions.len(), 1);
        assert_eq!(executions[0].amount, 50.0);
        assert_eq!(executions[0].price, 10.25); // Midpoint
    }

    #[tokio::test]
    async fn test_market_depth() {
        let dex = RelayDEX::new();
        let pair = TradingPair {
            base: "DJED".to_string(),
            quote: "FUEL".to_string(),
        };

        // Add multiple orders at different price levels
        for i in 0..5 {
            let buy_order = Order {
                id: format!("buy{}", i),
                trader: "alice".to_string(),
                side: OrderSide::Buy,
                price: 10.0 - i as f64 * 0.1,
                amount: 100.0,
                remaining: 100.0,
                timestamp: 1000 + i,
                relay_id: "relay1".to_string(),
            };

            dex.submit_order(buy_order, pair.clone()).await.unwrap();
        }

        let (bids, asks) = dex.get_market_depth(&pair, 3).await;
        assert_eq!(bids.len(), 3);
        assert!(bids[0].0 > bids[1].0); // Descending price order
    }
}