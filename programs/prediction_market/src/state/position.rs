use anchor_lang::prelude::*;

/// User's position in a specific market
#[account]
#[derive(Debug)]
pub struct Position {
    /// Market this position belongs to
    pub market: Pubkey,

    /// Owner of this position
    pub owner: Pubkey,

    // === Holdings ===
    /// Number of YES shares owned
    pub yes_shares: u64,

    /// Number of NO shares owned
    pub no_shares: u64,

    // === Entry Data (for analytics) ===
    /// Average entry price for YES shares (in basis points)
    pub avg_yes_entry_price: u64,

    /// Average entry price for NO shares (in basis points)
    pub avg_no_entry_price: u64,

    /// Total amount invested (in market currency)
    pub total_invested: u64,

    // === P&L Tracking ===
    /// Realized profit/loss from trades
    pub realized_pnl: i64,

    /// Number of trades made
    pub trade_count: u64,

    // === Lifecycle ===
    /// When position was first created
    pub created_at: i64,

    /// Last trade timestamp
    pub last_trade_at: i64,

    // === Claiming ===
    /// Whether winnings have been claimed
    pub claimed: bool,

    /// Amount claimed (if winner)
    pub payout_amount: u64,

    /// PDA bump
    pub bump: u8,
}

impl Position {
    pub const LEN: usize = 8 + // discriminator
        32 + // market
        32 + // owner
        8 + // yes_shares
        8 + // no_shares
        8 + // avg_yes_entry_price
        8 + // avg_no_entry_price
        8 + // total_invested
        8 + // realized_pnl (i64)
        8 + // trade_count
        8 + // created_at
        8 + // last_trade_at
        1 + // claimed
        8 + // payout_amount
        1; // bump

    /// Calculate current position value based on market prices
    pub fn current_value(&self, yes_price_bps: u16, no_price_bps: u16) -> u64 {
        let yes_value = (self.yes_shares as u128 * yes_price_bps as u128) / 10000;
        let no_value = (self.no_shares as u128 * no_price_bps as u128) / 10000;
        (yes_value + no_value) as u64
    }

    /// Calculate unrealized P&L
    pub fn unrealized_pnl(&self, yes_price_bps: u16, no_price_bps: u16) -> i64 {
        let current = self.current_value(yes_price_bps, no_price_bps) as i64;
        let invested = self.total_invested as i64;
        current - invested
    }

    /// Total P&L (realized + unrealized)
    pub fn total_pnl(&self, yes_price_bps: u16, no_price_bps: u16) -> i64 {
        self.realized_pnl + self.unrealized_pnl(yes_price_bps, no_price_bps)
    }

    /// Check if position can claim (has winning shares and not claimed)
    pub fn can_claim(&self, outcome: Option<bool>) -> bool {
        if self.claimed {
            return false;
        }

        match outcome {
            Some(true) => self.yes_shares > 0,  // YES won
            Some(false) => self.no_shares > 0,  // NO won
            None => false,                      // Not resolved
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_position_value() {
        let position = Position {
            market: Pubkey::default(),
            owner: Pubkey::default(),
            yes_shares: 100,
            no_shares: 50,
            avg_yes_entry_price: 5000,
            avg_no_entry_price: 5000,
            total_invested: 150,
            realized_pnl: 0,
            trade_count: 2,
            created_at: 0,
            last_trade_at: 0,
            claimed: false,
            payout_amount: 0,
            bump: 0,
        };

        // At 60/40 odds
        let value = position.current_value(6000, 4000);
        // 100 * 0.6 + 50 * 0.4 = 60 + 20 = 80
        assert_eq!(value, 80);

        // Unrealized P&L = 80 - 150 = -70
        let pnl = position.unrealized_pnl(6000, 4000);
        assert_eq!(pnl, -70);
    }

    #[test]
    fn test_can_claim() {
        let mut position = Position {
            market: Pubkey::default(),
            owner: Pubkey::default(),
            yes_shares: 100,
            no_shares: 0,
            avg_yes_entry_price: 5000,
            avg_no_entry_price: 0,
            total_invested: 100,
            realized_pnl: 0,
            trade_count: 1,
            created_at: 0,
            last_trade_at: 0,
            claimed: false,
            payout_amount: 0,
            bump: 0,
        };

        // Can claim if YES wins
        assert!(position.can_claim(Some(true)));
        // Cannot claim if NO wins
        assert!(!position.can_claim(Some(false)));
        // Cannot claim if not resolved
        assert!(!position.can_claim(None));

        // Cannot claim if already claimed
        position.claimed = true;
        assert!(!position.can_claim(Some(true)));
    }
}
