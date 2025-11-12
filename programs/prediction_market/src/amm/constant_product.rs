use anchor_lang::prelude::*;
use crate::constants::*;
use crate::errors::PredictionMarketError;

/// AMM calculations using constant product formula: x * y = k
///
/// In our case:
/// - x = YES pool reserves
/// - y = NO pool reserves
/// - k = constant product (x * y)
///
/// When buying YES shares:
/// - User pays amount_in to YES pool
/// - YES pool increases: x' = x + amount_in
/// - To maintain k, NO pool decreases: y' = k / x'
/// - User receives: y - y' NO shares (burned)
/// - Effectively, user gets YES exposure

/// Calculate shares output for a given input amount
/// Returns (shares_out, price_impact_bps, fee_amount)
pub fn calculate_shares_out(
    amount_in: u64,
    reserve_in: u64,
    reserve_out: u64,
    fee_bps: u16,
) -> Result<(u64, u16, u64)> {
    require!(amount_in > 0, PredictionMarketError::AmountBelowMinimum);
    require!(reserve_in > 0 && reserve_out > 0, PredictionMarketError::InsufficientLiquidity);

    // Calculate fee
    let fee_amount = ((amount_in as u128 * fee_bps as u128) / 10000) as u64;
    let amount_in_after_fee = amount_in
        .checked_sub(fee_amount)
        .ok_or(PredictionMarketError::MathOverflow)?;

    // Calculate output using constant product formula
    // k = reserve_in * reserve_out
    // New reserve_in = reserve_in + amount_in_after_fee
    // New reserve_out = k / new_reserve_in
    // shares_out = reserve_out - new_reserve_out

    let k = (reserve_in as u128)
        .checked_mul(reserve_out as u128)
        .ok_or(PredictionMarketError::MathOverflow)?;

    let new_reserve_in = (reserve_in as u128)
        .checked_add(amount_in_after_fee as u128)
        .ok_or(PredictionMarketError::MathOverflow)?;

    let new_reserve_out = k
        .checked_div(new_reserve_in)
        .ok_or(PredictionMarketError::DivisionByZero)?;

    let shares_out = (reserve_out as u128)
        .checked_sub(new_reserve_out)
        .ok_or(PredictionMarketError::MathOverflow)?;

    // Check for overflow when converting to u64
    require!(
        shares_out <= u64::MAX as u128,
        PredictionMarketError::MathOverflow
    );

    // Calculate price impact in basis points
    // Price impact = (shares_out / amount_in) / (reserve_out / reserve_in) - 1
    // Simplified: price_impact = (shares_out * reserve_in) / (amount_in * reserve_out) - 1
    let expected_shares = ((amount_in_after_fee as u128 * reserve_out as u128) / reserve_in as u128);
    let price_impact = if expected_shares > 0 {
        let ratio = ((shares_out * 10000) / expected_shares) as i64;
        let impact = 10000 - ratio;
        impact.max(0).min(10000) as u16
    } else {
        0
    };

    Ok((shares_out as u64, price_impact, fee_amount))
}

/// Calculate amount out for selling shares
/// Returns (amount_out, fee_amount)
pub fn calculate_amount_out(
    shares_in: u64,
    reserve_in: u64,
    reserve_out: u64,
    fee_bps: u16,
) -> Result<(u64, u64)> {
    require!(shares_in > 0, PredictionMarketError::AmountBelowMinimum);
    require!(reserve_in > 0 && reserve_out > 0, PredictionMarketError::InsufficientLiquidity);

    // Calculate output using constant product formula
    // k = reserve_in * reserve_out
    // New reserve_in = reserve_in - shares_in
    // New reserve_out = k / new_reserve_in
    // amount_out = new_reserve_out - reserve_out

    let k = (reserve_in as u128)
        .checked_mul(reserve_out as u128)
        .ok_or(PredictionMarketError::MathOverflow)?;

    let new_reserve_in = (reserve_in as u128)
        .checked_sub(shares_in as u128)
        .ok_or(PredictionMarketError::InsufficientLiquidity)?;

    let new_reserve_out = k
        .checked_div(new_reserve_in)
        .ok_or(PredictionMarketError::DivisionByZero)?;

    let amount_out = new_reserve_out
        .checked_sub(reserve_out as u128)
        .ok_or(PredictionMarketError::MathOverflow)?;

    require!(
        amount_out <= u64::MAX as u128,
        PredictionMarketError::MathOverflow
    );

    // Apply fee
    let amount_out_u64 = amount_out as u64;
    let fee_amount = ((amount_out_u64 as u128 * fee_bps as u128) / 10000) as u64;
    let final_amount_out = amount_out_u64
        .checked_sub(fee_amount)
        .ok_or(PredictionMarketError::MathOverflow)?;

    Ok((final_amount_out, fee_amount))
}

