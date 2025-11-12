use anchor_lang::prelude::*;

#[error_code]
pub enum PredictionMarketError {
    #[msg("Math overflow occurred")]
    MathOverflow,

    #[msg("Division by zero")]
    DivisionByZero,

    #[msg("Invalid market status for this operation")]
    InvalidMarketStatus,

    #[msg("Market is still open for trading")]
    MarketStillOpen,

    #[msg("Market is locked, no trading allowed")]
    MarketLocked,

    #[msg("Market has already been resolved")]
    MarketAlreadyResolved,

    #[msg("Market has not been resolved yet")]
    MarketNotResolved,

    #[msg("Battle has not finished yet")]
    BattleNotFinished,

    #[msg("Insufficient liquidity in pool")]
    InsufficientLiquidity,

    #[msg("Amount below minimum trade amount")]
    AmountBelowMinimum,

    #[msg("Amount exceeds maximum allowed")]
    AmountTooLarge,

    #[msg("Slippage tolerance exceeded")]
    SlippageExceeded,

    #[msg("Invalid slippage tolerance (max 10%)")]
    InvalidSlippage,

    #[msg("Deadline has passed")]
    DeadlinePassed,

    #[msg("Invalid odds ratio")]
    InvalidOdds,

    #[msg("Invalid currency specified")]
    InvalidCurrency,

    #[msg("Currency mismatch between market and trade")]
    CurrencyMismatch,

    #[msg("Token mint mismatch")]
    TokenMintMismatch,

    #[msg("Insufficient balance")]
    InsufficientBalance,

    #[msg("No shares to claim")]
    NoSharesToClaim,

    #[msg("Position already claimed")]
    AlreadyClaimed,

    #[msg("Invalid outcome")]
    InvalidOutcome,

    #[msg("You did not win this market")]
    NotAWinner,

    #[msg("Invalid betting period duration")]
    InvalidBettingPeriod,

    #[msg("Betting period has not started")]
    BettingNotStarted,

    #[msg("Betting period has ended")]
    BettingEnded,

    #[msg("Cannot resolve before betting ends")]
    TooEarlyToResolve,

    #[msg("Invalid scheduled match reference")]
    InvalidScheduledMatch,

    #[msg("Battle has not been created yet")]
    BattleNotCreated,

    #[msg("Unauthorized access")]
    Unauthorized,

    #[msg("Invalid initial pool ratio")]
    InvalidInitialRatio,

    #[msg("Liquidity too low to remove")]
    LiquidityTooLow,

    #[msg("No liquidity position found")]
    NoLiquidityPosition,

    #[msg("LP shares amount invalid")]
    InvalidLPShares,

    #[msg("Cannot remove liquidity while market is active")]
    MarketStillActive,

    #[msg("Invalid fixed-point calculation")]
    InvalidFixedPoint,

    #[msg("Price impact too high")]
    PriceImpactTooHigh,

    #[msg("Constant product invariant violation")]
    InvariantViolation,

    #[msg("Invalid fee parameters")]
    InvalidFeeParams,

    #[msg("Token transfer failed")]
    TokenTransferFailed,

    #[msg("Invalid vault account")]
    InvalidVault,

    #[msg("Account not found or invalid")]
    InvalidAccount,

    #[msg("Numerical overflow in conversion")]
    ConversionOverflow,
}
