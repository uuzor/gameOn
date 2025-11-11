use anchor_lang::prelude::*;
use crate::state::character::CharacterClass;

#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq, Eq, InitSpace, Debug)]
pub enum Currency {
    SOL,
    SPL(Pubkey),
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, InitSpace, Debug)]
pub enum JoinStatus {
    Pending,
    Approved,
    Rejected,
    Withdrawn,
}

#[account]
#[derive(InitSpace)]
pub struct Offer {
    pub creator: Pubkey,
    pub offer_nonce: u64,
    pub currency: Currency,
    pub stake_amount: u64,
    pub min_level: u16,
    pub max_level: u16,
    #[max_len(5)]
    pub allowed_classes: Vec<CharacterClass>,
    pub auto_approve: bool,
    pub start_ts: i64,
    pub inactivity_timeout: i64,
    pub created_at: i64,
    pub is_active: bool,
    pub bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct Request {
    pub offer: Pubkey,
    pub challenger: Pubkey,
    pub character: Pubkey,
    pub offered_stake: u64,
    pub created_at: i64,
    pub status: JoinStatus,
    pub bump: u8,
}