/// Calculate LP shares to mint when adding liquidity
/// Returns (lp_shares, yes_amount_actual, no_amount_actual)
pub fn calculate_lp_shares(
    yes_amount: u64,
    no_amount: u64,
    yes_reserve: u64,
    no_reserve: u64,
    total_supply: u64,
) -> Result<(u64, u64, u64)> {
    require!(yes_amount > 0 && no_amount > 0, PredictionMarketError::AmountBelowMinimum);

    if total_supply == 0 {
        // Initial liquidity
        // LP shares = sqrt(yes * no) using integer approximation
        let product = (yes_amount as u128)
            .checked_mul(no_amount as u128)
            .ok_or(PredictionMarketError::MathOverflow)?;

        let lp_shares = integer_sqrt(product);

        require!(lp_shares > 0, PredictionMarketError::InsufficientLiquidity);
        require!(lp_shares <= u64::MAX as u128, PredictionMarketError::MathOverflow);

        Ok((lp_shares as u64, yes_amount, no_amount))
    } else {
        // Subsequent liquidity
        // Maintain pool ratio: yes_amount / yes_reserve = no_amount / no_reserve
        // LP shares = (yes_amount / yes_reserve) * total_supply

        require!(yes_reserve > 0 && no_reserve > 0, PredictionMarketError::InsufficientLiquidity);

        let yes_share = ((yes_amount as u128 * total_supply as u128) / yes_reserve as u128) as u64;
        let no_share = ((no_amount as u128 * total_supply as u128) / no_reserve as u128) as u64;

        // Take the minimum to maintain ratio
        let lp_shares = yes_share.min(no_share);

        // Calculate actual amounts needed (might be less than provided)
        let yes_amount_actual = ((lp_shares as u128 * yes_reserve as u128) / total_supply as u128) as u64;
        let no_amount_actual = ((lp_shares as u128 * no_reserve as u128) / total_supply as u128) as u64;

        require!(lp_shares > 0, PredictionMarketError::LiquidityTooLow);

        Ok((lp_shares, yes_amount_actual, no_amount_actual))
    }
}

/// Calculate amounts to return when burning LP shares
/// Returns (yes_amount, no_amount, fees_earned)
pub fn calculate_lp_withdrawal(
    lp_shares: u64,
    yes_reserve: u64,
    no_reserve: u64,
    total_supply: u64,
    lp_fees_accumulated: u64,
) -> Result<(u64, u64, u64)> {
    require!(lp_shares > 0, PredictionMarketError::InvalidLPShares);
    require!(total_supply > 0, PredictionMarketError::DivisionByZero);
    require!(lp_shares <= total_supply, PredictionMarketError::InvalidLPShares);

    // Proportional share of reserves
    let yes_amount = ((lp_shares as u128 * yes_reserve as u128) / total_supply as u128) as u64;
    let no_amount = ((lp_shares as u128 * no_reserve as u128) / total_supply as u128) as u64;

    // Proportional share of accumulated fees
    let fees_earned = ((lp_shares as u128 * lp_fees_accumulated as u128) / total_supply as u128) as u64;

    Ok((yes_amount, no_amount, fees_earned))
}

/// Verify slippage tolerance
pub fn check_slippage(
    expected: u64,
    actual: u64,
    max_slippage_bps: u16,
) -> Result<()> {
    require!(
        max_slippage_bps <= MAX_SLIPPAGE_BPS,
        PredictionMarketError::InvalidSlippage
    );

    if expected == 0 {
        return Ok(());
    }

    let slippage_bps = if actual < expected {
        ((expected - actual) as u128 * 10000 / expected as u128) as u16
    } else {
        0
    };

    require!(
        slippage_bps <= max_slippage_bps,
        PredictionMarketError::SlippageExceeded
    );

    Ok(())
}

