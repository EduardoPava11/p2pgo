// SPDX-License-Identifier: MIT OR Apache-2.0

//! Minimal DJED Stablecoin Implementation
//! 
//! Based on the DJED paper's "Minimal DJED" algorithm.
//! We use relay credits as the base coin and maintain a peg to game credits.
//! 
//! Key concepts:
//! - Base coin: Relay fuel credits (volatile)
//! - Stablecoin: DJED (pegged to 1 game credit)
//! - Reserve coin: RC (Reserve Coins for stability providers)

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// The minimal DJED implementation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MinimalDjed {
    /// Total reserves in base coin (relay credits)
    pub reserves: u64,
    
    /// Total stablecoins in circulation
    pub stablecoin_supply: u64,
    
    /// Total reserve coins in circulation  
    pub reserve_coin_supply: u64,
    
    /// Target price (1 DJED = 1 game credit worth of relay fuel)
    pub peg_target: f64,
    
    /// Current oracle price (relay credits per game credit)
    pub oracle_price: f64,
    
    /// Minimum reserve ratio (e.g., 150%)
    pub min_reserve_ratio: f64,
    
    /// Maximum reserve ratio (e.g., 400%)
    pub max_reserve_ratio: f64,
    
    /// Fee percentage (e.g., 1%)
    pub fee_rate: f64,
    
    /// User balances
    pub balances: HashMap<String, UserBalances>,
    
    /// Fee collector
    pub collected_fees: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UserBalances {
    /// DJED stablecoin balance
    pub djed: u64,
    /// Reserve coin balance
    pub reserve_coins: u64,
    /// Base coin balance (relay credits)
    pub base_coins: u64,
}

impl MinimalDjed {
    pub fn new(initial_oracle_price: f64) -> Self {
        Self {
            reserves: 0,
            stablecoin_supply: 0,
            reserve_coin_supply: 0,
            peg_target: 1.0,
            oracle_price: initial_oracle_price,
            min_reserve_ratio: 1.5, // 150%
            max_reserve_ratio: 4.0, // 400%
            fee_rate: 0.01, // 1%
            balances: HashMap::new(),
            collected_fees: 0,
        }
    }
    
    /// Get current reserve ratio
    pub fn reserve_ratio(&self) -> f64 {
        if self.stablecoin_supply == 0 {
            return f64::INFINITY;
        }
        
        let stablecoin_liabilities = self.stablecoin_supply as f64 * self.oracle_price;
        self.reserves as f64 / stablecoin_liabilities
    }
    
    /// Calculate equity (value belonging to reserve coin holders)
    pub fn equity(&self) -> f64 {
        let stablecoin_liabilities = self.stablecoin_supply as f64 * self.oracle_price;
        (self.reserves as f64 - stablecoin_liabilities).max(0.0)
    }
    
    /// Price of one reserve coin in base coins
    pub fn reserve_coin_price(&self) -> f64 {
        if self.reserve_coin_supply == 0 {
            // Initial price when no reserve coins exist
            return self.oracle_price;
        }
        
        let equity = self.equity();
        equity / self.reserve_coin_supply as f64
    }
    
    /// Buy stablecoins by depositing base coins
    pub fn buy_stablecoins(
        &mut self, 
        user: &str, 
        base_amount: u64
    ) -> Result<u64, &'static str> {
        // Check if we're below max reserve ratio after minting
        let djed_to_mint = (base_amount as f64 / self.oracle_price) as u64;
        let new_stablecoin_supply = self.stablecoin_supply + djed_to_mint;
        let new_reserves = self.reserves + base_amount;
        let new_ratio = new_reserves as f64 / (new_stablecoin_supply as f64 * self.oracle_price);
        
        if new_ratio > self.max_reserve_ratio {
            return Err("Would exceed maximum reserve ratio");
        }
        
        // Apply fee
        let fee = (base_amount as f64 * self.fee_rate) as u64;
        let net_base = base_amount - fee;
        let djed_received = (net_base as f64 / self.oracle_price) as u64;
        
        // Update state
        self.reserves += net_base;
        self.stablecoin_supply += djed_received;
        self.collected_fees += fee;
        
        // Update user balance
        let balance = self.balances.entry(user.to_string()).or_default();
        balance.djed += djed_received;
        
