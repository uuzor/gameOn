use anchor_lang::prelude::*;

pub mod amm;
pub mod constants;
pub mod errors;
pub mod events;
pub mod instructions;
pub mod state;

use instructions::*;

declare_id!("11111111111111111111111111111111");

#[program]
pub mod prediction_market {
    use super::*;

    /// Create a new prediction market for a scheduled match
    pub fn create_market(
        ctx: Context<CreateMarket>,
        match_id: u64,
        initial_yes_pool: u64,
        initial_no_pool: u64,
    ) -> Result<()> {
        instructions::create_market::handler(ctx, match_id, initial_yes_pool, initial_no_pool)
    }

    /// Buy shares (YES or NO) in a prediction market
    pub fn buy_shares(
        ctx: Context<BuyShares>,
        amount_in: u64,
        is_yes: bool,
        min_shares_out: u64,
        max_slippage_bps: u16,
    ) -> Result<()> {
        instructions::buy_shares::handler(ctx, amount_in, is_yes, min_shares_out, max_slippage_bps)
    }

    /// Sell shares (YES or NO) back to the market
    pub fn sell_shares(
        ctx: Context<SellShares>,
        shares_in: u64,
        is_yes: bool,
        min_amount_out: u64,
        max_slippage_bps: u16,
    ) -> Result<()> {
        instructions::sell_shares::handler(ctx, shares_in, is_yes, min_amount_out, max_slippage_bps)
    }

    /// Add liquidity to a market (become an LP)
    pub fn add_liquidity(
        ctx: Context<AddLiquidity>,
        yes_amount: u64,
        no_amount: u64,
    ) -> Result<()> {
        instructions::add_liquidity::handler(ctx, yes_amount, no_amount)
    }

    /// Remove liquidity from a market (burn LP shares)
    pub fn remove_liquidity(
        ctx: Context<RemoveLiquidity>,
        lp_shares: u64,
        min_yes_out: u64,
        min_no_out: u64,
    ) -> Result<()> {
        instructions::remove_liquidity::handler(ctx, lp_shares, min_yes_out, min_no_out)
    }

    /// Resolve a market based on battle outcome
    pub fn resolve_market(ctx: Context<ResolveMarket>) -> Result<()> {
        instructions::resolve_market::handler(ctx)
    }

    /// Cancel a market if the match was cancelled
    pub fn cancel_market(ctx: Context<CancelMarket>) -> Result<()> {
        instructions::resolve_market::cancel_market_handler(ctx)
    }

    /// Claim winnings after market resolution
    pub fn claim_winnings(ctx: Context<ClaimWinnings>) -> Result<()> {
        instructions::claim_winnings::handler(ctx)
    }

    /// Claim refund if market was cancelled
    pub fn claim_refund(ctx: Context<ClaimRefund>) -> Result<()> {
        instructions::claim_winnings::claim_refund_handler(ctx)
    }

    /// Create a scheduled match
    pub fn create_scheduled_match(
        ctx: Context<CreateScheduledMatch>,
        match_id: u64,
        player1: Pubkey,
        player2: Pubkey,
        game_reference: Pubkey,
        is_external: bool,
        betting_start: i64,
        betting_end: i64,
        estimated_battle_time: i64,
        title: String,
        description: String,
    ) -> Result<()> {
        instructions::scheduled_match::create_scheduled_match_handler(
            ctx,
            match_id,
            player1,
            player2,
            game_reference,
            is_external,
            betting_start,
            betting_end,
            estimated_battle_time,
            title,
            description,
        )
    }

    /// Start a scheduled match
    pub fn start_match(ctx: Context<UpdateScheduledMatch>) -> Result<()> {
        instructions::scheduled_match::start_match_handler(ctx)
    }

    /// Complete a scheduled match with winner
    pub fn complete_scheduled_match(
        ctx: Context<UpdateScheduledMatch>,
        winner: Pubkey,
        battle_account: Pubkey,
    ) -> Result<()> {
        instructions::scheduled_match::complete_match_handler(ctx, winner, battle_account)
    }

    /// Cancel a scheduled match
    pub fn cancel_scheduled_match(ctx: Context<UpdateScheduledMatch>) -> Result<()> {
        instructions::scheduled_match::cancel_match_handler(ctx)
    }
}
