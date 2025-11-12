use anchor_lang::prelude::*;
use anchor_lang::system_program;
use crate::amm::*;
use crate::constants::*;
use crate::errors::PredictionMarketError;
use crate::events::*;
use crate::state::*;

#[derive(Accounts)]
pub struct SellShares<'info> {
    #[account(mut)]
    pub market: Account<'info, Market>,

    #[account(
        mut,
        seeds = [POSITION_SEED, market.key().as_ref(), seller.key().as_ref()],
        bump = position.bump
    )]
    pub position: Account<'info, Position>,

    #[account(mut)]
    pub seller: Signer<'info>,

    pub system_program: Program<'info, System>,
}

pub fn handler(
    ctx: Context<SellShares>,
    shares_in: u64,
    is_yes: bool,
    min_amount_out: u64,
    max_slippage_bps: u16,
) -> Result<()> {
    let market = &mut ctx.accounts.market;
    let position = &mut ctx.accounts.position;
    let clock = Clock::get()?;

    // Validate market is open
    require!(
        market.status == MarketStatus::Open,
        PredictionMarketError::MarketLocked
    );

    // Validate betting period hasn't ended
    require!(
        clock.unix_timestamp < market.betting_end_time,
        PredictionMarketError::BettingPeriodEnded
    );

    // Validate shares amount
    require!(
        shares_in >= MIN_TRADE_AMOUNT,
        PredictionMarketError::AmountBelowMinimum
    );

    // Validate user has enough shares
    let user_shares = if is_yes {
        position.yes_shares
    } else {
        position.no_shares
    };
    require!(
        user_shares >= shares_in,
        PredictionMarketError::InsufficientShares
    );

    // Get reserves based on share type
    // When selling YES shares, we're adding to NO pool (buying NO)
    // and removing from YES pool (receiving currency)
    let (reserve_in, reserve_out) = if is_yes {
        (market.no_pool, market.yes_pool)
    } else {
        (market.yes_pool, market.no_pool)
    };

    // Calculate amount out using AMM
    let (amount_out, fee_amount) =
        calculate_amount_out(shares_in, reserve_in, reserve_out, market.fee_bps)?;

    // Calculate price impact (simplified for sell orders)
    let price_impact = 0u16; // TODO: Implement price impact calculation for sell orders

    // Check slippage protection
    check_slippage(min_amount_out, amount_out, max_slippage_bps)?;

    // Validate pool has enough liquidity
    require!(
        amount_out <= reserve_out,
        PredictionMarketError::InsufficientLiquidity
    );

    // Transfer SOL from market to seller (for SOL markets)
    if market.currency == Currency::SOL {
        **market.to_account_info().try_borrow_mut_lamports()? = market
            .to_account_info()
            .lamports()
            .checked_sub(amount_out)
            .ok_or(PredictionMarketError::InsufficientFunds)?;
        **ctx.accounts.seller.try_borrow_mut_lamports()? = ctx
            .accounts
            .seller
            .lamports()
            .checked_add(amount_out)
            .ok_or(PredictionMarketError::MathOverflow)?;
    }

    // Update market reserves
    if is_yes {
        // Selling YES: add shares to YES pool, remove currency from NO pool
        market.yes_pool = market.yes_pool
            .checked_add(shares_in)
            .ok_or(PredictionMarketError::MathOverflow)?;
        market.no_pool = market.no_pool
            .checked_sub(amount_out)
            .ok_or(PredictionMarketError::MathOverflow)?;
    } else {
        // Selling NO: add shares to NO pool, remove currency from YES pool
        market.no_pool = market.no_pool
            .checked_add(shares_in)
            .ok_or(PredictionMarketError::MathOverflow)?;
        market.yes_pool = market.yes_pool
            .checked_sub(amount_out)
            .ok_or(PredictionMarketError::MathOverflow)?;
    }
    market.k_last = (market.yes_pool as u128) * (market.no_pool as u128);

    // Distribute fees
    let lp_fee = ((fee_amount as u128 * LP_FEE_BPS as u128) / TRADING_FEE_BPS as u128) as u64;
    let protocol_fee = ((fee_amount as u128 * PROTOCOL_FEE_BPS as u128) / TRADING_FEE_BPS as u128) as u64;
    let creator_fee = fee_amount
        .checked_sub(lp_fee)
        .and_then(|v| v.checked_sub(protocol_fee))
        .ok_or(PredictionMarketError::MathOverflow)?;

    market.lp_fees = market.lp_fees.checked_add(lp_fee).ok_or(PredictionMarketError::MathOverflow)?;
    market.protocol_fees = market.protocol_fees.checked_add(protocol_fee).ok_or(PredictionMarketError::MathOverflow)?;
    market.creator_fees = market.creator_fees.checked_add(creator_fee).ok_or(PredictionMarketError::MathOverflow)?;
    market.total_fees_collected = market.total_fees_collected.checked_add(fee_amount).ok_or(PredictionMarketError::MathOverflow)?;

    // Update volume
    market.total_volume = market.total_volume
        .checked_add(amount_out)
        .ok_or(PredictionMarketError::MathOverflow)?;

    // Update position
    if is_yes {
        position.yes_shares = position.yes_shares
            .checked_sub(shares_in)
            .ok_or(PredictionMarketError::InsufficientShares)?;
    } else {
        position.no_shares = position.no_shares
            .checked_sub(shares_in)
            .ok_or(PredictionMarketError::InsufficientShares)?;
    }

    // Update realized P&L
    let sale_value = amount_out as i64;
    position.realized_pnl = position.realized_pnl
        .checked_add(sale_value)
        .ok_or(PredictionMarketError::MathOverflow)?;
    position.last_trade_at = clock.unix_timestamp;

    // Emit event
    emit!(SharesSold {
        market: market.key(),
        seller: ctx.accounts.seller.key(),
        is_yes,
        shares_in,
        amount_out,
        price_impact,
        fee_amount,
        yes_price: market.yes_price_bps(),
        no_price: market.no_price_bps(),
    });

    Ok(())
}
