use crate::constants::FP_SCALE;
use crate::state::{CharacterClass, StanceType};

pub fn stance_multipliers(att: StanceType, def: StanceType) -> (u128, u128, u16, u16) {
    let mut att_fp = FP_SCALE;
    let mut def_fp = FP_SCALE;
    let mut self_bps = 0u16;
    let mut counter_bps = 0u16;

    match att {
        StanceType::Aggressive => att_fp = FP_SCALE * 130 / 100,
        StanceType::Defensive => att_fp = FP_SCALE * 70 / 100,
        StanceType::Berserker => {
            att_fp = FP_SCALE * 200 / 100;
            self_bps = 2500;
        }
        StanceType::Counter => att_fp = FP_SCALE * 90 / 100,
        StanceType::Balanced => {}
    }

    match def {
        StanceType::Defensive => def_fp = FP_SCALE * 50 / 100,
        StanceType::Aggressive => def_fp = FP_SCALE * 150 / 100,
        StanceType::Counter => counter_bps = 4000,
        _ => {}
    }

    (att_fp, def_fp, self_bps, counter_bps)
}

pub fn sample_stance_with_preference(
    class: CharacterClass,
    pref: (bool, u8),
    rng: u64,
) -> StanceType {
    // baseline weights
    // Balanced=40, Aggressive=20, Defensive=20, Berserker=10, Counter=10 (sum=100)
    let mut weights = vec![
        (StanceType::Balanced, 40u32),
        (StanceType::Aggressive, 20u32),
        (StanceType::Defensive, 20u32),
        (StanceType::Berserker, 10u32),
        (StanceType::Counter, 10u32),
    ];

    // class biases (small adjustments)
    match class {
        CharacterClass::Warrior => {
            adjust_weight(&mut weights, StanceType::Berserker, 5);
            adjust_weight(&mut weights, StanceType::Aggressive, 3);
        }
        CharacterClass::Assassin => {
            adjust_weight(&mut weights, StanceType::Aggressive, 6);
            adjust_weight(&mut weights, StanceType::Counter, 2);
        }
        CharacterClass::Mage => {
            adjust_weight(&mut weights, StanceType::Defensive, 4);
            adjust_weight(&mut weights, StanceType::Balanced, 3);
        }
        CharacterClass::Tank => {
            adjust_weight(&mut weights, StanceType::Defensive, 8);
            adjust_weight(&mut weights, StanceType::Counter, 2);
        }
        CharacterClass::Trickster => {
            adjust_weight(&mut weights, StanceType::Aggressive, 4);
            adjust_weight(&mut weights, StanceType::Balanced, 4);
        }
    }

    // preference bias if set
    if pref.0 {
        let pref_st = match pref.1 {
            0 => StanceType::Balanced,
            1 => StanceType::Aggressive,
            2 => StanceType::Defensive,
            3 => StanceType::Berserker,
            4 => StanceType::Counter,
            _ => StanceType::Balanced,
        };
        multiply_weight(&mut weights, pref_st, 3u32); // 3x bias
    }

    // sample using rng
    let total: u128 = weights.iter().map(|(_, w)| *w as u128).sum();
    let pick = (rng as u128) % total;
    let mut acc = 0u128;
    for (st, w) in weights {
        acc += w as u128;
        if pick < acc {
            return st;
        }
    }
    StanceType::Balanced
}

fn adjust_weight(weights: &mut Vec<(StanceType, u32)>, stance: StanceType, delta: i32) {
    for (st, w) in weights.iter_mut() {
        if *st == stance {
            let nw = (*w as i32).saturating_add(delta);
            *w = nw.max(0) as u32;
        }
    }
}

fn multiply_weight(weights: &mut Vec<(StanceType, u32)>, stance: StanceType, factor: u32) {
    for (st, w) in weights.iter_mut() {
        if *st == stance {
            *w = (*w).saturating_mul(factor);
        }
    }
}
