use anchor_lang::prelude::*;
use crate::constants::FP_SCALE;
use crate::errors::GameError;
use crate::state::{Battle, Character};
use crate::utils::math::mul_fp_checked;

pub fn apply_wildcard_effect(
    wild_roll: u64,
    damage_fp: &mut u128,
    battle: &mut Account<Battle>,
    att: &mut Account<Character>,
    _def: &mut Account<Character>,
) -> Result<u8> {
    // Table:
    // 0..3999 => +25% damage (40%)
    // 4000..6999 => heal attacker 10 hp (30%)
    // 7000..8499 => stun opponent next turn (15%) => represented as light dot
    // 8500..9499 => nothing (10%)
    // 9500..9999 => huge effect double damage (5%)
    let r = wild_roll % 10000;

    if r < 4000 {
        // +25%
        let add_fp = FP_SCALE * 125 / 100; // 1.25
        *damage_fp = mul_fp_checked(*damage_fp, add_fp)?;
        return Ok(1);
    } else if r < 7000 {
        // heal attacker 10 hp
        att.current_hp = att.current_hp.saturating_add(10);
        if att.current_hp > att.max_hp {
            att.current_hp = att.max_hp;
        }
        return Ok(2);
    } else if r < 8500 {
        // reserved for stun: apply as dot to opponent as light penalty
        if att.nft_mint == battle.player1 {
            battle.player2_dot_damage = battle.player2_dot_damage.saturating_add(2);
            battle.player2_dot_turns = battle.player2_dot_turns.saturating_add(1);
        } else {
            battle.player1_dot_damage = battle.player1_dot_damage.saturating_add(2);
            battle.player1_dot_turns = battle.player1_dot_turns.saturating_add(1);
        }
        return Ok(3);
    } else if r < 9500 {
        // nothing
        return Ok(4);
    } else {
        // double damage
        *damage_fp = mul_fp_checked(*damage_fp, FP_SCALE * 2)?;
        return Ok(5);
    }
}