        Ok(djed_received)
    }
    
    /// Sell stablecoins to receive base coins
    pub fn sell_stablecoins(
        &mut self,
        user: &str,
        djed_amount: u64
    ) -> Result<u64, &'static str> {
        let balance = self.balances.get_mut(user)
            .ok_or("User not found")?;
            
        if balance.djed < djed_amount {
            return Err("Insufficient DJED balance");
        }
        
        // Check if we're above min reserve ratio after burning
        let base_to_pay = (djed_amount as f64 * self.oracle_price) as u64;
        let new_reserves = self.reserves.saturating_sub(base_to_pay);
        let new_stablecoin_supply = self.stablecoin_supply - djed_amount;
        
        if new_stablecoin_supply > 0 {
            let new_ratio = new_reserves as f64 / (new_stablecoin_supply as f64 * self.oracle_price);
            if new_ratio < self.min_reserve_ratio {
                return Err("Would fall below minimum reserve ratio");
            }
        }
        
        // Apply fee
        let fee = (base_to_pay as f64 * self.fee_rate) as u64;
        let net_base = base_to_pay - fee;
        
        // Update state
        self.reserves -= base_to_pay;
        self.stablecoin_supply -= djed_amount;
        self.collected_fees += fee;
        
        // Update user balance
        balance.djed -= djed_amount;
        balance.base_coins += net_base;
        
        Ok(net_base)
    }
    
    /// Buy reserve coins to provide stability
    pub fn buy_reserve_coins(
        &mut self,
        user: &str,
        base_amount: u64
    ) -> Result<u64, &'static str> {
        let rc_price = self.reserve_coin_price();
        let _rc_to_mint = (base_amount as f64 / rc_price) as u64;
        
        // Apply fee
        let fee = (base_amount as f64 * self.fee_rate) as u64;
        let net_base = base_amount - fee;
        let rc_received = (net_base as f64 / rc_price) as u64;
        
        // Update state
        self.reserves += net_base;
        self.reserve_coin_supply += rc_received;
        self.collected_fees += fee;
        
        // Update user balance
        let balance = self.balances.entry(user.to_string()).or_default();
        balance.reserve_coins += rc_received;
        
        Ok(rc_received)
    }
    
    /// Sell reserve coins to receive base coins
    pub fn sell_reserve_coins(
        &mut self,
        user: &str,
        rc_amount: u64
    ) -> Result<u64, &'static str> {
        // Get price before mutable borrow
        let rc_price = self.reserve_coin_price();
        let base_to_pay = (rc_amount as f64 * rc_price) as u64;
        
        let balance = self.balances.get_mut(user)
            .ok_or("User not found")?;
            
        if balance.reserve_coins < rc_amount {
            return Err("Insufficient reserve coin balance");
        }
        
        // Ensure we don't deplete reserves below stablecoin liabilities
        let stablecoin_liabilities = (self.stablecoin_supply as f64 * self.oracle_price) as u64;
        if self.reserves < stablecoin_liabilities + base_to_pay {
            return Err("Insufficient reserves to pay out");
        }
        
        // Apply fee
        let fee = (base_to_pay as f64 * self.fee_rate) as u64;
        let net_base = base_to_pay - fee;
        
        // Update state
        self.reserves -= base_to_pay;
        self.reserve_coin_supply -= rc_amount;
        self.collected_fees += fee;
        
        // Update user balance
        balance.reserve_coins -= rc_amount;
        balance.base_coins += net_base;
        
        Ok(net_base)
    }
    
    /// Update oracle price (in production, this would come from multiple sources)
    pub fn update_oracle_price(&mut self, new_price: f64) {
        self.oracle_price = new_price;
    }
    
    /// Distribute collected fees to reserve coin holders
    pub fn distribute_fees(&mut self) {
        if self.reserve_coin_supply == 0 || self.collected_fees == 0 {
            return;
        }
        
        // Add fees to reserves (benefiting RC holders)
        self.reserves += self.collected_fees;
        self.collected_fees = 0;
    }
}

