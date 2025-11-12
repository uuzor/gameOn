use anchor_lang::prelude::*;
use anchor_lang::system_program;
use crate::amm::*;
use crate::constants::*;
use crate::errors::PredictionMarketError;
use crate::events::*;
use crate::state::*;

#[derive(Accounts)]
pub struct AddLiquidity<'info> {
    #[account(mut)]
    pub market: Account<'info, Market>,

    #[account(
        init_if_needed,
        payer = provider,
        space = LiquidityPosition::LEN,
        seeds = [LIQUIDITY_SEED, market.key().as_ref(), provider.key().as_ref()],
        bump
    )]
    pub liquidity_position: Account<'info, LiquidityPosition>,

    #[account(mut)]
    pub provider: Signer<'info>,

    pub system_program: Program<'info, System>,
}

pub fn handler(
    ctx: Context<AddLiquidity>,
    yes_amount: u64,
    no_amount: u64,
) -> Result<()> {
    let market = &mut ctx.accounts.market;
    let lp_position = &mut ctx.accounts.liquidity_position;
    let clock = Clock::get()?;

    // Validate market is open
    require!(
        market.status == MarketStatus::Open,
        PredictionMarketError::MarketLocked
    );

    // Validate amounts
    require!(
        yes_amount >= MIN_INITIAL_LIQUIDITY && no_amount >= MIN_INITIAL_LIQUIDITY,
        PredictionMarketError::AmountBelowMinimum
    );

    // Calculate LP shares
    let (lp_shares, yes_actual, no_actual) = calculate_lp_shares(
        yes_amount,
        no_amount,
        market.yes_pool,
        market.no_pool,
        market.total_liquidity,
    )?;

    // Transfer SOL from provider to market (for SOL markets)
    if market.currency == Currency::SOL {
        let total_amount = yes_actual.checked_add(no_actual).ok_or(PredictionMarketError::MathOverflow)?;
        system_program::transfer(
            CpiContext::new(
                ctx.accounts.system_program.to_account_info(),
                system_program::Transfer {
                    from: ctx.accounts.provider.to_account_info(),
                    to: market.to_account_info(),
                },
            ),
            total_amount,
        )?;
    }

    // Update market reserves
    market.yes_pool = market.yes_pool
        .checked_add(yes_actual)
        .ok_or(PredictionMarketError::MathOverflow)?;
    market.no_pool = market.no_pool
        .checked_add(no_actual)
        .ok_or(PredictionMarketError::MathOverflow)?;
    market.total_liquidity = market.total_liquidity
        .checked_add(lp_shares)
        .ok_or(PredictionMarketError::MathOverflow)?;
    market.k_last = (market.yes_pool as u128) * (market.no_pool as u128);

    // Initialize LP position if new
    if lp_position.provider == Pubkey::default() {
        lp_position.market = market.key();
        lp_position.provider = ctx.accounts.provider.key();
        lp_position.lp_shares = 0;
        lp_position.yes_deposited = 0;
        lp_position.no_deposited = 0;
        lp_position.entry_value = 0;
        lp_position.fees_earned = 0;
        lp_position.last_fee_collection = clock.unix_timestamp;
        lp_position.deposited_at = clock.unix_timestamp;
        lp_position.withdrawn_at = None;
        lp_position.withdrawn = false;
        lp_position.bump = ctx.bumps.liquidity_position;
    }

    // Update LP position
    lp_position.lp_shares = lp_position.lp_shares
        .checked_add(lp_shares)
        .ok_or(PredictionMarketError::MathOverflow)?;
    lp_position.yes_deposited = lp_position.yes_deposited
        .checked_add(yes_actual)
        .ok_or(PredictionMarketError::MathOverflow)?;
    lp_position.no_deposited = lp_position.no_deposited
        .checked_add(no_actual)
        .ok_or(PredictionMarketError::MathOverflow)?;
    lp_position.entry_value = lp_position.entry_value
        .checked_add(yes_actual + no_actual)
        .ok_or(PredictionMarketError::MathOverflow)?;

    // Emit event
    emit!(LiquidityAdded {
        market: market.key(),
        provider: ctx.accounts.provider.key(),
        yes_amount: yes_actual,
        no_amount: no_actual,
        lp_shares_minted: lp_shares,
        total_liquidity: market.total_liquidity,
    });

    Ok(())
}
