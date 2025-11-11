use anchor_lang::prelude::*;

#[event]
pub struct ConfigCreated {
    pub config: Pubkey,
    pub admin: Pubkey,
}

#[event]
pub struct EntropyPoolCreated {
    pub pool: Pubkey,
    pub vrf_oracle: Pubkey,
}

#[event]
pub struct SeedBatchRefilled {
    pub pool: Pubkey,
    pub added: u64,
    pub total_available: u64,
}

#[event]
pub struct ProgressionCreated {
    pub nft_mint: Pubkey,
}

#[event]
pub struct CharacterCreated {
    pub nft_mint: Pubkey,
    pub owner: Pubkey,
}

#[event]
pub struct TraitApplied {
    pub nft_mint: Pubkey,
    pub by: Pubkey,
}

#[event]
pub struct OfferCreated {
    pub offer: Pubkey,
    pub creator: Pubkey,
    pub stake: u64,
}

#[event]
pub struct JoinRequested {
    pub offer: Pubkey,
    pub request: Pubkey,
    pub challenger: Pubkey,
    pub stake: u64,
}

#[event]
pub struct RequestWithdrawn {
    pub request: Pubkey,
    pub by: Pubkey,
}

#[event]
pub struct OfferCancelled {
    pub offer: Pubkey,
    pub by: Pubkey,
}

#[event]
pub struct BattleCreated {
    pub battle: Pubkey,
    pub player1: Pubkey,
    pub player2: Pubkey,
    pub first_turn: u8,
    pub stake_total: u64,
}

#[event]
pub struct BattleForfeited {
    pub battle: Pubkey,
    pub winner: Pubkey,
}

#[event]
pub struct BattleEnded {
    pub battle: Pubkey,
    pub winner: Option<Pubkey>,
}

#[event]
pub struct DamageClamped {
    pub battle: Pubkey,
    pub attacker: Pubkey,
}

#[event]
pub struct ComboApplied {
    pub battle: Pubkey,
    pub attacker: Pubkey,
    pub combo: u8,
    pub added: u64,
}

#[event]
pub struct SpecialUsed {
    pub battle: Pubkey,
    pub attacker: Pubkey,
    pub special: u8,
}

#[event]
pub struct AttackMissed {
    pub battle: Pubkey,
    pub attacker: Pubkey,
    pub defender: Pubkey,
}

#[event]
pub struct ReflectionApplied {
    pub battle: Pubkey,
    pub defender: Pubkey,
    pub reflected: u64,
}

#[event]
pub struct CounterApplied {
    pub battle: Pubkey,
    pub player: Pubkey,
    pub damage: u64,
}

#[event]
pub struct SelfDamageApplied {
    pub battle: Pubkey,
    pub player: Pubkey,
    pub damage: u64,
}

#[event]
pub struct LifeConsumed {
    pub character: Pubkey,
    pub remaining: u8,
}

#[event]
pub struct TurnResolved {
    pub battle: Pubkey,
    pub turn_number: u64,
    pub attacker: Pubkey,
    pub defender: Pubkey,
    pub used_entropy_index: u64,
    pub att_stance: u8,
    pub def_stance: u8,
    pub base_roll: u64,
    pub is_crit: bool,
    pub crit_roll: u64,
    pub dodge_roll: u64,
    pub is_dodge: bool,
    pub wild_roll: u64,
    pub used_wild: bool,
    pub wildcard_result: u8,
    pub damage_dealt: u64,
    pub post_health_p1: u64,
    pub post_health_p2: u64,
    pub combo_count: u8,
    pub special_used: bool,
}

#[event]
pub struct BattleSettled {
    pub battle: Pubkey,
    pub total_paid: u64,
}

#[event]
pub struct ProgressionLevelUp {
    pub nft_mint: Pubkey,
    pub new_level: u16,
}

#[event]
pub struct RoundCompleted {
    pub battle: Pubkey,
    pub round_number: u8,
    pub turns_executed: u8,
    pub battle_ended: bool,
}
