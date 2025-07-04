# DJED Stablecoin & DEX Implementation

## Overview

This implements a minimal DJED stablecoin system with a decentralized exchange (DEX) using the relay network as the computation layer.

## DJED Stablecoin

Based on the "Minimal DJED" algorithm from the paper:

### Core Concepts

1. **Three Token System**:
   - **Base Coin**: Relay fuel credits (volatile)
   - **Stablecoin**: DJED (pegged to 1 game credit)
   - **Reserve Coin**: RC (for stability providers)

2. **Reserve Ratio Mechanism**:
   - Minimum ratio: 150% (protects against drops)
   - Maximum ratio: 400% (prevents over-collateralization)
   - Current ratio = Reserves / (DJED Supply × Oracle Price)

3. **Price Stability**:
   ```rust
   // When minting DJED
   if new_ratio > max_reserve_ratio {
       return Err("Would exceed maximum reserve ratio");
   }
   
   // When burning DJED
   if new_ratio < min_reserve_ratio {
       return Err("Would fall below minimum reserve ratio");
   }
   ```

4. **Reserve Coin Pricing**:
   - RC price = Equity / RC Supply
   - Equity = Reserves - (DJED Supply × Oracle Price)
   - Reserve coin holders absorb volatility

### Key Features

- **No Strong Peg Requirement**: Can deviate within bounds
- **Fee Collection**: 1% on all operations
- **Equity Protection**: RC holders can't withdraw below DJED liabilities
- **Oracle Price Updates**: Flexible price feed mechanism

## Decentralized Exchange (DEX)

Uses relay network for distributed order matching and settlement:

### Relay Roles

1. **Order Book Keeper**: Maintains order books for specific pairs
2. **Matcher**: Matches orders and creates executions
3. **Settler**: Validates and settles trades
4. **Archiver**: Stores historical data

### Order Flow

```
User → Submit Order → OrderBook Relay
                           ↓
                    Matcher Relay
                           ↓
                    Pending Execution
                           ↓
                     Settler Relay
                           ↓
                      Settlement
```

### Order Book Structure

- **Price Precision**: 8 decimal places (price_e8)
- **FIFO at Same Price**: Orders sorted by timestamp
- **Midpoint Execution**: Trades execute at (bid + ask) / 2

### Market Depth Aggregation

```rust
// Aggregate orders by price level
let mut bid_levels: BTreeMap<u64, f64> = BTreeMap::new();
for (key, order) in book.bids.iter() {
    *bid_levels.entry(key.price_e8).or_insert(0.0) += order.remaining;
}
```

## Trading Pairs

Primary pairs for the P2P Go ecosystem:

1. **DJED/FUEL**: Stablecoin vs relay credits
2. **RC/FUEL**: Reserve coin vs relay credits
3. **DJED/RC**: Direct stablecoin/reserve trading

## Integration with Guilds

Different guilds interact with the DEX differently:

- **Activity Guild**: Market makers, high-frequency trading
- **Reactivity Guild**: Arbitrage, reactive strategies
- **Avoidance Guild**: Long-term positions, stability provision

## Future Enhancements

1. **Atomic Swaps**: Cross-relay trading without intermediaries
2. **Liquidity Pools**: AMM-style pools alongside order book
3. **Governance Token**: For DEX parameter updates
4. **Cross-Chain Bridge**: Connect to other networks

## Example Usage

```rust
// Create DJED system
let mut djed = MinimalDjed::new(10.0); // 10 fuel per game credit

// Provide stability (buy reserve coins)
djed.buy_reserve_coins("alice", 1000)?;

// Mint stablecoins
let djed_amount = djed.buy_stablecoins("bob", 100)?;

// Trade on DEX
let order = Order {
    trader: "bob".to_string(),
    side: OrderSide::Sell,
    price: 10.5, // DJED/FUEL rate
    amount: djed_amount as f64,
    // ...
};

dex.submit_order(order, pair).await?;
```

## Security Considerations

1. **Oracle Manipulation**: Multiple price sources needed
2. **Front-Running**: Relay order fairness protocols
3. **Sybil Attacks**: Relay reputation system
4. **Reserve Draining**: Protected by min ratio requirements

The system provides a foundation for decentralized finance within the P2P Go network, allowing players to trade resources and maintain stable value storage.