use anchor_lang::prelude::*;
use crate::constants::*;
use crate::errors::PredictionMarketError;
use crate::events::*;
use crate::state::*;

#[derive(Accounts)]
#[instruction(match_id: u64)]
pub struct CreateMarket<'info> {
    #[account(
        init,
        payer = creator,
        space = Market::LEN,
        seeds = [MARKET_SEED, match_id.to_le_bytes().as_ref()],
        bump
    )]
    pub market: Account<'info, Market>,

    #[account(
        seeds = [SCHEDULED_MATCH_SEED, match_id.to_le_bytes().as_ref()],
        bump
    )]
    pub scheduled_match: Account<'info, ScheduledMatch>,

    #[account(mut)]
    pub creator: Signer<'info>,

    pub system_program: Program<'info, System>,
}

pub fn handler(
    ctx: Context<CreateMarket>,
    match_id: u64,
    initial_yes_pool: u64,
    initial_no_pool: u64,
) -> Result<()> {
    let market = &mut ctx.accounts.market;
    let scheduled_match = &ctx.accounts.scheduled_match;
    let clock = Clock::get()?;

    // Validate initial liquidity
    require!(
        initial_yes_pool >= MIN_INITIAL_LIQUIDITY,
        PredictionMarketError::InsufficientLiquidity
    );
    require!(
        initial_no_pool >= MIN_INITIAL_LIQUIDITY,
        PredictionMarketError::InsufficientLiquidity
    );

    // Validate match is not already started
    require!(
        scheduled_match.status == MatchStatus::Open,
        PredictionMarketError::InvalidMarketStatus
    );

    // Initialize market
    market.market_id = match_id;
    market.match_id = match_id;
    market.scheduled_match = scheduled_match.key();
    market.yes_pool = initial_yes_pool;
    market.no_pool = initial_no_pool;
    market.k_last = (initial_yes_pool as u128) * (initial_no_pool as u128);
    market.total_liquidity = 0; // Will be set when first LP adds liquidity
    market.fee_bps = TRADING_FEE_BPS;
    market.total_volume = 0;
    market.total_fees_collected = 0;
    market.lp_fees = 0;
    market.protocol_fees = 0;
    market.creator_fees = 0;
    market.currency = Currency::SOL; // Default to SOL
    market.vault = None;
    market.created_at = clock.unix_timestamp;
    market.betting_end_time = scheduled_match.betting_end;
    market.resolved_at = None;
    market.status = MarketStatus::Open;
    market.outcome = None;
    market.winner = None;
    market.battle = None;
    market.creator = ctx.accounts.creator.key();
    market.bump = ctx.bumps.market;

    // Emit event
    emit!(MarketCreated {
        market: market.key(),
        match_id,
        scheduled_match: scheduled_match.key(),
        currency: market.currency.to_u8(),
        initial_yes_pool,
        initial_no_pool,
        betting_start: scheduled_match.betting_start,
        betting_end: scheduled_match.betting_end,
    });

    Ok(())
}
