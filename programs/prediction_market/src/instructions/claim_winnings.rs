use anchor_lang::prelude::*;
use anchor_lang::system_program;
use crate::constants::*;
use crate::errors::PredictionMarketError;
use crate::events::*;
use crate::state::*;

#[derive(Accounts)]
pub struct ClaimWinnings<'info> {
    #[account(
        mut,
        seeds = [MARKET_SEED, market.match_id.to_le_bytes().as_ref()],
        bump = market.bump
    )]
    pub market: Account<'info, Market>,

    #[account(
        mut,
        seeds = [POSITION_SEED, market.key().as_ref(), claimer.key().as_ref()],
        bump = position.bump
    )]
    pub position: Account<'info, Position>,

    #[account(mut)]
    pub claimer: Signer<'info>,

    pub system_program: Program<'info, System>,
}

pub fn handler(ctx: Context<ClaimWinnings>) -> Result<()> {
    let market = &mut ctx.accounts.market;
    let position = &mut ctx.accounts.position;
    let clock = Clock::get()?;

    // Validate market is resolved
    require!(
        market.status == MarketStatus::Resolved,
        PredictionMarketError::MarketNotResolved
    );

    // Validate position hasn't been claimed
    require!(
        !position.claimed,
        PredictionMarketError::AlreadyClaimed
    );

    let outcome = market.outcome.ok_or(PredictionMarketError::NoOutcomeSet)?;

    // Calculate payout based on outcome
    let winning_shares = if outcome {
        position.yes_shares
    } else {
        position.no_shares
    };

    // Payout = 1:1 for winning shares (each share worth 1 unit of currency)
    let payout = winning_shares;

    require!(
        payout > 0,
        PredictionMarketError::NoWinningsToClaim
    );

    // Transfer winnings from market to claimer (for SOL markets)
    if market.currency == Currency::SOL {
        **market.to_account_info().try_borrow_mut_lamports()? = market
            .to_account_info()
            .lamports()
            .checked_sub(payout)
            .ok_or(PredictionMarketError::InsufficientFunds)?;
        **ctx.accounts.claimer.try_borrow_mut_lamports()? = ctx
            .accounts
            .claimer
            .lamports()
            .checked_add(payout)
            .ok_or(PredictionMarketError::MathOverflow)?;
    }

    // Calculate final P&L
    let initial_investment = position.total_invested;
    let net_pnl = (payout as i64)
        .checked_sub(initial_investment as i64)
        .ok_or(PredictionMarketError::MathOverflow)?;

    // Update position
    position.claimed = true;
    position.payout_amount = payout;
    position.realized_pnl = position.realized_pnl
        .checked_add(net_pnl)
        .ok_or(PredictionMarketError::MathOverflow)?;
    position.last_trade_at = clock.unix_timestamp;

    // Emit event
    emit!(WinningsClaimed {
        market: market.key(),
        claimer: ctx.accounts.claimer.key(),
        payout,
        outcome,
        net_pnl,
    });

    Ok(())
}

#[derive(Accounts)]
pub struct ClaimRefund<'info> {
    #[account(
        mut,
        seeds = [MARKET_SEED, market.match_id.to_le_bytes().as_ref()],
        bump = market.bump
    )]
    pub market: Account<'info, Market>,

    #[account(
        mut,
        seeds = [POSITION_SEED, market.key().as_ref(), claimer.key().as_ref()],
        bump = position.bump
    )]
    pub position: Account<'info, Position>,

    #[account(mut)]
    pub claimer: Signer<'info>,

    pub system_program: Program<'info, System>,
}

pub fn claim_refund_handler(ctx: Context<ClaimRefund>) -> Result<()> {
    let market = &mut ctx.accounts.market;
    let position = &mut ctx.accounts.position;
    let clock = Clock::get()?;

    // Validate market is cancelled
    require!(
        market.status == MarketStatus::Cancelled,
        PredictionMarketError::MarketNotCancelled
    );

    // Validate position hasn't been claimed
    require!(
        !position.claimed,
        PredictionMarketError::AlreadyClaimed
    );

    // Refund = total invested (all shares are refunded at cost)
    let refund = position.total_invested;

    require!(
        refund > 0,
        PredictionMarketError::NoRefundAvailable
    );

    // Transfer refund from market to claimer (for SOL markets)
    if market.currency == Currency::SOL {
        **market.to_account_info().try_borrow_mut_lamports()? = market
            .to_account_info()
            .lamports()
            .checked_sub(refund)
            .ok_or(PredictionMarketError::InsufficientFunds)?;
        **ctx.accounts.claimer.try_borrow_mut_lamports()? = ctx
            .accounts
            .claimer
            .lamports()
            .checked_add(refund)
            .ok_or(PredictionMarketError::MathOverflow)?;
    }

    // Update position
    position.claimed = true;
    position.payout_amount = refund;
    position.realized_pnl = 0; // No profit/loss on cancelled markets
    position.last_trade_at = clock.unix_timestamp;

    // Emit event
    emit!(RefundClaimed {
        market: market.key(),
        claimer: ctx.accounts.claimer.key(),
        refund_amount: refund,
    });

    Ok(())
}
