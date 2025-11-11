// Fixed-point & limits
pub const FP_SCALE: u128 = 1_000_000u128;
pub const DEFAULT_CRIT_FP: u128 = FP_SCALE * 2;
pub const MAX_TOTAL_MULTIPLIER_FP: u128 = FP_SCALE * 10;
pub const MAX_COMBO_STACK: u8 = 5;
pub const SEED_LEN: usize = 32;
pub const MAX_BATCHES: usize = 8;
pub const MIN_ENTROPY_PER_TURN: u64 = 1;

// Round system constants
pub const MAX_ROUNDS: u8 = 3;
pub const TURNS_PER_ROUND: u8 = 3;
pub const MAX_TOTAL_TURNS: u8 = MAX_ROUNDS * TURNS_PER_ROUND; // 9 turns max
