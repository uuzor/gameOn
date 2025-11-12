use anchor_lang::prelude::*;
use anchor_lang::system_program;
use crate::amm::*;
use crate::constants::*;
use crate::errors::PredictionMarketError;
use crate::events::*;
use crate::state::*;

#[derive(Accounts)]
pub struct BuyShares<'info> {
    #[account(mut)]
    pub market: Account<'info, Market>,

    #[account(
        init_if_needed,
        payer = buyer,
        space = Position::LEN,
        seeds = [POSITION_SEED, market.key().as_ref(), buyer.key().as_ref()],
        bump
    )]
    pub position: Account<'info, Position>,

    #[account(mut)]
    pub buyer: Signer<'info>,

    pub system_program: Program<'info, System>,
}

pub fn handler(
    ctx: Context<BuyShares>,
    amount_in: u64,
    is_yes: bool,
    min_shares_out: u64,
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

    // Validate betting period
    require!(
        clock.unix_timestamp < market.betting_end_time,
        PredictionMarketError::BettingEnded
    );

    // Validate amount
    require!(
        amount_in >= MIN_TRADE_AMOUNT,
        PredictionMarketError::AmountBelowMinimum
    );

    // Calculate shares using AMM
    let (reserve_in, reserve_out) = if is_yes {
        (market.yes_pool, market.no_pool)
    } else {
        (market.no_pool, market.yes_pool)
    };

    let (shares_out, price_impact, fee_amount) =
        calculate_shares_out(amount_in, reserve_in, reserve_out, market.fee_bps)?;

    // Check slippage
    check_slippage(min_shares_out, shares_out, max_slippage_bps)?;

    // Transfer SOL from buyer to market (for SOL markets)
    if market.currency == Currency::SOL {
        system_program::transfer(
            CpiContext::new(
                ctx.accounts.system_program.to_account_info(),
                system_program::Transfer {
                    from: ctx.accounts.buyer.to_account_info(),
                    to: market.to_account_info(),
                },
            ),
            amount_in,
        )?;
    }

    // Update market reserves
    if is_yes {
        market.yes_pool = market.yes_pool
            .checked_add(amount_in)
            .ok_or(PredictionMarketError::MathOverflow)?;
        market.no_pool = market.no_pool
            .checked_sub(shares_out)
            .ok_or(PredictionMarketError::InsufficientLiquidity)?;
    } else {
        market.no_pool = market.no_pool
            .checked_add(amount_in)
            .ok_or(PredictionMarketError::MathOverflow)?;
        market.yes_pool = market.yes_pool
            .checked_sub(shares_out)
            .ok_or(PredictionMarketError::InsufficientLiquidity)?;
    }

    // Update k_last
    market.k_last = (market.yes_pool as u128) * (market.no_pool as u128);

    // Distribute fees
    let lp_fee = (fee_amount as u128 * LP_FEE_BPS as u128) / TRADING_FEE_BPS as u128;
    let protocol_fee = (fee_amount as u128 * PROTOCOL_FEE_BPS as u128) / TRADING_FEE_BPS as u128;
    let creator_fee = fee_amount as u128 - lp_fee - protocol_fee;

    market.lp_fees = market.lp_fees.checked_add(lp_fee as u64).ok_or(PredictionMarketError::MathOverflow)?;
    market.protocol_fees = market.protocol_fees.checked_add(protocol_fee as u64).ok_or(PredictionMarketError::MathOverflow)?;
    market.creator_fees = market.creator_fees.checked_add(creator_fee as u64).ok_or(PredictionMarketError::MathOverflow)?;
    market.total_fees_collected = market.total_fees_collected.checked_add(fee_amount).ok_or(PredictionMarketError::MathOverflow)?;

    // Update volume
    market.total_volume = market.total_volume
        .checked_add(amount_in)
        .ok_or(PredictionMarketError::MathOverflow)?;

    // Initialize position if new
    if position.owner == Pubkey::default() {
        position.market = market.key();
        position.owner = ctx.accounts.buyer.key();
        position.yes_shares = 0;
        position.no_shares = 0;
        position.avg_yes_entry_price = 0;
        position.avg_no_entry_price = 0;
        position.total_invested = 0;
        position.realized_pnl = 0;
        position.trade_count = 0;
        position.created_at = clock.unix_timestamp;
        position.last_trade_at = clock.unix_timestamp;
        position.claimed = false;
        position.payout_amount = 0;
        position.bump = ctx.bumps.position;
    }

    // Update position
    if is_yes {
        let old_shares = position.yes_shares;
        let old_price = position.avg_yes_entry_price;
        position.yes_shares = position.yes_shares
            .checked_add(shares_out)
            .ok_or(PredictionMarketError::MathOverflow)?;

        // Update average entry price
        if old_shares > 0 {
            position.avg_yes_entry_price =
                ((old_shares as u128 * old_price as u128 + amount_in as u128 * 10000) /
                    position.yes_shares as u128) as u64;
        } else {
            position.avg_yes_entry_price = (amount_in as u128 * 10000 / shares_out as u128) as u64;
        }
    } else {
        let old_shares = position.no_shares;
        let old_price = position.avg_no_entry_price;
        position.no_shares = position.no_shares
            .checked_add(shares_out)
            .ok_or(PredictionMarketError::MathOverflow)?;

        // Update average entry price
        if old_shares > 0 {
            position.avg_no_entry_price =
                ((old_shares as u128 * old_price as u128 + amount_in as u128 * 10000) /
                    position.no_shares as u128) as u64;
        } else {
            position.avg_no_entry_price = (amount_in as u128 * 10000 / shares_out as u128) as u64;
        }
    }

    position.total_invested = position.total_invested
        .checked_add(amount_in)
        .ok_or(PredictionMarketError::MathOverflow)?;
    position.trade_count = position.trade_count.checked_add(1).ok_or(PredictionMarketError::MathOverflow)?;
    position.last_trade_at = clock.unix_timestamp;

    // Emit events
    emit!(SharesPurchased {
        market: market.key(),
        buyer: ctx.accounts.buyer.key(),
        is_yes,
        amount_in,
        shares_out,
        price_impact_bps: price_impact,
        fee_paid: fee_amount,
        new_yes_pool: market.yes_pool,
        new_no_pool: market.no_pool,
    });

    emit!(PriceUpdate {
        market: market.key(),
        yes_pool: market.yes_pool,
        no_pool: market.no_pool,
        yes_price_bps: market.yes_price_bps(),
        no_price_bps: market.no_price_bps(),
        k: market.k_last,
    });

    Ok(())
}
