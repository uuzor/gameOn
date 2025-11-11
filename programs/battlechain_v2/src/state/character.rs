use anchor_lang::prelude::*;

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, InitSpace, Debug)]
pub enum CharacterClass {
    Warrior,
    Assassin,
    Mage,
    Tank,
    Trickster,
}

#[account]
#[derive(InitSpace)]
pub struct Character {
    pub nft_mint: Pubkey,
    pub base_class: CharacterClass,
    pub max_hp: u32,
    pub current_hp: u32,
    pub base_damage_min: u16,
    pub base_damage_max: u16,
    pub crit_bps: u16,
    pub crit_multiplier_fp: u32,
    pub dodge_bps: u16,
    pub defense: u16,
    pub special_cooldown: u8,
    pub last_base_damage: u16,
    pub combo_count: u8,
    pub lifes: u8,
    // trait modifiers:
    pub mod_attack_bps: i16,
    pub mod_defense_bps: i16,
    pub mod_crit_bps: i16,
    pub rarity: u8,
    pub created_at: i64,
    pub bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct Progression {
    pub nft_mint: Pubkey,
    pub xp: u64,
    pub level: u16,
    pub mmr: u64,
    pub last_played: i64,
    pub bump: u8,
}

// Trait bundle
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Default)]
pub struct TraitBundle {
    pub rarity: u8,
    pub attack_bps: i16,
    pub defense_bps: i16,
    pub crit_bps: i16,
    pub nonce: i64,
}
