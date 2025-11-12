use anchor_lang::prelude::*;
use crate::constants::*;
use crate::errors::PredictionMarketError;
use crate::events::*;
use crate::state::*;

#[derive(Accounts)]
pub struct ResolveMarket<'info> {
    #[account(
        mut,
        seeds = [MARKET_SEED, market.match_id.to_le_bytes().as_ref()],
        bump = market.bump
    )]
    pub market: Account<'info, Market>,

    #[account(
        seeds = [SCHEDULED_MATCH_SEED, market.match_id.to_le_bytes().as_ref()],
        bump
    )]
    pub scheduled_match: Account<'info, ScheduledMatch>,

    /// Authority that can resolve markets (could be oracle, admin, or automated)
    pub resolver: Signer<'info>,
}

pub fn handler(ctx: Context<ResolveMarket>) -> Result<()> {
    let market = &mut ctx.accounts.market;
    let scheduled_match = &ctx.accounts.scheduled_match;
    let clock = Clock::get()?;

    // Validate market is in correct state
    require!(
        market.status == MarketStatus::Locked || market.status == MarketStatus::Open,
        PredictionMarketError::InvalidMarketStatus
    );

    // Validate betting period has ended
    require!(
        clock.unix_timestamp >= market.betting_end_time,
        PredictionMarketError::BettingPeriodNotEnded
    );

    // Validate match has been resolved
    require!(
        scheduled_match.status == MatchStatus::Resolved,
        PredictionMarketError::MatchNotCompleted
    );

    // Get winner from scheduled match
    let winner = scheduled_match.winner.ok_or(PredictionMarketError::NoWinnerSet)?;

    // Determine outcome based on winner
    // YES outcome = player1 wins, NO outcome = player2 wins
    let outcome = winner == scheduled_match.player1;

    // Update market state
    market.status = MarketStatus::Resolved;
    market.outcome = Some(outcome);
    market.winner = Some(winner);
    market.battle = Some(scheduled_match.game_reference); // Store game reference
    market.resolved_at = Some(clock.unix_timestamp);

    // Calculate final prices for the event
    let final_yes_price = market.yes_price_bps();
    let final_no_price = market.no_price_bps();

    // Emit resolution event
    emit!(MarketResolved {
        market: market.key(),
        match_id: market.match_id,
        outcome,
        winner,
        yes_pool: market.yes_pool,
        no_pool: market.no_pool,
        total_volume: market.total_volume,
        final_yes_price,
        final_no_price,
        resolved_at: market.resolved_at.unwrap(),
    });

    Ok(())
}

#[derive(Accounts)]
pub struct CancelMarket<'info> {
    #[account(
        mut,
        seeds = [MARKET_SEED, market.match_id.to_le_bytes().as_ref()],
        bump = market.bump
    )]
    pub market: Account<'info, Market>,

    #[account(
        seeds = [SCHEDULED_MATCH_SEED, market.match_id.to_le_bytes().as_ref()],
        bump
    )]
    pub scheduled_match: Account<'info, ScheduledMatch>,

    /// Authority that can cancel markets
    pub authority: Signer<'info>,
}

pub fn cancel_market_handler(ctx: Context<CancelMarket>) -> Result<()> {
    let market = &mut ctx.accounts.market;
    let scheduled_match = &ctx.accounts.scheduled_match;
    let clock = Clock::get()?;

    // Validate market hasn't been resolved
    require!(
        market.status != MarketStatus::Resolved,
        PredictionMarketError::MarketAlreadyResolved
    );

    // Validate match was cancelled
    require!(
        scheduled_match.status == MatchStatus::Cancelled,
        PredictionMarketError::InvalidMatchStatus
    );

    // Update market state
    market.status = MarketStatus::Cancelled;
    market.resolved_at = Some(clock.unix_timestamp);

    // Emit cancellation event
    emit!(MarketCancelled {
        market: market.key(),
        match_id: market.match_id,
        reason: "Match was cancelled".to_string(),
        cancelled_at: clock.unix_timestamp,
    });

    Ok(())
}
