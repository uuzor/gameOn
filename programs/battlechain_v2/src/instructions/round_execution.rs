use anchor_lang::prelude::*;
// Import types from lib.rs (crate root) instead of state module to avoid conflicts
use crate::{
    ExecuteRound, EntropyPool, Battle, Character, Progression,
    BattleState, StanceType, CharacterClass,
};
use crate::constants::*;
use crate::errors::GameError;
use crate::events::*;
use crate::utils::*;
use anchor_lang::solana_program::sysvar::clock::Clock;
use anchor_spl::token::{Token, TokenAccount};

/// Player move for a single turn within a round
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy)]
pub struct TurnMove {
    pub preference: i8,      // Stance preference (-1 for no preference)
    pub use_special: bool,   // Use class special ability
    pub use_wildcard: bool,  // Use wildcard effect
}

/// Round moves - player submits 3 turns at once
#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct RoundMoves {
    pub moves: Vec<TurnMove>,  // Should be exactly TURNS_PER_ROUND (3)
}

/// Execute a full round (3 turns) for gas optimization
/// Players submit all 3 turn moves at once to reduce transaction overhead
pub fn execute_round_handler(
    ctx: Context<ExecuteRound>,
    round_moves: RoundMoves,
) -> Result<()> {
    let signer_key = ctx.accounts.signer.key();
    let battle_key = ctx.accounts.battle.key();

    // Validate battle state
    require!(
        ctx.accounts.battle.state == BattleState::Active,
        GameError::InvalidBattleState
    );

    // Validate round moves
    require!(
        round_moves.moves.len() == TURNS_PER_ROUND as usize,
        GameError::InvalidRange
    );

    // Check if max rounds exceeded
    require!(
        ctx.accounts.battle.current_round < MAX_ROUNDS,
        GameError::InvalidRange
    );

    // Determine which player
    let is_player1 = if signer_key == ctx.accounts.battle.player1 {
        true
    } else if signer_key == ctx.accounts.battle.player2 {
        false
    } else {
        return Err(error!(GameError::Unauthorized).into());
    };

    // Verify it's this player's turn
    if is_player1 {
        require!(ctx.accounts.battle.current_turn == 1, GameError::NotYourTurn);
    } else {
        require!(ctx.accounts.battle.current_turn == 2, GameError::NotYourTurn);
    }

    // Check entropy availability - need enough for full round
    // Each turn needs multiple entropy values, estimate 6 per turn
    let estimated_entropy_needed = (TURNS_PER_ROUND as u64) * 6;
    require!(
        ctx.accounts.pool.total_available >= estimated_entropy_needed,
        GameError::NoEntropyAvailable
    );

    let now = Clock::get()?.unix_timestamp;
    ctx.accounts.battle.last_action_ts = now;

    // Execute each turn in the round
    for (turn_idx, turn_move) in round_moves.moves.iter().enumerate() {
        // Check if battle already ended
        if ctx.accounts.battle.state == BattleState::Finished {
            break;
        }

        // Execute single turn
        execute_single_turn(
            &mut ctx.accounts.pool,
            &mut ctx.accounts.battle,
            &mut ctx.accounts.attacker_character,
            &mut ctx.accounts.defender_character,
            &mut ctx.accounts.attacker_prog,
            &mut ctx.accounts.defender_prog,
            &signer_key,
            &battle_key,
            is_player1,
            *turn_move,
            turn_idx as u8,
        )?;

        // If battle ended, break early
        if ctx.accounts.battle.state == BattleState::Finished {
            emit!(RoundCompleted {
                battle: battle_key,
                round_number: ctx.accounts.battle.current_round,
                turns_executed: (turn_idx + 1) as u8,
                battle_ended: true,
            });
            break;
        }

        // Increment turn counter
        ctx.accounts.battle.turns_in_current_round += 1;

        // Switch turn to other player
        ctx.accounts.battle.current_turn = if ctx.accounts.battle.current_turn == 1 { 2 } else { 1 };
    }

    // If battle not finished and round completed
    if ctx.accounts.battle.state != BattleState::Finished {
        // Check if round is complete
        if ctx.accounts.battle.turns_in_current_round >= TURNS_PER_ROUND {
            ctx.accounts.battle.rounds_completed += 1;
            ctx.accounts.battle.current_round += 1;
            ctx.accounts.battle.turns_in_current_round = 0;

            emit!(RoundCompleted {
                battle: battle_key,
                round_number: ctx.accounts.battle.rounds_completed,
                turns_executed: TURNS_PER_ROUND,
                battle_ended: false,
            });

            // Check if max rounds reached
            if ctx.accounts.battle.current_round >= MAX_ROUNDS {
                // Battle ends in a draw
                ctx.accounts.battle.state = BattleState::Finished;
                ctx.accounts.battle.winner = None;

                // Award tie XP
                ctx.accounts.attacker_prog.xp = ctx.accounts.attacker_prog.xp.saturating_add(25);
                ctx.accounts.defender_prog.xp = ctx.accounts.defender_prog.xp.saturating_add(25);

                emit!(BattleEnded {
                    battle: battle_key,
                    winner: None
                });
            }
        }
    }

    Ok(())
}

