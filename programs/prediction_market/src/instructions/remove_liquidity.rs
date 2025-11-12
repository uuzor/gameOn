use anchor_lang::prelude::*;
use anchor_lang::system_program;
use crate::amm::*;
use crate::constants::*;
use crate::errors::PredictionMarketError;
use crate::events::*;
use crate::state::*;

#[derive(Accounts)]
pub struct RemoveLiquidity<'info> {
    #[account(mut)]
    pub market: Account<'info, Market>,

    #[account(
        mut,
        seeds = [LIQUIDITY_SEED, market.key().as_ref(), provider.key().as_ref()],
        bump = liquidity_position.bump
    )]
    pub liquidity_position: Account<'info, LiquidityPosition>,

    #[account(mut)]
    pub provider: Signer<'info>,

    pub system_program: Program<'info, System>,
}

pub fn handler(
    ctx: Context<RemoveLiquidity>,
    lp_shares: u64,
    min_yes_out: u64,
    min_no_out: u64,
) -> Result<()> {
    let market = &mut ctx.accounts.market;
    let lp_position = &mut ctx.accounts.liquidity_position;
    let clock = Clock::get()?;

    // Validate market is open
    require!(
        market.status == MarketStatus::Open,
        PredictionMarketError::MarketLocked
    );

    // Validate LP has enough shares
    require!(
        lp_position.lp_shares >= lp_shares,
        PredictionMarketError::InsufficientLPShares
    );

    require!(
        lp_shares > 0,
        PredictionMarketError::AmountBelowMinimum
    );

    // Calculate amounts to withdraw (includes proportional fees)
    let (yes_amount, no_amount, fees_earned) = calculate_lp_withdrawal(
        lp_shares,
        market.yes_pool,
        market.no_pool,
        market.total_liquidity,
        market.lp_fees,
    )?;

    // Check minimum output protection
    require!(
        yes_amount >= min_yes_out,
        PredictionMarketError::SlippageExceeded
    );
    require!(
        no_amount >= min_no_out,
        PredictionMarketError::SlippageExceeded
    );

    // Add fees to withdrawal amounts
    let total_yes_out = yes_amount.checked_add(fees_earned / 2).ok_or(PredictionMarketError::MathOverflow)?;
    let total_no_out = no_amount.checked_add(fees_earned / 2).ok_or(PredictionMarketError::MathOverflow)?;

    // Transfer SOL from market to provider (for SOL markets)
    if market.currency == Currency::SOL {
        let total_amount = total_yes_out.checked_add(total_no_out).ok_or(PredictionMarketError::MathOverflow)?;

        **market.to_account_info().try_borrow_mut_lamports()? = market
            .to_account_info()
            .lamports()
            .checked_sub(total_amount)
            .ok_or(PredictionMarketError::InsufficientFunds)?;
        **ctx.accounts.provider.try_borrow_mut_lamports()? = ctx
            .accounts
            .provider
            .lamports()
            .checked_add(total_amount)
            .ok_or(PredictionMarketError::MathOverflow)?;
    }

    // Update market reserves
    market.yes_pool = market.yes_pool
        .checked_sub(total_yes_out)
        .ok_or(PredictionMarketError::InsufficientLiquidity)?;
    market.no_pool = market.no_pool
        .checked_sub(total_no_out)
        .ok_or(PredictionMarketError::InsufficientLiquidity)?;
    market.total_liquidity = market.total_liquidity
        .checked_sub(lp_shares)
        .ok_or(PredictionMarketError::MathOverflow)?;
    market.k_last = (market.yes_pool as u128) * (market.no_pool as u128);

    // Update lp_fees (subtract distributed fees)
    market.lp_fees = market.lp_fees
        .checked_sub(fees_earned)
        .ok_or(PredictionMarketError::MathOverflow)?;

    // Update LP position
    lp_position.lp_shares = lp_position.lp_shares
        .checked_sub(lp_shares)
        .ok_or(PredictionMarketError::InsufficientLPShares)?;
    lp_position.yes_deposited = lp_position.yes_deposited
        .checked_sub(yes_amount.min(lp_position.yes_deposited))
        .ok_or(PredictionMarketError::MathOverflow)?;
    lp_position.no_deposited = lp_position.no_deposited
        .checked_sub(no_amount.min(lp_position.no_deposited))
        .ok_or(PredictionMarketError::MathOverflow)?;
    lp_position.fees_earned = lp_position.fees_earned
        .checked_add(fees_earned)
        .ok_or(PredictionMarketError::MathOverflow)?;
    lp_position.last_fee_collection = clock.unix_timestamp;

    // Mark as withdrawn if all shares removed
    if lp_position.lp_shares == 0 {
        lp_position.withdrawn = true;
        lp_position.withdrawn_at = Some(clock.unix_timestamp);
    }

    // Calculate impermanent loss
    let il = lp_position.calculate_il(
        market.yes_pool,
        market.no_pool,
        market.total_liquidity,
    );

    // Emit event
    emit!(LiquidityRemoved {
        market: market.key(),
        provider: ctx.accounts.provider.key(),
        lp_shares_burned: lp_shares,
        yes_amount: total_yes_out,
        no_amount: total_no_out,
        fees_earned,
        impermanent_loss: il,
        total_liquidity: market.total_liquidity,
    });

    Ok(())
}

/// Calculate LP's share of accumulated fees
fn calculate_lp_fees(
    lp_shares: u64,
    total_liquidity: u64,
    total_lp_fees: u64,
    already_collected: u64,
) -> Result<u64> {
    if total_liquidity == 0 {
        return Ok(0);
    }

    let share_percentage = (lp_shares as u128)
        .checked_mul(FP_SCALE)
        .and_then(|v| v.checked_div(total_liquidity as u128))
        .ok_or(PredictionMarketError::MathOverflow)?;

    let entitled_fees = ((total_lp_fees as u128)
        .checked_mul(share_percentage)
        .ok_or(PredictionMarketError::MathOverflow)? / FP_SCALE) as u64;

    let new_fees = entitled_fees
        .checked_sub(already_collected)
        .ok_or(PredictionMarketError::MathOverflow)?;

    Ok(new_fees)
}
