use anchor_lang::prelude::*;
use anchor_lang::solana_program::hash::hashv;
use crate::constants::*;
use crate::errors::GameError;

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Default, InitSpace)]
pub struct SeedBatch {
    pub seed: [u8; SEED_LEN],
    pub start: u64,
    pub count: u32,
    pub consumed: u32,
}

#[account]
#[derive(InitSpace)]
pub struct EntropyPool {
    pub authority: Pubkey,
    pub vrf_oracle: Pubkey,
    pub head: u8,
    pub tail: u8,
    pub total_available: u64,
    pub global_next_index: u64,
    pub bump: u8,
    pub last_refill_ts: i64,
    pub batches: [SeedBatch; MAX_BATCHES],
}

impl EntropyPool {
    pub fn consume_seed_bytes_return_index(
        &mut self,
        signer: &Pubkey,
        user_tag: &[u8],
        turn_number: u32,
    ) -> Result<([u8; 32], u64)> {
        require!(self.total_available > 0, GameError::NoEntropyAvailable);

        // find head batch
        let mut idx = self.head as usize % MAX_BATCHES;
        // skip empty batches
        while self.batches[idx].count <= self.batches[idx].consumed {
            idx = (idx + 1) % MAX_BATCHES;
            // if looped fully and nothing available
            if idx == (self.head as usize % MAX_BATCHES) {
                return Err(error!(GameError::NoEntropyAvailable).into());
            }
        }
        let batch = &mut self.batches[idx];
        let offset = batch.start.saturating_add(batch.consumed as u64);
        let mut tn_bytes = [0u8; 4];
        tn_bytes.copy_from_slice(&turn_number.to_le_bytes());

        // Build the hash input: seed || offset_le || signer || user_tag || turn_number
        let h = hashv(&[
            &batch.seed,
            &offset.to_le_bytes(),
            &signer.to_bytes(),
            user_tag,
            &tn_bytes,
        ])
        .to_bytes();

        // update consumed counts and pool counters
        batch.consumed = batch.consumed.saturating_add(1);
        self.total_available = self.total_available.saturating_sub(1);
        let used_global_index = offset;
        if batch.consumed >= batch.count {
            // advance head
            self.head = ((self.head as usize + 1) % MAX_BATCHES) as u8;
        }

        Ok((h, used_global_index))
    }
}
