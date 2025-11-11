use anchor_lang::prelude::*;

#[error_code]
pub enum GameError {
    #[msg("Unauthorized refill")]
    UnauthorizedRefill,
    #[msg("Seed replay")]
    SeedReplay,
    #[msg("Entropy pool full")]
    EntropyPoolFull,
    #[msg("No entropy available")]
    NoEntropyAvailable,
    #[msg("Invalid index")]
    InvalidIndex,
    #[msg("Invalid range")]
    InvalidRange,
    #[msg("Math overflow")]
    MathOverflow,
    #[msg("Invalid NFT token account")]
    InvalidNftAta,
    #[msg("Not NFT owner")]
    NotNftOwner,
    #[msg("Offer not active")]
    OfferNotActive,
    #[msg("Character fails constraints")]
    CharacterConstraint,
    #[msg("Unauthorized")]
    Unauthorized,
    #[msg("Invalid request state")]
    InvalidRequestState,
    #[msg("Invalid battle state")]
    InvalidBattleState,
    #[msg("Battle already finished")]
    BattleAlreadyFinished,
    #[msg("Not your turn")]
    NotYourTurn,
    #[msg("Special on cooldown")]
    SpecialOnCooldown,
    #[msg("Invalid timestamp")]
    InvalidTimestamp,
    #[msg("Battle not finished")]
    BattleNotFinished,
    #[msg("Auto-approve disabled")]
    AutoApproveDisabled,
    #[msg("SPL not whitelisted")]
    SPLNotWhitelisted,
    #[msg("Timeout not reached")]
    TimeoutNotReached,
}
