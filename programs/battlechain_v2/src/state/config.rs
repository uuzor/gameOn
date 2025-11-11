use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct Config {
    pub admin: Pubkey,
    pub fee_bps: u16,
    pub inactivity_timeout: i64,
    #[max_len(8)]
    pub spl_whitelist: Vec<Pubkey>,
    pub trait_authority: Pubkey,
    pub bump: u8,
}
