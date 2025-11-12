use anchor_lang::prelude::*;

#[event]
pub struct MarketCreated {
    pub market: Pubkey,
    pub match_id: u64,
    pub scheduled_match: Pubkey,
    pub currency: u8, // 0=SOL, 1=USDC, 2=USDT
    pub initial_yes_pool: u64,
    pub initial_no_pool: u64,
    pub betting_start: i64,
    pub betting_end: i64,
}

#[event]
pub struct SharesPurchased {
    pub market: Pubkey,
    pub buyer: Pubkey,
    pub is_yes: bool,
    pub amount_in: u64,
    pub shares_out: u64,
    pub price_impact_bps: u16,
    pub fee_paid: u64,
    pub new_yes_pool: u64,
    pub new_no_pool: u64,
}

#[event]
pub struct SharesSold {
    pub market: Pubkey,
    pub seller: Pubkey,
    pub is_yes: bool,
    pub shares_in: u64,
    pub amount_out: u64,
    pub fee_paid: u64,
    pub new_yes_pool: u64,
    pub new_no_pool: u64,
}

#[event]
pub struct LiquidityAdded {
    pub market: Pubkey,
    pub provider: Pubkey,
    pub yes_amount: u64,
    pub no_amount: u64,
    pub lp_shares_minted: u64,
    pub total_liquidity: u64,
}

#[event]
pub struct LiquidityRemoved {
    pub market: Pubkey,
    pub provider: Pubkey,
    pub lp_shares_burned: u64,
    pub yes_amount: u64,
    pub no_amount: u64,
    pub fees_earned: u64,
}

#[event]
pub struct MarketLocked {
    pub market: Pubkey,
    pub locked_at: i64,
    pub final_yes_pool: u64,
    pub final_no_pool: u64,
    pub final_yes_odds_bps: u16, // Implied probability in bps
    pub final_no_odds_bps: u16,
    pub total_volume: u64,
}

#[event]
pub struct MarketResolved {
    pub market: Pubkey,
    pub scheduled_match: Pubkey,
    pub battle: Pubkey,
    pub outcome: bool, // true = YES wins (player1), false = NO wins (player2)
    pub winner: Pubkey,
    pub resolved_at: i64,
    pub total_yes_shares: u64,
    pub total_no_shares: u64,
    pub total_payout: u64,
}

#[event]
pub struct WinningsClaimed {
    pub market: Pubkey,
    pub claimer: Pubkey,
    pub shares_burned: u64,
    pub payout: u64,
    pub currency: u8,
}

#[event]
pub struct ScheduledMatchCreated {
    pub match_id: u64,
    pub scheduled_match: Pubkey,
    pub player1: Pubkey,
    pub player2: Pubkey,
    pub battle_offer: Pubkey,
    pub betting_start: i64,
    pub betting_end: i64,
    pub estimated_battle_time: i64,
}

#[event]
pub struct FeesCollected {
    pub market: Pubkey,
    pub trade_amount: u64,
    pub lp_fee: u64,
    pub protocol_fee: u64,
    pub creator_fee: u64,
    pub total_fee: u64,
}

#[event]
pub struct PriceUpdate {
    pub market: Pubkey,
    pub yes_pool: u64,
    pub no_pool: u64,
    pub yes_price_bps: u16, // Price per share in bps
    pub no_price_bps: u16,
    pub k: u128, // Constant product
}

#[event]
pub struct SlippageProtectionTriggered {
    pub market: Pubkey,
    pub trader: Pubkey,
    pub expected_shares: u64,
    pub actual_shares: u64,
    pub slippage_bps: u16,
}