/// Order book for DJED/base coin trading
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DjedOrderBook {
    /// Buy orders (want to buy DJED with base coins)
    pub buy_orders: Vec<Order>,
    /// Sell orders (want to sell DJED for base coins)
    pub sell_orders: Vec<Order>,
    /// Completed trades
    pub trade_history: Vec<Trade>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Order {
    pub id: String,
    pub trader: String,
    pub price: f64, // Base coins per DJED
    pub amount: u64, // DJED amount
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trade {
    pub buyer: String,
    pub seller: String,
    pub price: f64,
    pub amount: u64,
    pub timestamp: u64,
}

impl DjedOrderBook {
    pub fn new() -> Self {
        Self {
            buy_orders: Vec::new(),
            sell_orders: Vec::new(),
            trade_history: Vec::new(),
        }
    }
    
    /// Place a buy order
    pub fn place_buy_order(&mut self, order: Order) {
        self.buy_orders.push(order);
        self.buy_orders.sort_by(|a, b| b.price.partial_cmp(&a.price).unwrap());
    }
    
    /// Place a sell order
    pub fn place_sell_order(&mut self, order: Order) {
        self.sell_orders.push(order);
        self.sell_orders.sort_by(|a, b| a.price.partial_cmp(&b.price).unwrap());
    }
    
    /// Match orders and execute trades
    pub fn match_orders(&mut self) -> Vec<Trade> {
        let mut trades = Vec::new();
        
        while !self.buy_orders.is_empty() && !self.sell_orders.is_empty() {
            let best_buy = &self.buy_orders[0];
            let best_sell = &self.sell_orders[0];
            
            // Check if orders match
            if best_buy.price >= best_sell.price {
                let trade_amount = best_buy.amount.min(best_sell.amount);
                let trade_price = (best_buy.price + best_sell.price) / 2.0;
                
                let trade = Trade {
                    buyer: best_buy.trader.clone(),
                    seller: best_sell.trader.clone(),
                    price: trade_price,
                    amount: trade_amount,
                    timestamp: 0, // Would use actual timestamp
                };
                
                trades.push(trade.clone());
                self.trade_history.push(trade);
                
                // Update order amounts
                if best_buy.amount == trade_amount {
                    self.buy_orders.remove(0);
                } else {
                    self.buy_orders[0].amount -= trade_amount;
                }
                
                if best_sell.amount == trade_amount {
                    self.sell_orders.remove(0);
                } else {
                    self.sell_orders[0].amount -= trade_amount;
                }
            } else {
                break;
            }
        }
        
        trades
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_minimal_djed_minting() {
        let mut djed = MinimalDjed::new(10.0); // 10 relay credits per game credit
        
        // Buy some reserve coins first to provide backing
        let rc_bought = djed.buy_reserve_coins("alice", 1000).unwrap();
        assert!(rc_bought > 0);
        assert_eq!(djed.reserves, 990); // 1000 - 1% fee
        
        // Buy stablecoins
        let djed_bought = djed.buy_stablecoins("bob", 100).unwrap();
        assert!(djed_bought > 0);
        assert!(djed.reserve_ratio() > djed.min_reserve_ratio);
    }
    
    #[test]
    fn test_reserve_ratio_limits() {
        let mut djed = MinimalDjed::new(10.0);
        
        // Need reserves first
        djed.buy_reserve_coins("alice", 10000).unwrap();
        
        // Buy maximum stablecoins
        let max_djed = (djed.reserves as f64 / djed.oracle_price / djed.max_reserve_ratio) as u64;
        let result = djed.buy_stablecoins("bob", (max_djed as f64 * djed.oracle_price * 1.5) as u64);
        assert!(result.is_err()); // Should fail due to max ratio
    }
    
    #[test]
    fn test_order_book() {
        let mut book = DjedOrderBook::new();
        
        book.place_buy_order(Order {
            id: "1".to_string(),
            trader: "alice".to_string(),
            price: 10.5,
            amount: 100,
            timestamp: 0,
        });
        
        book.place_sell_order(Order {
            id: "2".to_string(),
            trader: "bob".to_string(),
            price: 10.0,
            amount: 50,
            timestamp: 0,
        });
        
        let trades = book.match_orders();
        assert_eq!(trades.len(), 1);
        assert_eq!(trades[0].amount, 50);
        assert_eq!(trades[0].price, 10.25); // Midpoint
    }
}