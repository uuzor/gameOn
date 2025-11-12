use anchor_lang::prelude::*;

/// Liquidity provider's position in a market
#[account]
#[derive(Debug)]
pub struct LiquidityPosition {
    /// Market this LP position belongs to
    pub market: Pubkey,

    /// Liquidity provider's wallet
    pub provider: Pubkey,

    // === LP Shares ===
    /// Number of LP shares owned
    /// Represents proportional ownership of the pool
    pub lp_shares: u64,

    // === Entry State (for IL calculation) ===
    /// YES tokens deposited at entry
    pub yes_deposited: u64,

    /// NO tokens deposited at entry
    pub no_deposited: u64,

    /// Total value at entry (in market currency)
    pub entry_value: u64,

    // === Fee Tracking ===
    /// Accumulated fees earned
    pub fees_earned: u64,

    /// Last fee collection timestamp
    pub last_fee_collection: i64,

    // === Lifecycle ===
    /// When liquidity was added
    pub deposited_at: i64,

    /// When liquidity was withdrawn (if applicable)
    pub withdrawn_at: Option<i64>,

    /// Whether position has been withdrawn
    pub withdrawn: bool,

    /// PDA bump
    pub bump: u8,
}

impl LiquidityPosition {
    pub const LEN: usize = 8 + // discriminator
        32 + // market
        32 + // provider
        8 + // lp_shares
        8 + // yes_deposited
        8 + // no_deposited
        8 + // entry_value
        8 + // fees_earned
        8 + // last_fee_collection
        8 + // deposited_at
        (1 + 8) + // withdrawn_at Option<i64>
        1 + // withdrawn
        1; // bump

    /// Calculate impermanent loss
    /// IL = (current_value - entry_value) / entry_value
    pub fn calculate_il(
        &self,
        current_yes_pool: u64,
        current_no_pool: u64,
        total_liquidity: u64,
    ) -> i64 {
        if total_liquidity == 0 {
            return 0;
        }

        // Calculate current value of this LP position
        let share_ratio = (self.lp_shares as u128 * 1_000_000) / total_liquidity as u128;
        let current_yes = (current_yes_pool as u128 * share_ratio) / 1_000_000;
        let current_no = (current_no_pool as u128 * share_ratio) / 1_000_000;
        let current_value = (current_yes + current_no) as u64;

        // IL = current - entry
        current_value as i64 - self.entry_value as i64
    }

    /// Calculate total returns (fees + IL)
    pub fn total_returns(
        &self,
        current_yes_pool: u64,
        current_no_pool: u64,
        total_liquidity: u64,
    ) -> i64 {
        let il = self.calculate_il(current_yes_pool, current_no_pool, total_liquidity);
        il + self.fees_earned as i64
    }

    /// Calculate APY based on time elapsed
    pub fn calculate_apy(&self, current_timestamp: i64) -> f64 {
        if self.entry_value == 0 {
            return 0.0;
        }

        let time_elapsed = (current_timestamp - self.deposited_at) as f64;
        if time_elapsed <= 0.0 {
            return 0.0;
        }

        // Convert to years
        let years = time_elapsed / (365.25 * 24.0 * 3600.0);
        if years <= 0.0 {
            return 0.0;
        }

        // APY = (fees_earned / entry_value) / years
        let return_rate = self.fees_earned as f64 / self.entry_value as f64;
        (return_rate / years) * 100.0 // Return as percentage
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_impermanent_loss() {
        let lp_position = LiquidityPosition {
            market: Pubkey::default(),
            provider: Pubkey::default(),
            lp_shares: 100,
            yes_deposited: 100,
            no_deposited: 100,
            entry_value: 200,
            fees_earned: 10,
            last_fee_collection: 0,
            deposited_at: 0,
            withdrawn_at: None,
            withdrawn: false,
            bump: 0,
        };

        // Scenario: Balanced exit (no IL)
        let il = lp_position.calculate_il(
            100, // current_yes_pool
            100, // current_no_pool
            100, // total_liquidity (LP owns 100%)
        );
        assert_eq!(il, 0); // No IL

        // Scenario: Imbalanced exit (YES won)
        let il = lp_position.calculate_il(
            150, // current_yes_pool (YES increased)
            50,  // current_no_pool (NO decreased)
            100, // total_liquidity
        );
        assert_eq!(il, 0); // Total is still 200, no IL

        // But if total pool shrunk (market resolved, some paid out)
        let il = lp_position.calculate_il(
            120, // current_yes_pool
            30,  // current_no_pool
            100, // total_liquidity
        );
        assert_eq!(il, -50); // Lost 50 due to IL
    }

    #[test]
    fn test_total_returns() {
        let lp_position = LiquidityPosition {
            market: Pubkey::default(),
            provider: Pubkey::default(),
            lp_shares: 100,
            yes_deposited: 100,
            no_deposited: 100,
            entry_value: 200,
            fees_earned: 60,
            last_fee_collection: 0,
            deposited_at: 0,
            withdrawn_at: None,
            withdrawn: false,
            bump: 0,
        };

        // IL = -50, fees = +60, total = +10
        let returns = lp_position.total_returns(120, 30, 100);
        assert_eq!(returns, 10);
    }

    #[test]
    fn test_calculate_apy() {
        let lp_position = LiquidityPosition {
            market: Pubkey::default(),
            provider: Pubkey::default(),
            lp_shares: 100,
            yes_deposited: 100,
            no_deposited: 100,
            entry_value: 100,
            fees_earned: 10,
            last_fee_collection: 0,
            deposited_at: 0,
            withdrawn_at: None,
            withdrawn: false,
            bump: 0,
        };

        // 10% return over 1 year = 10% APY
        let current_time = 365 * 24 * 3600; // 1 year later
        let apy = lp_position.calculate_apy(current_time);
        assert!((apy - 10.0).abs() < 0.1); // ~10% APY
    }
}
