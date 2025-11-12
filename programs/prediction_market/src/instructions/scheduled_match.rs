use anchor_lang::prelude::*;
use crate::constants::*;
use crate::errors::PredictionMarketError;
use crate::events::*;
use crate::state::*;

#[derive(Accounts)]
#[instruction(match_id: u64)]
pub struct CreateScheduledMatch<'info> {
    #[account(
        init,
        payer = creator,
        space = ScheduledMatch::LEN,
        seeds = [SCHEDULED_MATCH_SEED, match_id.to_le_bytes().as_ref()],
        bump
    )]
    pub scheduled_match: Account<'info, ScheduledMatch>,

    #[account(mut)]
    pub creator: Signer<'info>,

    pub system_program: Program<'info, System>,
}

pub fn create_scheduled_match_handler(
    ctx: Context<CreateScheduledMatch>,
    match_id: u64,
    player1: Pubkey,
    player2: Pubkey,
    game_reference: Pubkey,
    is_external: bool,
    betting_start: i64,
    betting_end: i64,
    estimated_battle_time: i64,
    title: String,
    description: String,
) -> Result<()> {
    let scheduled_match = &mut ctx.accounts.scheduled_match;
    let clock = Clock::get()?;

    // Validate betting period
    require!(
        betting_end > betting_start,
        PredictionMarketError::InvalidBettingPeriod
    );
    require!(
        betting_start >= clock.unix_timestamp,
        PredictionMarketError::InvalidBettingPeriod
    );

    // Validate players are different
    require!(
        player1 != player2,
        PredictionMarketError::InvalidAccount
    );

    // Initialize scheduled match
    scheduled_match.match_id = match_id;
    scheduled_match.player1 = player1;
    scheduled_match.player2 = player2;
    scheduled_match.game_reference = game_reference;
    scheduled_match.is_external = is_external;
    scheduled_match.external_result_id = None;
    scheduled_match.market = Pubkey::default(); // Will be set when market is created
    scheduled_match.betting_start = betting_start;
    scheduled_match.betting_end = betting_end;
    scheduled_match.estimated_battle_time = estimated_battle_time;
    scheduled_match.resolved_at = None;
    scheduled_match.status = MatchStatus::Open;
    scheduled_match.winner = None;
    scheduled_match.creator = ctx.accounts.creator.key();
    scheduled_match.title = title.clone();
    scheduled_match.metadata = description;
    scheduled_match.created_at = clock.unix_timestamp;
    scheduled_match.bump = ctx.bumps.scheduled_match;

    // Emit event
    emit!(ScheduledMatchCreated {
        match_id,
        scheduled_match: scheduled_match.key(),
        player1,
        player2,
        battle_offer: game_reference,
        betting_start,
        betting_end,
        estimated_battle_time,
    });

    Ok(())
}

#[derive(Accounts)]
pub struct UpdateScheduledMatch<'info> {
    #[account(
        mut,
        seeds = [SCHEDULED_MATCH_SEED, scheduled_match.match_id.to_le_bytes().as_ref()],
        bump = scheduled_match.bump
    )]
    pub scheduled_match: Account<'info, ScheduledMatch>,

    /// Authority (creator or admin)
    pub authority: Signer<'info>,
}

pub fn start_match_handler(ctx: Context<UpdateScheduledMatch>) -> Result<()> {
    let scheduled_match = &mut ctx.accounts.scheduled_match;

    // Validate current status
    require!(
        scheduled_match.status == MatchStatus::Open,
        PredictionMarketError::InvalidMatchStatus
    );

    // Update status to locked (betting ended)
    scheduled_match.status = MatchStatus::Locked;

    Ok(())
}

pub fn complete_match_handler(
    ctx: Context<UpdateScheduledMatch>,
    winner: Pubkey,
    _battle_account: Pubkey, // Prefix with _ to suppress unused warning
) -> Result<()> {
    let scheduled_match = &mut ctx.accounts.scheduled_match;
    let clock = Clock::get()?;

    // Validate current status
    require!(
        scheduled_match.status == MatchStatus::Locked || scheduled_match.status == MatchStatus::Open,
        PredictionMarketError::InvalidMatchStatus
    );

    // Validate winner is one of the players
    require!(
        winner == scheduled_match.player1 || winner == scheduled_match.player2,
        PredictionMarketError::InvalidAccount
    );

    // Update status
    scheduled_match.status = MatchStatus::Resolved;
    scheduled_match.winner = Some(winner);
    scheduled_match.resolved_at = Some(clock.unix_timestamp);

    Ok(())
}

pub fn cancel_match_handler(ctx: Context<UpdateScheduledMatch>) -> Result<()> {
    let scheduled_match = &mut ctx.accounts.scheduled_match;

    // Validate current status (can't cancel completed matches)
    require!(
        scheduled_match.status != MatchStatus::Resolved,
        PredictionMarketError::InvalidMatchStatus
    );

    // Update status
    scheduled_match.status = MatchStatus::Cancelled;

    Ok(())
}
