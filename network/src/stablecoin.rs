// SPDX-License-Identifier: MIT OR Apache-2.0

//! DJED-style Stablecoin for P2P Go Marketplace
//!
//! A simple stablecoin implementation for network mobility payments
//! and ELO bracket advancement rewards.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// DJED stablecoin configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DjedConfig {
    /// Target price in relay credits
    pub target_price: f64,
    /// Minimum collateral ratio
    pub min_collateral_ratio: f64,
    /// Maximum collateral ratio
    pub max_collateral_ratio: f64,
    /// Fee percentage for minting/burning
    pub fee_percentage: f64,
}

impl Default for DjedConfig {
    fn default() -> Self {
        Self {
            target_price: 1.0, // 1 DJED = 1 relay credit
            min_collateral_ratio: 1.5, // 150% collateralized
            max_collateral_ratio: 4.0, // 400% max
            fee_percentage: 0.01, // 1% fee
        }
    }
}

/// Stablecoin system state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StablecoinSystem {
    /// Configuration
    pub config: DjedConfig,
    /// Total DJED supply
    pub total_supply: u64,
    /// Reserve of relay credits
    pub reserve_credits: u64,
    /// User balances
    pub balances: HashMap<String, UserBalance>,
    /// ELO-based rewards pool
    pub elo_rewards_pool: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UserBalance {
    /// DJED stablecoin balance
    pub djed: u64,
    /// Reserve coin balance (for stability providers)
    pub reserve_coins: u64,
    /// Locked for ELO advancement
    pub locked_for_elo: u64,
}

impl StablecoinSystem {
    pub fn new() -> Self {
        Self {
            config: DjedConfig::default(),
            total_supply: 0,
            reserve_credits: 0,
            balances: HashMap::new(),
            elo_rewards_pool: 1000, // Initial rewards
        }
    }

    /// Get current collateral ratio
    pub fn collateral_ratio(&self) -> f64 {
        if self.total_supply == 0 {
            return self.config.max_collateral_ratio;
        }
        self.reserve_credits as f64 / self.total_supply as f64
    }

    /// Mint DJED by depositing relay credits
    pub fn mint_djed(&mut self, user: String, credits: u64) -> Result<u64, &'static str> {
        let ratio = self.collateral_ratio();
        if ratio < self.config.min_collateral_ratio {
            return Err("System under-collateralized");
        }

        let fee = (credits as f64 * self.config.fee_percentage) as u64;
        let net_credits = credits - fee;
        let djed_amount = (net_credits as f64 * self.config.target_price) as u64;

        self.reserve_credits += net_credits;
        self.total_supply += djed_amount;
        self.elo_rewards_pool += fee;

        let balance = self.balances.entry(user).or_default();
        balance.djed += djed_amount;

        Ok(djed_amount)
    }

    /// Burn DJED to receive relay credits
    pub fn burn_djed(&mut self, user: &str, djed_amount: u64) -> Result<u64, &'static str> {
        let balance = self.balances.get_mut(user)
            .ok_or("User not found")?;

        if balance.djed < djed_amount {
            return Err("Insufficient DJED balance");
        }

        let credits = (djed_amount as f64 / self.config.target_price) as u64;
        let fee = (credits as f64 * self.config.fee_percentage) as u64;
        let net_credits = credits - fee;

        if self.reserve_credits < credits {
            return Err("Insufficient reserves");
        }

        balance.djed -= djed_amount;
        self.total_supply -= djed_amount;
        self.reserve_credits -= credits;
        self.elo_rewards_pool += fee;

        Ok(net_credits)
    }

    /// Lock DJED for ELO advancement
    pub fn lock_for_elo(&mut self, user: &str, amount: u64, target_elo: u32) -> Result<(), &'static str> {
        let balance = self.balances.get_mut(user)
            .ok_or("User not found")?;

        if balance.djed < amount {
            return Err("Insufficient DJED balance");
        }

        // Higher ELO requires more locked DJED
        let required = match target_elo {
            0..=1200 => 10,
            1201..=1500 => 25,
            1501..=1800 => 50,
            1801..=2000 => 100,
            _ => 200,
        };

        if amount < required {
            return Err("Insufficient amount for target ELO");
        }

        balance.djed -= amount;
        balance.locked_for_elo += amount;

        Ok(())
    }

    /// Distribute entropy rewards based on ELO
    pub fn distribute_entropy_reward(&mut self, user: &str, elo: u32) -> u64 {
        let reward = match elo {
            0..=1200 => 1,
            1201..=1500 => 2,
            1501..=1800 => 3,
            1801..=2000 => 5,
            _ => 8,
        };

        let actual_reward = reward.min(self.elo_rewards_pool);
        if actual_reward > 0 {
            self.elo_rewards_pool -= actual_reward;
            let balance = self.balances.entry(user.to_string()).or_default();
            balance.djed += actual_reward;
        }

        actual_reward
    }
}

