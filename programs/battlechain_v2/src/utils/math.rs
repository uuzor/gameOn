use anchor_lang::prelude::*;
use anchor_lang::solana_program::hash::hashv;
use crate::constants::FP_SCALE;
use crate::errors::GameError;

pub fn mul_fp_checked(value_fp: u128, mul_fp: u128) -> Result<u128> {
    let prod = value_fp
        .checked_mul(mul_fp)
        .ok_or(GameError::MathOverflow)?;
    Ok(prod.checked_div(FP_SCALE).ok_or(GameError::MathOverflow)?)
}

pub fn fp_to_u64_clamped(value_fp: u128, err: GameError) -> Result<u64> {
    let val = value_fp.checked_div(FP_SCALE).ok_or(err)?;
    if val > (u64::MAX as u128) {
        return Err(err.into());
    }
    Ok(val as u64)
}

pub fn derive_u64_from_seed_bytes(seed: &[u8; 32], tag: u8) -> u64 {
    let h = hashv(&[seed, &[tag]]).to_bytes();
    let mut arr = [0u8; 8];
    arr.copy_from_slice(&h[0..8]);
    u64::from_le_bytes(arr)
}
