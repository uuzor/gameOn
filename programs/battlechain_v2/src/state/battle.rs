use anchor_lang::prelude::*;

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, InitSpace, Debug)]
pub enum BattleState {
    Waiting,
    Active,
    Finished,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, InitSpace, Debug)]
pub enum StanceType {
    Balanced,
    Aggressive,
    Defensive,
    Berserker,
    Counter,
}

#[account]
#[derive(InitSpace)]
pub struct Battle {
    pub battle_id: u64,
    pub player1: Pubkey,
    pub player2: Pubkey,
    pub start_ts: i64,
    pub current_turn: u8,
    pub turn_number: u64,
    // Round tracking
    pub current_round: u8,
    pub rounds_completed: u8,
    pub turns_in_current_round: u8,
    pub player1_health: u64,
    pub player2_health: u64,
    pub state: BattleState,
    pub player1_stance: StanceType,
    pub player2_stance: StanceType,
    pub created_at: i64,
    pub inactivity_timeout: i64,
    pub last_action_ts: i64,
    pub winner: Option<Pubkey>,
    pub player1_dot_damage: u64,
    pub player2_dot_damage: u64,
    pub player1_dot_turns: u8,
    pub player2_dot_turns: u8,
    pub player1_reflection: u16,
    pub player2_reflection: u16,
    pub player1_miss_count: u16,
    pub player2_miss_count: u16,
    pub last_entropy_index: u64,
    pub bump: u8,
    pub player1_used_wildcard: bool,
    pub player2_used_wildcard: bool,
    pub player1_special_used: bool,
    pub player2_special_used: bool,
    pub player1_preference: u8,
    pub player2_preference: u8,
    pub player1_pref_set: bool,
    pub player2_pref_set: bool,
}