/// Network mobility pricing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MobilityPricing {
    /// Base cost per relay hop in DJED
    pub base_hop_cost: u64,
    /// Cost multiplier for crossing ELO brackets
    pub elo_bracket_multiplier: f64,
    /// Guild-based discounts
    pub guild_discounts: HashMap<crate::guilds::Guild, f64>,
}

impl Default for MobilityPricing {
    fn default() -> Self {
        use crate::guilds::Guild;

        let mut discounts = HashMap::new();
        discounts.insert(Guild::Activity, 0.9);    // 10% discount for aggressive play
        discounts.insert(Guild::Reactivity, 1.0);  // Standard pricing
        discounts.insert(Guild::Avoidance, 1.1);   // 10% premium for careful play

        Self {
            base_hop_cost: 1,
            elo_bracket_multiplier: 1.5,
            guild_discounts: discounts,
        }
    }
}

impl MobilityPricing {
    /// Calculate cost for a relay hop
    pub fn calculate_hop_cost(
        &self,
        from_elo: u32,
        to_elo: u32,
        guild: crate::guilds::Guild
    ) -> u64 {
        let mut cost = self.base_hop_cost as f64;

        // Apply ELO bracket crossing penalty
        let from_bracket = from_elo / 300;
        let to_bracket = to_elo / 300;
        if to_bracket > from_bracket {
            cost *= self.elo_bracket_multiplier.powf((to_bracket - from_bracket) as f64);
        }

        // Apply guild discount
        if let Some(discount) = self.guild_discounts.get(&guild) {
            cost *= discount;
        }

        cost.ceil() as u64
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::guilds::Guild;

    #[test]
    fn test_mint_burn_cycle() {
        let mut system = StablecoinSystem::new();

        // Mint 100 DJED with 100 credits
        let minted = system.mint_djed("alice".to_string(), 100).unwrap();
        assert!(minted > 0);
        assert_eq!(system.balances["alice"].djed, minted);

        // Burn half
        let burned = system.burn_djed("alice", minted / 2).unwrap();
        assert!(burned > 0);
        assert_eq!(system.balances["alice"].djed, minted / 2);
    }

    #[test]
    fn test_elo_locking() {
        let mut system = StablecoinSystem::new();
        system.mint_djed("bob".to_string(), 200).unwrap();

        // Lock for 1500 ELO bracket
        assert!(system.lock_for_elo("bob", 25, 1500).is_ok());
        assert_eq!(system.balances["bob"].locked_for_elo, 25);
    }

    #[test]
    fn test_mobility_pricing() {
        let pricing = MobilityPricing::default();

        // Same bracket hop
        let cost1 = pricing.calculate_hop_cost(1200, 1250, Guild::Activity);
        assert_eq!(cost1, 0); // 10% discount rounds down

        // Cross bracket hop
        let cost2 = pricing.calculate_hop_cost(1200, 1500, Guild::Avoidance);
        assert!(cost2 > cost1);
    }
}