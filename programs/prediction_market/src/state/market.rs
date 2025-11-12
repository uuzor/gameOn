use anchor_lang::prelude::*;

/// Currency type for the market
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub enum Currency {
    SOL,
    USDC,
    USDT,
}

impl Currency {
    pub fn decimals(&self) -> u8 {
        match self {
            Currency::SOL => 9,
            Currency::USDC => 6,
            Currency::USDT => 6,
        }
    }

    pub fn to_u8(&self) -> u8 {
        match self {
            Currency::SOL => 0,
            Currency::USDC => 1,
            Currency::USDT => 2,
        }
    }
}

/// Market status
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub enum MarketStatus {
    /// Market is open for trading
    Open,
    /// Betting period ended, waiting for battle
    Locked,
    /// Battle completed, market resolved
    Resolved,
    /// Market cancelled (battle didn't happen)
    Cancelled,
}

/// Main prediction market account
/// Uses AMM (constant product) for pricing
#[account]
#[derive(Debug)]
pub struct Market {
    /// Unique market identifier
    pub market_id: u64,

    /// Reference to scheduled match
    pub match_id: u64,
    pub scheduled_match: Pubkey,

    // === AMM State ===
    /// YES pool reserves (in currency decimals)
    pub yes_pool: u64,

    /// NO pool reserves (in currency decimals)
    pub no_pool: u64,

    /// Constant product (k = x * y)
    /// Stored as u128 to handle large values
    pub k_last: u128,

    // === Liquidity ===
    /// Total LP shares issued
    pub total_liquidity: u64,

    // === Economics ===
    /// Trading fee in basis points (40 bps = 0.4%)
    pub fee_bps: u16,

    /// Total trading volume (in currency)
    pub total_volume: u64,

    /// Total fees collected
    pub total_fees_collected: u64,

    /// Fees allocated to LPs (accumulated)
    pub lp_fees: u64,

    /// Fees allocated to protocol
    pub protocol_fees: u64,

    /// Fees allocated to match creator
    pub creator_fees: u64,

    // === Currency ===
    /// Market currency (SOL, USDC, or USDT)
    pub currency: Currency,

    /// Token vault for SPL tokens (None for SOL)
    pub vault: Option<Pubkey>,

    // === Timing ===
    /// When market was created
    pub created_at: i64,

    /// When betting period ends
    pub betting_end_time: i64,

    /// When market was resolved
    pub resolved_at: Option<i64>,

    // === Status ===
    /// Current market status
    pub status: MarketStatus,

    /// Outcome after resolution (true = YES wins, false = NO wins)
    pub outcome: Option<bool>,

    /// Winner of the battle
    pub winner: Option<Pubkey>,

    // === References ===
    /// Reference to BattleChain battle for resolution
    pub battle: Option<Pubkey>,

    /// Match creator who receives fees
    pub creator: Pubkey,

    /// PDA bump
    pub bump: u8,
}

impl Market {
    pub const LEN: usize = 8 + // discriminator
        8 + // market_id
        8 + // match_id
        32 + // scheduled_match
        8 + // yes_pool
        8 + // no_pool
        16 + // k_last
        8 + // total_liquidity
        2 + // fee_bps
        8 + // total_volume
        8 + // total_fees_collected
        8 + // lp_fees
        8 + // protocol_fees
        8 + // creator_fees
        1 + // currency enum
        (1 + 32) + // vault Option<Pubkey>
        8 + // created_at
        8 + // betting_end_time
        (1 + 8) + // resolved_at Option<i64>
        1 + // status enum
        (1 + 1) + // outcome Option<bool>
        (1 + 32) + // winner Option<Pubkey>
        (1 + 32) + // battle Option<Pubkey>
        32 + // creator
        1; // bump

    /// Check if market is open for trading
    pub fn is_open(&self) -> bool {
        self.status == MarketStatus::Open
    }

    /// Check if betting period has ended
    pub fn is_betting_ended(&self, current_time: i64) -> bool {
        current_time >= self.betting_end_time
    }

    /// Check if market is resolved
    pub fn is_resolved(&self) -> bool {
        self.status == MarketStatus::Resolved
    }

    /// Get current YES price in basis points (0-10000)
    /// Price = yes_pool / (yes_pool + no_pool) * 10000
    pub fn yes_price_bps(&self) -> u16 {
        if self.yes_pool == 0 && self.no_pool == 0 {
            return 5000; // 50% if no liquidity
        }
        let total = self.yes_pool as u128 + self.no_pool as u128;
        let price = (self.yes_pool as u128 * 10000) / total;
        price.min(10000) as u16
    }

    /// Get current NO price in basis points
    pub fn no_price_bps(&self) -> u16 {
        10000 - self.yes_price_bps()
    }

    /// Calculate implied probabilities
    pub fn implied_probability_yes(&self) -> f64 {
        self.yes_price_bps() as f64 / 10000.0
    }

    pub fn implied_probability_no(&self) -> f64 {
        self.no_price_bps() as f64 / 10000.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_currency_decimals() {
        assert_eq!(Currency::SOL.decimals(), 9);
        assert_eq!(Currency::USDC.decimals(), 6);
        assert_eq!(Currency::USDT.decimals(), 6);
    }

    #[test]
    fn test_market_prices() {
        let mut market = Market {
            market_id: 1,
            match_id: 1,
            scheduled_match: Pubkey::default(),
            yes_pool: 100,
            no_pool: 100,
            k_last: 10000,
            total_liquidity: 0,
            fee_bps: 40,
            total_volume: 0,
            total_fees_collected: 0,
            lp_fees: 0,
            protocol_fees: 0,
            creator_fees: 0,
            currency: Currency::SOL,
            vault: None,
            created_at: 0,
            betting_end_time: 1000,
            resolved_at: None,
            status: MarketStatus::Open,
            outcome: None,
            winner: None,
            battle: None,
            creator: Pubkey::default(),
            bump: 0,
        };

        // Equal pools = 50/50 odds
        assert_eq!(market.yes_price_bps(), 5000);
        assert_eq!(market.no_price_bps(), 5000);

        // Skewed pools
        market.yes_pool = 150;
        market.no_pool = 50;
        assert_eq!(market.yes_price_bps(), 7500); // 75%
        assert_eq!(market.no_price_bps(), 2500); // 25%
    }

    #[test]
    fn test_market_status_checks() {
        let market = Market {
            market_id: 1,
            match_id: 1,
            scheduled_match: Pubkey::default(),
            yes_pool: 100,
            no_pool: 100,
            k_last: 10000,
            total_liquidity: 0,
            fee_bps: 40,
            total_volume: 0,
            total_fees_collected: 0,
            lp_fees: 0,
            protocol_fees: 0,
            creator_fees: 0,
            currency: Currency::SOL,
            vault: None,
            created_at: 0,
            betting_end_time: 1000,
            resolved_at: None,
            status: MarketStatus::Open,
            outcome: None,
            winner: None,
            battle: None,
            creator: Pubkey::default(),
            bump: 0,
        };

        assert!(market.is_open());
        assert!(!market.is_resolved());
        assert!(market.is_betting_ended(1001));
        assert!(!market.is_betting_ended(999));
    }
}