/// Execute a single turn within a round
/// This is a helper function to reduce code duplication
#[allow(clippy::too_many_arguments)]
fn execute_single_turn(
    pool: &mut Account<EntropyPool>,
    battle: &mut Account<Battle>,
    attacker_char: &mut Account<Character>,
    defender_char: &mut Account<Character>,
    attacker_prog: &mut Account<Progression>,
    defender_prog: &mut Account<Progression>,
    signer_key: &Pubkey,
    battle_key: &Pubkey,
    is_player1: bool,
    turn_move: TurnMove,
    _turn_in_round: u8,
) -> Result<()> {
    // Handle preference setting
    if turn_move.preference >= 0 {
        let p = turn_move.preference as u8;
        require!(p <= StanceType::Counter as u8, GameError::InvalidRange);
        if is_player1 {
            battle.player1_preference = p;
            battle.player1_pref_set = true;
        } else {
            battle.player2_preference = p;
            battle.player2_pref_set = true;
        }
    }

    // Consume entropy - optimized batch consumption
    let turn_num = battle.turn_number;
    let (seed_bytes, used_index) = pool.consume_seed_bytes_return_index(
        signer_key,
        b"turn",
        turn_num as u32
    )?;

    require!(
        used_index > battle.last_entropy_index,
        GameError::SeedReplay
    );
    battle.last_entropy_index = used_index;

    // Derive all random values from single seed
    let min_d = attacker_char.base_damage_min as u64;
    let max_d = attacker_char.base_damage_max as u64;
    let base_rand = derive_u64_from_seed_bytes(&seed_bytes, 0) % (max_d - min_d + 1) + min_d;
    let crit_roll = derive_u64_from_seed_bytes(&seed_bytes, 1) % 10000;
    let dodge_roll = derive_u64_from_seed_bytes(&seed_bytes, 2) % 10000;
    let wild_roll = derive_u64_from_seed_bytes(&seed_bytes, 3) % 10000;
    let att_stance_pick = derive_u64_from_seed_bytes(&seed_bytes, 4);
    let def_stance_pick = derive_u64_from_seed_bytes(&seed_bytes, 5);

    // Get player preferences
    let att_pref = if is_player1 {
        (battle.player1_pref_set, battle.player1_preference)
    } else {
        (battle.player2_pref_set, battle.player2_preference)
    };
    let def_pref = if is_player1 {
        (battle.player2_pref_set, battle.player2_preference)
    } else {
        (battle.player1_pref_set, battle.player1_preference)
    };

    // Sample stances
    let att_stance = sample_stance_with_preference(
        attacker_char.base_class,
        att_pref,
        att_stance_pick
    );
    let def_stance = sample_stance_with_preference(
        defender_char.base_class,
        def_pref,
        def_stance_pick
    );

    // Update battle stances
    if is_player1 {
        battle.player1_stance = att_stance;
        battle.player2_stance = def_stance;
    } else {
        battle.player2_stance = att_stance;
        battle.player1_stance = def_stance;
    }

    // Calculate base damage with level bonus
    let attacker_level = attacker_prog.level as u64;
    let base_u128 = (base_rand as u128)
        .checked_add((attacker_level.saturating_sub(1) * 2) as u128)
        .ok_or(GameError::MathOverflow)?;
    let mut damage_fp = base_u128
        .checked_mul(FP_SCALE)
        .ok_or(GameError::MathOverflow)?;

    // Apply critical hit
    let stored_crit_fp = attacker_char.crit_multiplier_fp as u128;
    let crit_mult_fp = if stored_crit_fp >= DEFAULT_CRIT_FP {
        stored_crit_fp
    } else {
        DEFAULT_CRIT_FP
    };

    let is_crit = (crit_roll as u64) < attacker_char.crit_bps as u64;
    if is_crit {
        damage_fp = mul_fp_checked(damage_fp, crit_mult_fp)?;
    }

    // Handle combo system
    let last_base = attacker_char.last_base_damage;
    let current_base = base_rand.min(u64::from(u16::MAX)) as u16;

    if last_base == current_base {
        attacker_char.combo_count = attacker_char.combo_count.saturating_add(1);
        if attacker_char.combo_count > MAX_COMBO_STACK {
            attacker_char.combo_count = MAX_COMBO_STACK;
        }
        let combo_mult_fp = FP_SCALE + (150_000u128 * (attacker_char.combo_count as u128));
        damage_fp = mul_fp_checked(damage_fp, combo_mult_fp)?;

        emit!(ComboApplied {
            battle: *battle_key,
            attacker: attacker_char.nft_mint,
            combo: attacker_char.combo_count,
            added: 0
        });
    } else {
        attacker_char.combo_count = 0;
    }
    attacker_char.last_base_damage = current_base;

    // Handle wildcard
    if turn_move.use_wildcard {
        if is_player1 {
            require!(
                !battle.player1_used_wildcard,
                GameError::InvalidRequestState
            );
            battle.player1_used_wildcard = true;
        } else {
            require!(
                !battle.player2_used_wildcard,
                GameError::InvalidRequestState
            );
            battle.player2_used_wildcard = true;
        }

        apply_wildcard_effect(
            wild_roll,
            &mut damage_fp,
            battle,
            attacker_char,
            defender_char,
        )?;

        emit!(SpecialUsed {
            battle: *battle_key,
            attacker: attacker_char.nft_mint,
            special: 255
        });
    }

    // Handle special abilities
    if turn_move.use_special {
        if is_player1 {
            require!(!battle.player1_special_used, GameError::InvalidRequestState);
            battle.player1_special_used = true;
        } else {
            require!(!battle.player2_special_used, GameError::InvalidRequestState);
            battle.player2_special_used = true;
        }

        require!(
            attacker_char.special_cooldown == 0,
            GameError::SpecialOnCooldown
        );

        match attacker_char.base_class {
            CharacterClass::Warrior => {
                damage_fp = mul_fp_checked(damage_fp, FP_SCALE * 3)?;
                attacker_char.special_cooldown = 3;
            }
            CharacterClass::Assassin => {
                damage_fp = mul_fp_checked(damage_fp, FP_SCALE * 3)?;
                attacker_char.special_cooldown = 4;
            }
            CharacterClass::Mage => {
                if is_player1 {
                    battle.player2_dot_damage = battle.player2_dot_damage.saturating_add(5);
                    battle.player2_dot_turns = battle.player2_dot_turns.saturating_add(3);
                } else {
                    battle.player1_dot_damage = battle.player1_dot_damage.saturating_add(5);
                    battle.player1_dot_turns = battle.player1_dot_turns.saturating_add(3);
                }
                attacker_char.special_cooldown = 3;
            }
            CharacterClass::Tank => {
                if is_player1 {
                    battle.player1_reflection = battle.player1_reflection.saturating_add(50);
                } else {
                    battle.player2_reflection = battle.player2_reflection.saturating_add(50);
                }
                attacker_char.special_cooldown = 4;
            }
            CharacterClass::Trickster => {
                damage_fp = mul_fp_checked(damage_fp, FP_SCALE * 2)?;
                attacker_char.special_cooldown = 2;
            }
        }

        emit!(SpecialUsed {
            battle: *battle_key,
            attacker: attacker_char.nft_mint,
            special: attacker_char.base_class as u8
        });
    }

    // Apply stance multipliers
    let (att_fp, def_fp, self_bps, counter_bps) = stance_multipliers(att_stance, def_stance);
    damage_fp = mul_fp_checked(damage_fp, att_fp)?;
    damage_fp = mul_fp_checked(damage_fp, def_fp)?;

    // Damage cap
    let max_allowed = base_u128
        .checked_mul(MAX_TOTAL_MULTIPLIER_FP)
        .ok_or(GameError::MathOverflow)?;
    if damage_fp > max_allowed {
        damage_fp = max_allowed;
        emit!(DamageClamped {
            battle: *battle_key,
            attacker: attacker_char.nft_mint
        });
    }

    // Convert to u64
    let mut final_damage = fp_to_u64_clamped(damage_fp, GameError::MathOverflow)?;

    // Apply defense
    if defender_char.defense as u64 >= final_damage {
        final_damage = 0;
    } else {
        final_damage = final_damage.saturating_sub(defender_char.defense as u64);
    }

    // Check dodge
    let is_dodge = (dodge_roll as u64) < defender_char.dodge_bps as u64;
    if is_dodge {
        final_damage = 0;
        if is_player1 {
            battle.player1_miss_count = battle.player1_miss_count.saturating_add(1);
        } else {
            battle.player2_miss_count = battle.player2_miss_count.saturating_add(1);
        }
        emit!(AttackMissed {
            battle: *battle_key,
            attacker: attacker_char.nft_mint,
            defender: defender_char.nft_mint
        });
    }

    // Apply damage and effects
    if is_player1 {
        battle.player2_health = battle.player2_health.saturating_sub(final_damage);

        // Reflection
        if battle.player1_reflection > 0 && final_damage > 0 {
            let reflected = final_damage.saturating_mul(battle.player1_reflection as u64) / 100;
            battle.player1_health = battle.player1_health.saturating_sub(reflected);
        }

        // Counter
        if counter_bps > 0 && final_damage > 0 {
            let counter = final_damage.saturating_mul(counter_bps as u64) / 10000u64;
            battle.player1_health = battle.player1_health.saturating_sub(counter);
        }

        // Self-damage
        if self_bps > 0 {
            let self_dmg = final_damage.saturating_mul(self_bps as u64) / 10000u64;
            battle.player1_health = battle.player1_health.saturating_sub(self_dmg);
        }
    } else {
        battle.player1_health = battle.player1_health.saturating_sub(final_damage);

        // Reflection
        if battle.player2_reflection > 0 && final_damage > 0 {
            let reflected = final_damage.saturating_mul(battle.player2_reflection as u64) / 100;
            battle.player2_health = battle.player2_health.saturating_sub(reflected);
        }

        // Counter
        if counter_bps > 0 && final_damage > 0 {
            let counter = final_damage.saturating_mul(counter_bps as u64) / 10000u64;
            battle.player2_health = battle.player2_health.saturating_sub(counter);
        }

        // Self-damage
        if self_bps > 0 {
            let self_dmg = final_damage.saturating_mul(self_bps as u64) / 10000u64;
            battle.player2_health = battle.player2_health.saturating_sub(self_dmg);
        }
    }

    // Decrement cooldowns
    if attacker_char.special_cooldown > 0 {
        attacker_char.special_cooldown = attacker_char.special_cooldown.saturating_sub(1);
    }

    // Apply DoT
    if battle.player1_dot_turns > 0 {
        let dot = battle.player1_dot_damage;
        if dot > 0 {
            battle.player1_health = battle.player1_health.saturating_sub(dot);
        }
        battle.player1_dot_turns = battle.player1_dot_turns.saturating_sub(1);
    }
    if battle.player2_dot_turns > 0 {
        let dot = battle.player2_dot_damage;
        if dot > 0 {
            battle.player2_health = battle.player2_health.saturating_sub(dot);
        }
        battle.player2_dot_turns = battle.player2_dot_turns.saturating_sub(1);
    }

    // Check for battle end
    let p1_health = battle.player1_health;
    let p2_health = battle.player2_health;

    if p1_health == 0 || p2_health == 0 {
        battle.state = BattleState::Finished;
        let winner_opt = if p1_health > p2_health {
            Some(battle.player1)
        } else if p2_health > p1_health {
            Some(battle.player2)
        } else {
            None
        };
        battle.winner = winner_opt;

        // Award XP
        if let Some(wpk) = winner_opt {
            if wpk == battle.player1 {
                attacker_prog.xp = attacker_prog.xp.saturating_add(100);
                level_up_if_needed(attacker_prog, attacker_char)?;
            } else {
                defender_prog.xp = defender_prog.xp.saturating_add(100);
                level_up_if_needed(defender_prog, defender_char)?;
            }
        } else {
            attacker_prog.xp = attacker_prog.xp.saturating_add(25);
            defender_prog.xp = defender_prog.xp.saturating_add(25);
        }

        emit!(BattleEnded {
            battle: *battle_key,
            winner: winner_opt
        });
    }

    // Emit turn event
    emit!(TurnResolved {
        battle: *battle_key,
        turn_number: battle.turn_number,
        attacker: attacker_char.nft_mint,
        defender: defender_char.nft_mint,
        used_entropy_index: used_index,
        att_stance: att_stance as u8,
        def_stance: def_stance as u8,
        base_roll: base_rand,
        is_crit,
        crit_roll,
        dodge_roll,
        is_dodge,
        wild_roll,
        used_wild: turn_move.use_wildcard,
        wildcard_result: 0,
        damage_dealt: final_damage,
        post_health_p1: battle.player1_health,
        post_health_p2: battle.player2_health,
        combo_count: attacker_char.combo_count,
        special_used: turn_move.use_special,
    });

    // Increment turn number
    battle.turn_number = battle.turn_number.saturating_add(1);

    Ok(())
}
