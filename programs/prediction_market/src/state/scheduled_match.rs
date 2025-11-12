use anchor_lang::prelude::*;

/// Status of a scheduled match
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub enum MatchStatus {
    /// Betting period is open
    Open,
    /// Betting ended, waiting for battle/game
    Locked,
    /// Game/battle completed
    Resolved,
    /// Match cancelled
    Cancelled,
}

/// Scheduled match that links to a prediction market
#[account]
#[derive(Debug)]
pub struct ScheduledMatch {
    /// Unique match identifier
    pub match_id: u64,

    /// Player/Team 1
    pub player1: Pubkey,

    /// Player/Team 2
    pub player2: Pubkey,

    // === Game Reference ===
    /// For BattleChain: battle offer PDA
    /// For external games: game_id from oracle
    pub game_reference: Pubkey,

    /// Is this an external game (via oracle)?
    pub is_external: bool,

    /// For external games: result_id in oracle
    pub external_result_id: Option<u64>,

    // === Market Reference ===
    /// Prediction market PDA for this match
    pub market: Pubkey,

    // === Timing ===
    /// When match was created/announced
    pub created_at: i64,

    /// When betting period starts
    pub betting_start: i64,

    /// When betting period ends
    pub betting_end: i64,

    /// Estimated battle/game start time
    pub estimated_battle_time: i64,

    /// Actual resolution time
    pub resolved_at: Option<i64>,

    // === Status ===
    /// Current match status
    pub status: MatchStatus,

    /// Winner after resolution
    pub winner: Option<Pubkey>,

    // === Match Info ===
    /// Match creator (receives fee)
    pub creator: Pubkey,

    /// Match title/description
    pub title: String, // Max 64 chars

    /// Additional metadata (JSON)
    pub metadata: String, // Max 256 chars

    /// PDA bump
    pub bump: u8,
}

impl ScheduledMatch {
    // Conservative size estimate
    pub const LEN: usize = 8 + // discriminator
        8 + // match_id
        32 + // player1
        32 + // player2
        32 + // game_reference
        1 + // is_external
        (1 + 8) + // external_result_id Option<u64>
        32 + // market
        8 + // created_at
        8 + // betting_start
        8 + // betting_end
        8 + // estimated_battle_time
        (1 + 8) + // resolved_at Option<i64>
        1 + // status enum
        (1 + 32) + // winner Option<Pubkey>
        32 + // creator
        (4 + 64) + // title String (length + max 64 chars)
        (4 + 256) + // metadata String (length + max 256 chars)
        1; // bump

    /// Check if betting period is active
    pub fn is_betting_open(&self, current_time: i64) -> bool {
        self.status == MatchStatus::Open
            && current_time >= self.betting_start
            && current_time < self.betting_end
    }

    /// Check if betting period has ended
    pub fn is_betting_ended(&self, current_time: i64) -> bool {
        current_time >= self.betting_end
    }

    /// Check if match can be resolved
    pub fn can_resolve(&self, current_time: i64) -> bool {
        self.status == MatchStatus::Locked && current_time >= self.betting_end
    }

    /// Check if match is resolved
    pub fn is_resolved(&self) -> bool {
        self.status == MatchStatus::Resolved
    }

    /// Get time until betting ends (in seconds)
    pub fn time_until_betting_ends(&self, current_time: i64) -> i64 {
        if current_time >= self.betting_end {
            0
        } else {
            self.betting_end - current_time
        }
    }

    /// Get time since match creation (in seconds)
    pub fn age(&self, current_time: i64) -> i64 {
        current_time - self.created_at
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_betting_status() {
        let scheduled_match = ScheduledMatch {
            match_id: 1,
            player1: Pubkey::default(),
            player2: Pubkey::default(),
            game_reference: Pubkey::default(),
            is_external: false,
            external_result_id: None,
            market: Pubkey::default(),
            created_at: 0,
            betting_start: 100,
            betting_end: 200,
            estimated_battle_time: 300,
            resolved_at: None,
            status: MatchStatus::Open,
            winner: None,
            creator: Pubkey::default(),
            title: String::from("Test Match"),
            metadata: String::from("{}"),
            bump: 0,
        };

        // Before betting starts
        assert!(!scheduled_match.is_betting_open(50));

        // During betting period
        assert!(scheduled_match.is_betting_open(150));

        // After betting ends
        assert!(!scheduled_match.is_betting_open(250));
        assert!(scheduled_match.is_betting_ended(250));
    }

    #[test]
    fn test_can_resolve() {
        let mut scheduled_match = ScheduledMatch {
            match_id: 1,
            player1: Pubkey::default(),
            player2: Pubkey::default(),
            game_reference: Pubkey::default(),
            is_external: false,
            external_result_id: None,
            market: Pubkey::default(),
            created_at: 0,
            betting_start: 100,
            betting_end: 200,
            estimated_battle_time: 300,
            resolved_at: None,
            status: MatchStatus::Open,
            winner: None,
            creator: Pubkey::default(),
            title: String::from("Test Match"),
            metadata: String::from("{}"),
            bump: 0,
        };

        // Cannot resolve while open
        assert!(!scheduled_match.can_resolve(250));

        // Can resolve after locked and betting ended
        scheduled_match.status = MatchStatus::Locked;
        assert!(scheduled_match.can_resolve(250));
    }

    #[test]
    fn test_time_calculations() {
        let scheduled_match = ScheduledMatch {
            match_id: 1,
            player1: Pubkey::default(),
            player2: Pubkey::default(),
            game_reference: Pubkey::default(),
            is_external: false,
            external_result_id: None,
            market: Pubkey::default(),
            created_at: 100,
            betting_start: 200,
            betting_end: 300,
            estimated_battle_time: 400,
            resolved_at: None,
            status: MatchStatus::Open,
            winner: None,
            creator: Pubkey::default(),
            title: String::from("Test Match"),
            metadata: String::from("{}"),
            bump: 0,
        };

        // Time until betting ends
        assert_eq!(scheduled_match.time_until_betting_ends(250), 50);
        assert_eq!(scheduled_match.time_until_betting_ends(350), 0);

        // Age
        assert_eq!(scheduled_match.age(150), 50);
    }
}