/// Integer square root using Newton's method
fn integer_sqrt(n: u128) -> u128 {
    if n == 0 {
        return 0;
    }

    // Initial guess
    let mut x = n;
    let mut y = (x + 1) / 2;

    // Newton's method
    while y < x {
        x = y;
        y = (x + n / x) / 2;
    }

    x
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_shares_out() {
        // Initial pool: 100 YES, 100 NO (50/50 odds)
        // User buys 10 YES (pays into YES pool)
        // Fee: 40 bps = 0.4%

        let (shares_out, price_impact, fee) = calculate_shares_out(
            10,   // amount_in
            100,  // reserve_in (YES)
            100,  // reserve_out (NO)
            40,   // fee_bps
        ).unwrap();

        // Fee = 10 * 0.004 = 0.04 ≈ 0
        assert_eq!(fee, 0); // Rounds to 0 for small amounts

        // After fee: 10 (fee rounds to 0 for testing)
        // New YES reserve: 110
        // k = 10000
        // New NO reserve: 10000 / 110 = 90.909...
        // Shares out: 100 - 90.909 = 9.09 ≈ 9
        assert_eq!(shares_out, 9);

        // Price impact should be small for this trade
        assert!(price_impact < 1000); // Less than 10%
    }

    #[test]
    fn test_calculate_amount_out() {
        // Sell 9 shares back
        let (amount_out, fee) = calculate_amount_out(
            9,    // shares_in
            90,   // reserve_in (NO after previous trade)
            110,  // reserve_out (YES after previous trade)
            40,   // fee_bps
        ).unwrap();

        // Should get close to 10 back (minus fees)
        assert!(amount_out >= 9 && amount_out <= 10);
    }

    #[test]
    fn test_calculate_lp_shares_initial() {
        // Initial liquidity: 100 YES, 100 NO
        let (lp_shares, yes_actual, no_actual) = calculate_lp_shares(
            100,  // yes_amount
            100,  // no_amount
            0,    // yes_reserve
            0,    // no_reserve
            0,    // total_supply
        ).unwrap();

        // LP shares = sqrt(100 * 100) = 100
        assert_eq!(lp_shares, 100);
        assert_eq!(yes_actual, 100);
        assert_eq!(no_actual, 100);
    }

    #[test]
    fn test_calculate_lp_shares_subsequent() {
        // Add more liquidity to existing pool
        // Pool: 100 YES, 100 NO, 100 LP shares
        // Add: 50 YES, 50 NO
        let (lp_shares, yes_actual, no_actual) = calculate_lp_shares(
            50,   // yes_amount
            50,   // no_amount
            100,  // yes_reserve
            100,  // no_reserve
            100,  // total_supply
        ).unwrap();

        // LP shares = (50/100) * 100 = 50
        assert_eq!(lp_shares, 50);
        assert_eq!(yes_actual, 50);
        assert_eq!(no_actual, 50);
    }

    #[test]
    fn test_calculate_lp_withdrawal() {
        // Withdraw 50 LP shares from pool with 150 YES, 150 NO, 150 total LP
        let (yes_amount, no_amount, fees) = calculate_lp_withdrawal(
            50,   // lp_shares
            150,  // yes_reserve
            150,  // no_reserve
            150,  // total_supply
            30,   // lp_fees_accumulated
        ).unwrap();

        // Should get 1/3 of pool
        assert_eq!(yes_amount, 50);
        assert_eq!(no_amount, 50);
        assert_eq!(fees, 10); // 1/3 of 30
    }

    #[test]
    fn test_slippage_check() {
        // Expected 100, got 95, 5% slippage
        // Max allowed: 10%
        let result = check_slippage(100, 95, 1000);
        assert!(result.is_ok());

        // Expected 100, got 85, 15% slippage
        // Max allowed: 10%
        let result = check_slippage(100, 85, 1000);
        assert!(result.is_err());
    }

    #[test]
    fn test_integer_sqrt() {
        assert_eq!(integer_sqrt(0), 0);
        assert_eq!(integer_sqrt(1), 1);
        assert_eq!(integer_sqrt(4), 2);
        assert_eq!(integer_sqrt(9), 3);
        assert_eq!(integer_sqrt(16), 4);
        assert_eq!(integer_sqrt(100), 10);
        assert_eq!(integer_sqrt(10000), 100);
    }

    #[test]
    fn test_price_impact_large_trade() {
        // Large trade should have high price impact
        let (shares_out, price_impact, _) = calculate_shares_out(
            50,   // amount_in (50% of pool!)
            100,  // reserve_in
            100,  // reserve_out
            40,   // fee_bps
        ).unwrap();

        // Should get less than 50 shares due to slippage
        assert!(shares_out < 50);

        // Price impact should be significant
        assert!(price_impact > 2000); // More than 20%
    }
}
