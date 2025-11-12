use anchor_lang::prelude::*;

/// Fixed-point scale for AMM calculations
/// All prices and ratios use this scale to avoid floating point
/// Example: 0.5 = 500_000 (500,000 / 1,000,000)
pub const FP_SCALE: u128 = 1_000_000;

/// Trading fee in basis points (0.40% total)
/// Breakdown: 0.30% to LPs, 0.05% to protocol, 0.05% to match creator
pub const TRADING_FEE_BPS: u16 = 40;

/// LP fee share (75% of total fee)
pub const LP_FEE_BPS: u16 = 30;

/// Protocol fee share (12.5% of total fee)
pub const PROTOCOL_FEE_BPS: u16 = 5;

/// Match creator fee share (12.5% of total fee)
pub const CREATOR_FEE_BPS: u16 = 5;

/// Minimum initial liquidity to prevent division by zero
/// Set to 1 SOL equivalent to ensure meaningful markets
pub const MIN_INITIAL_LIQUIDITY: u64 = 1_000_000_000; // 1 SOL in lamports

/// Minimum trade amount (0.01 SOL equivalent)
/// Prevents dust trades and reduces spam
pub const MIN_TRADE_AMOUNT: u64 = 10_000_000; // 0.01 SOL

/// Maximum slippage in basis points (10% = 1000 bps)
/// Protects users from excessive price impact
pub const MAX_SLIPPAGE_BPS: u16 = 1000;

/// Default betting period (1 hour before battle)
pub const DEFAULT_BETTING_PERIOD_SECONDS: i64 = 3600;

/// Minimum betting period (5 minutes)
pub const MIN_BETTING_PERIOD_SECONDS: i64 = 300;

/// Maximum betting period (7 days)
pub const MAX_BETTING_PERIOD_SECONDS: i64 = 604800;

/// Program authority seeds
pub const AUTHORITY_SEED: &[u8] = b"authority";
pub const MARKET_SEED: &[u8] = b"market";
pub const POSITION_SEED: &[u8] = b"position";
pub const LIQUIDITY_SEED: &[u8] = b"liquidity";
pub const VAULT_SEED: &[u8] = b"vault";
pub const SCHEDULED_MATCH_SEED: &[u8] = b"scheduled_match";

/// Supported token mints (devnet addresses)
/// Mainnet addresses will be different
pub mod token_mints {
    use anchor_lang::prelude::*;

    /// Native SOL (no mint needed)
    pub const SOL: Option<Pubkey> = None;

    /// USDC on devnet
    /// Mainnet: EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v
    pub fn usdc_devnet() -> Pubkey {
        // Devnet USDC mint (will update when deploying)
        Pubkey::default()
    }

    /// USDT on devnet
    /// Mainnet: Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB
    pub fn usdt_devnet() -> Pubkey {
        // Devnet USDT mint (will update when deploying)
        Pubkey::default()
    }
}

/// Token decimals
pub const SOL_DECIMALS: u8 = 9;
pub const USDC_DECIMALS: u8 = 6;
pub const USDT_DECIMALS: u8 = 6;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fee_distribution() {
        // Ensure fees add up correctly
        assert_eq!(
            LP_FEE_BPS + PROTOCOL_FEE_BPS + CREATOR_FEE_BPS,
            TRADING_FEE_BPS
        );
    }

    #[test]
    fn test_fp_scale() {
        // Test basic fixed point arithmetic
        let half: u128 = FP_SCALE / 2;
        assert_eq!(half, 500_000);

        let quarter: u128 = FP_SCALE / 4;
        assert_eq!(quarter, 250_000);
    }

    #[test]
    fn test_min_amounts() {
        // Ensure minimum amounts are reasonable
        assert!(MIN_TRADE_AMOUNT > 0);
        assert!(MIN_INITIAL_LIQUIDITY >= MIN_TRADE_AMOUNT);
    }

    #[test]
    fn test_betting_period_bounds() {
        // Ensure timing constraints make sense
        assert!(MIN_BETTING_PERIOD_SECONDS < DEFAULT_BETTING_PERIOD_SECONDS);
        assert!(DEFAULT_BETTING_PERIOD_SECONDS < MAX_BETTING_PERIOD_SECONDS);
    }
}
