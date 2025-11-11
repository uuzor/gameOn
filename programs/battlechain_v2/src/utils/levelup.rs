use anchor_lang::prelude::*;
use crate::state::{Character, Progression};
use crate::events::ProgressionLevelUp;

pub fn next_level_xp(level: u16) -> u64 {
    // 100 * level^2
    let l = level as u64;
    100u64.saturating_mul(l.saturating_mul(l))
}

pub fn level_up_if_needed(
    prog: &mut Account<Progression>,
    ch: &mut Account<Character>,
) -> Result<()> {
    loop {
        let need = next_level_xp(prog.level);
        if prog.xp >= need {
            prog.xp = prog.xp.saturating_sub(need);
            prog.level = prog.level.saturating_add(1);

            // evolve stats modestly
            ch.max_hp = ch.max_hp.saturating_add((ch.max_hp / 20).max(1)); // +5%
            ch.current_hp = ch.max_hp;
            ch.base_damage_min = ch
                .base_damage_min
                .saturating_add((ch.base_damage_min / 10).max(1));
            ch.base_damage_max = ch
                .base_damage_max
                .saturating_add((ch.base_damage_max / 10).max(1));

            emit!(ProgressionLevelUp {
                nft_mint: prog.nft_mint,
                new_level: prog.level
            });
        } else {
            break;
        }
    }
    Ok(())
}
