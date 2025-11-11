use anchor_lang::prelude::*;
use anchor_lang::solana_program::{
    hash::hashv, program::invoke_signed, pubkey::Pubkey, system_instruction, sysvar::clock::Clock,
};
use anchor_spl::associated_token::{self, AssociatedToken};
use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer};

declare_id!("7rCHo4qDucdmW8FWQTvPCRtg6Y5vNSLj8L7vbuKiQ9Pu");

// Fixed-point & limits
pub const FP_SCALE: u128 = 1_000_000u128;
pub const DEFAULT_CRIT_FP: u128 = FP_SCALE * 2;
pub const MAX_TOTAL_MULTIPLIER_FP: u128 = FP_SCALE * 10;
pub const MAX_COMBO_STACK: u8 = 5;
pub const SEED_LEN: usize = 32;
pub const MAX_BATCHES: usize = 8;
pub const MIN_ENTROPY_PER_TURN: u64 = 1;

#[program]
pub mod battlechain_v2 {
    use super::*;

    pub fn create_config(
        ctx: Context<CreateConfig>,
        fee_bps: u16,
        inactivity_timeout: i64,
        spl_whitelist: Vec<Pubkey>,
        trait_authority: Pubkey,
    ) -> Result<()> {
        let config_key = ctx.accounts.config.key();
        let admin_key = ctx.accounts.admin.key();

        let cfg = &mut ctx.accounts.config;
        cfg.admin = admin_key;
        cfg.fee_bps = fee_bps;
        cfg.inactivity_timeout = inactivity_timeout;
        cfg.spl_whitelist = spl_whitelist;
        cfg.trait_authority = trait_authority;
        cfg.bump = ctx.bumps.config;

        emit!(ConfigCreated {
            config: config_key,
            admin: admin_key
        });
        Ok(())
    }

    pub fn create_entropy_pool(ctx: Context<CreateEntropyPool>, vrf_oracle: Pubkey) -> Result<()> {
        let pool_key = ctx.accounts.pool.key();
        let authority_key = ctx.accounts.authority.key();
        let now = Clock::get()?.unix_timestamp;

        let pool = &mut ctx.accounts.pool;
        pool.authority = authority_key;
        pool.vrf_oracle = vrf_oracle;
        pool.head = 0;
        pool.tail = 0;
        pool.total_available = 0;
        pool.global_next_index = 0;
        pool.bump = ctx.bumps.pool;
        pool.last_refill_ts = now;
        pool.batches = [SeedBatch::default(); MAX_BATCHES];

        emit!(EntropyPoolCreated {
            pool: pool_key,
            vrf_oracle
        });
        Ok(())
    }

    pub fn refill_seed_batch(
        ctx: Context<RefillSeedBatch>,
        seed: [u8; SEED_LEN],
        start_index: u64,
        count: u32,
    ) -> Result<()> {
        let pool_key = ctx.accounts.pool.key();
        let caller = ctx.accounts.refiller.key();
        let now = Clock::get()?.unix_timestamp;

        let pool = &mut ctx.accounts.pool;
        require!(
            caller == pool.vrf_oracle || caller == pool.authority,
            GameError::UnauthorizedRefill
        );
        require!(count > 0, GameError::InvalidRange);
        require!(start_index >= pool.global_next_index, GameError::SeedReplay);

        let idx = pool.tail as usize % MAX_BATCHES;
        pool.batches[idx].seed = seed;
        pool.batches[idx].start = start_index;
        pool.batches[idx].count = count;
        pool.batches[idx].consumed = 0;

        pool.tail = ((pool.tail as usize + 1) % MAX_BATCHES) as u8;
        pool.total_available = pool.total_available.saturating_add(count as u64);
        pool.global_next_index = start_index
            .checked_add(count as u64)
            .ok_or(GameError::MathOverflow)?;
        pool.last_refill_ts = now;

        emit!(SeedBatchRefilled {
            pool: pool_key,
            added: count as u64,
            total_available: pool.total_available
        });
        Ok(())
    }

    pub fn create_character_from_nft(
        ctx: Context<CreateCharacterFromNft>,
        base_class: CharacterClass,
    ) -> Result<()> {
        let nft_mint_key = ctx.accounts.nft_mint.key();
        let payer_key = ctx.accounts.payer.key();
        let now = Clock::get()?.unix_timestamp;

        require!(
            ctx.accounts.nft_ata.mint == nft_mint_key,
            GameError::InvalidNftAta
        );
        require!(ctx.accounts.nft_ata.amount == 1, GameError::NotNftOwner);
        require!(
            ctx.accounts.nft_ata.owner == payer_key,
            GameError::NotNftOwner
        );

        let character = &mut ctx.accounts.character;
        character.nft_mint = nft_mint_key;
        character.base_class = base_class;

        match base_class {
            CharacterClass::Warrior => {
                character.max_hp = 120;
                character.current_hp = 120;
                character.base_damage_min = 8;
                character.base_damage_max = 15;
                character.crit_bps = 1500;
            }
            CharacterClass::Assassin => {
                character.max_hp = 90;
                character.current_hp = 90;
                character.base_damage_min = 12;
                character.base_damage_max = 20;
                character.crit_bps = 3500;
            }
            CharacterClass::Mage => {
                character.max_hp = 80;
                character.current_hp = 80;
                character.base_damage_min = 10;
                character.base_damage_max = 18;
                character.crit_bps = 2000;
            }
            CharacterClass::Tank => {
                character.max_hp = 150;
                character.current_hp = 150;
                character.base_damage_min = 6;
                character.base_damage_max = 12;
                character.crit_bps = 1000;
            }
            CharacterClass::Trickster => {
                character.max_hp = 100;
                character.current_hp = 100;
                character.base_damage_min = 8;
                character.base_damage_max = 16;
                character.crit_bps = 2500;
            }
        }

        character.defense = 0;
        character.dodge_bps = 0;
        character.special_cooldown = 0;
        character.last_base_damage = 0;
        character.combo_count = 0;
        character.lifes = 0;
        character.crit_multiplier_fp = DEFAULT_CRIT_FP as u32;
        character.mod_attack_bps = 0;
        character.mod_defense_bps = 0;
        character.mod_crit_bps = 0;
        character.rarity = 0;
        character.bump = ctx.bumps.character;
        character.created_at = now;

        if ctx.accounts.progression.to_account_info().data_is_empty() {
            let prog = &mut ctx.accounts.progression;
            prog.nft_mint = nft_mint_key;
            prog.xp = 0;
            prog.level = 1;
            prog.mmr = 100;
            prog.last_played = 0;
            prog.bump = ctx.bumps.progression;

            emit!(ProgressionCreated {
                nft_mint: nft_mint_key
            });
        }

        emit!(CharacterCreated {
            nft_mint: nft_mint_key,
            owner: payer_key
        });
        Ok(())
    }

    pub fn apply_trait_bundle(ctx: Context<ApplyTraitBundle>, bundle: TraitBundle) -> Result<()> {
        let trait_auth_key = ctx.accounts.trait_authority.key();

        require!(
            trait_auth_key == ctx.accounts.config.trait_authority,
            GameError::Unauthorized
        );

        let ch = &mut ctx.accounts.character;
        let nft_mint = ch.nft_mint;

        ch.mod_attack_bps = ch.mod_attack_bps.saturating_add(bundle.attack_bps);
        ch.mod_defense_bps = ch.mod_defense_bps.saturating_add(bundle.defense_bps);
        ch.mod_crit_bps = ch.mod_crit_bps.saturating_add(bundle.crit_bps);
        ch.rarity = bundle.rarity;

        emit!(TraitApplied {
            nft_mint,
            by: trait_auth_key
        });
        Ok(())
    }

    pub fn create_battle_offer(
        ctx: Context<CreateBattleOffer>,
        offer_nonce: u64,
        currency: Currency,
        stake_amount: u64,
        min_level: u16,
        max_level: u16,
        allowed_classes: Vec<CharacterClass>,
        auto_approve: bool,
        start_ts: i64,
        inactivity_timeout: i64,
    ) -> Result<()> {
        if let Currency::SPL(mint) = currency {
            require!(
                ctx.accounts.config.spl_whitelist.contains(&mint),
                GameError::SPLNotWhitelisted
            );
        }

        let clock = Clock::get()?;
        require!(
            start_ts >= clock.unix_timestamp,
            GameError::InvalidTimestamp
        );

        let offer_key = ctx.accounts.offer.key();
        let creator_key = ctx.accounts.creator.key();

        let offer = &mut ctx.accounts.offer;
        offer.creator = creator_key;
        offer.offer_nonce = offer_nonce;
        offer.currency = currency;
        offer.stake_amount = stake_amount;
        offer.min_level = min_level;
        offer.max_level = max_level;
        offer.allowed_classes = allowed_classes;
        offer.auto_approve = auto_approve;
        offer.start_ts = start_ts;
        offer.created_at = clock.unix_timestamp;
        offer.is_active = true;
        offer.inactivity_timeout = inactivity_timeout;
        offer.bump = ctx.bumps.offer;

        match offer.currency {
            Currency::SOL => {
                if stake_amount > 0 {
                    invoke_signed(
                        &system_instruction::transfer(
                            &creator_key,
                            &offer_key,
                            stake_amount,
                        ),
                        &[
                            ctx.accounts.creator.to_account_info(),
                            ctx.accounts.offer.to_account_info(),
                        ],
                        &[],
                    )?;
                }
            }
            Currency::SPL(_) => {
                if stake_amount > 0 {
                    if ctx.accounts.offer_escrow.as_ref().unwrap().to_account_info().data_is_empty() {
                        let cpi_accounts = associated_token::Create {
                            payer: ctx.accounts.creator.to_account_info(),
                            associated_token: ctx.accounts.offer_escrow.as_ref().unwrap().to_account_info(),
                            authority: ctx.accounts.offer.to_account_info(),
                            mint: ctx.accounts.currency_mint.as_ref().unwrap().to_account_info(),
                            system_program: ctx.accounts.system_program.to_account_info(),
                            token_program: ctx.accounts.token_program.to_account_info(),
                        };
                        let cpi_ctx = CpiContext::new(
                            ctx.accounts.associated_token_program.to_account_info(),
                            cpi_accounts,
                        );
                        associated_token::create(cpi_ctx)?;
                    }

                    let cpi_accounts = token::Transfer {
                        from: ctx.accounts.creator_ata.as_ref().unwrap().to_account_info(),
                        to: ctx.accounts.offer_escrow.as_ref().unwrap().to_account_info(),
                        authority: ctx.accounts.creator.to_account_info(),
                    };
                    let cpi_ctx = CpiContext::new(
                        ctx.accounts.token_program.to_account_info(),
                        cpi_accounts
                    );
                    token::transfer(cpi_ctx, stake_amount)?;
                }
            }
        }

        emit!(OfferCreated {
            offer: offer_key,
            creator: creator_key,
            stake: stake_amount
        });
        Ok(())
    }

    pub fn join_battle_offer(ctx: Context<JoinBattleOffer>, offered_stake: u64) -> Result<()> {
        require!(ctx.accounts.offer.is_active, GameError::OfferNotActive);

        let prog = &ctx.accounts.progression;
        require!(
            prog.level >= ctx.accounts.offer.min_level && prog.level <= ctx.accounts.offer.max_level,
            GameError::CharacterConstraint
        );

        if !ctx.accounts.offer.allowed_classes.is_empty() {
            require!(
                ctx.accounts.offer.allowed_classes.contains(&ctx.accounts.character.base_class),
                GameError::CharacterConstraint
            );
        }

        let clock = Clock::get()?;
        let offer_key = ctx.accounts.offer.key();
        let challenger_key = ctx.accounts.challenger.key();
        let character_key = ctx.accounts.character.key();
        let request_key = ctx.accounts.request.key();
        let currency = &ctx.accounts.offer.currency;

        let request = &mut ctx.accounts.request;
        request.offer = offer_key;
        request.challenger = challenger_key;
        request.character = character_key;
        request.offered_stake = offered_stake;
        request.created_at = clock.unix_timestamp;
        request.status = JoinStatus::Pending;
        request.bump = ctx.bumps.request;

        match currency {
            Currency::SOL => {
                if offered_stake > 0 {
                    invoke_signed(
                        &system_instruction::transfer(
                            &challenger_key,
                            &request_key,
                            offered_stake,
                        ),
                        &[
                            ctx.accounts.challenger.to_account_info(),
                            ctx.accounts.request.to_account_info(),
                        ],
                        &[],
                    )?;
                }
            }
            Currency::SPL(_) => {
                if offered_stake > 0 {
                    if ctx.accounts.request_escrow.as_ref().unwrap().to_account_info().data_is_empty() {
                        let cpi_accounts = associated_token::Create {
                            payer: ctx.accounts.challenger.to_account_info(),
                            associated_token: ctx.accounts.request_escrow.as_ref().unwrap().to_account_info(),
                            authority: ctx.accounts.request.to_account_info(),
                            mint: ctx.accounts.currency_mint.as_ref().unwrap().to_account_info(),
                            system_program: ctx.accounts.system_program.to_account_info(),
                            token_program: ctx.accounts.token_program.to_account_info(),
                        };
                        let cpi_ctx = CpiContext::new(
                            ctx.accounts.associated_token_program.to_account_info(),
                            cpi_accounts,
                        );
                        associated_token::create(cpi_ctx)?;
                    }

                    let cpi_accounts = token::Transfer {
                        from: ctx.accounts.challenger_ata.as_ref().unwrap().to_account_info(),
                        to: ctx.accounts.request_escrow.as_ref().unwrap().to_account_info(),
                        authority: ctx.accounts.challenger.to_account_info(),
                    };
                    token::transfer(
                        CpiContext::new(ctx.accounts.token_program.to_account_info(), cpi_accounts),
                        offered_stake,
                    )?;
                }
            }
        }

        emit!(JoinRequested {
            offer: offer_key,
            request: request_key,
            challenger: challenger_key,
            stake: offered_stake
        });
        Ok(())
    }

    pub fn withdraw_request(ctx: Context<WithdrawRequest>) -> Result<()> {
        require!(
            ctx.accounts.request.status == JoinStatus::Pending,
            GameError::InvalidRequestState
        );
        require!(
            ctx.accounts.challenger.key() == ctx.accounts.request.challenger,
            GameError::Unauthorized
        );

        let currency = &ctx.accounts.offer.currency;
        let challenger_key = ctx.accounts.challenger.key();
        let request_key = ctx.accounts.request.key();
        let request_bump = ctx.accounts.request.bump;
        let offer_key = ctx.accounts.offer.key();

        match currency {
            Currency::SOL => {
                let bal = ctx.accounts.request.to_account_info().lamports();
                if bal > 0 {
                    invoke_signed(
                        &system_instruction::transfer(
                            &request_key,
                            &challenger_key,
                            bal,
                        ),
                        &[
                            ctx.accounts.request.to_account_info(),
                            ctx.accounts.challenger.to_account_info(),
                        ],
                        &[],
                    )?;
                }
            }
            Currency::SPL(_) => {
                let amount = ctx.accounts.request_escrow.as_ref().unwrap().amount;
                if amount > 0 {
                    let cpi_accounts = token::Transfer {
                        from: ctx.accounts.request_escrow.as_ref().unwrap().to_account_info(),
                        to: ctx.accounts.challenger_ata.as_ref().unwrap().to_account_info(),
                        authority: ctx.accounts.request.to_account_info(),
                    };
                    let signer_seeds = &[
                        b"request",
                        offer_key.as_ref(),
                        challenger_key.as_ref(),
                        &[request_bump],
                    ];
                    token::transfer(
                        CpiContext::new_with_signer(
                            ctx.accounts.token_program.to_account_info(),
                            cpi_accounts,
                            &[signer_seeds],
                        ),
                        amount,
                    )?;
                }
            }
        }

        ctx.accounts.request.status = JoinStatus::Withdrawn;

        emit!(RequestWithdrawn {
            request: request_key,
            by: challenger_key
        });
        Ok(())
    }

    pub fn cancel_offer(ctx: Context<CancelOffer>) -> Result<()> {
        require!(
            ctx.accounts.creator.key() == ctx.accounts.offer.creator,
            GameError::Unauthorized
        );
        require!(ctx.accounts.offer.is_active, GameError::OfferNotActive);

        let currency = &ctx.accounts.offer.currency;
        let creator_key = ctx.accounts.creator.key();
        let offer_key = ctx.accounts.offer.key();
        let offer_nonce = ctx.accounts.offer.offer_nonce;
        let offer_bump = ctx.accounts.offer.bump;

        match currency {
            Currency::SOL => {
                let bal = ctx.accounts.offer.to_account_info().lamports();
                if bal > 0 {
                    invoke_signed(
                        &system_instruction::transfer(
                            &offer_key,
                            &creator_key,
                            bal,
                        ),
                        &[
                            ctx.accounts.offer.to_account_info(),
                            ctx.accounts.creator.to_account_info(),
                        ],
                        &[],
                    )?;
                }
            }
            Currency::SPL(_) => {
                let amount = ctx.accounts.offer_escrow.as_ref().unwrap().amount;
                if amount > 0 {
                    let cpi_accounts = token::Transfer {
                        from: ctx.accounts.offer_escrow.as_ref().unwrap().to_account_info(),
                        to: ctx.accounts.creator_ata.as_ref().unwrap().to_account_info(),
                        authority: ctx.accounts.offer.to_account_info(),
                    };
                    let signer_seeds = &[
                        b"offer",
                        creator_key.as_ref(),
                        &offer_nonce.to_le_bytes(),
                        &[offer_bump],
                    ];
                    token::transfer(
                        CpiContext::new_with_signer(
                            ctx.accounts.token_program.to_account_info(),
                            cpi_accounts,
                            &[signer_seeds],
                        ),
                        amount,
                    )?;
                }
            }
        }

        ctx.accounts.offer.is_active = false;

        emit!(OfferCancelled {
            offer: offer_key,
            by: creator_key
        });
        Ok(())
    }

    pub fn approve_challenger(ctx: Context<ApproveChallenger>) -> Result<()> {

         let battle_key = ctx.accounts.battle.key();
        let battle_account_info = ctx.accounts.battle.to_account_info();
        let creator_key = ctx.accounts.creator.key();

        require!(ctx.accounts.offer.is_active, GameError::OfferNotActive);
        require!(
            ctx.accounts.request.status == JoinStatus::Pending,
            GameError::InvalidRequestState
        );
        require!(creator_key == ctx.accounts.offer.creator, GameError::Unauthorized);

        let clock = Clock::get()?;

        let player1 = ctx.accounts.offer.creator;
        let player2 = ctx.accounts.request.challenger;
        let offer_nonce = ctx.accounts.offer.offer_nonce;
        let start_ts = ctx.accounts.offer.start_ts;
        let inactivity_timeout = if ctx.accounts.offer.inactivity_timeout > 0 {
            ctx.accounts.offer.inactivity_timeout
        } else {
            ctx.accounts.config.inactivity_timeout
        };
        let currency = &ctx.accounts.offer.currency;
        let offer_stake = ctx.accounts.offer.stake_amount;
        let request_stake = ctx.accounts.request.offered_stake;
        let total_stake = offer_stake.saturating_add(request_stake);

        let battle = &mut ctx.accounts.battle;
        battle.battle_id = offer_nonce.wrapping_add(clock.unix_timestamp as u64);
        battle.player1 = player1;
        battle.player2 = player2;
        battle.start_ts = start_ts;
        battle.current_turn = 0;
        battle.turn_number = 0;
        battle.player1_health = 100;
        battle.player2_health = 100;
        battle.state = BattleState::Active;
        battle.player1_stance = StanceType::Balanced;
        battle.player2_stance = StanceType::Balanced;
        battle.created_at = clock.unix_timestamp;
        battle.inactivity_timeout = inactivity_timeout;
        battle.last_action_ts = clock.unix_timestamp;
        battle.winner = None;
        battle.player1_dot_damage = 0;
        battle.player2_dot_damage = 0;
        battle.player1_dot_turns = 0;
        battle.player2_dot_turns = 0;
        battle.player1_reflection = 0;
        battle.player2_reflection = 0;
        battle.player1_miss_count = 0;
        battle.player2_miss_count = 0;
        battle.last_entropy_index = 0;
        battle.bump = ctx.bumps.battle;
        battle.player1_used_wildcard = false;
        battle.player2_used_wildcard = false;
        battle.player1_special_used = false;
        battle.player2_special_used = false;
        battle.player1_pref_set = false;
        battle.player2_pref_set = false;
        battle.player1_preference = StanceType::Balanced as u8;
        battle.player2_preference = StanceType::Balanced as u8;

        match currency {
            Currency::SOL => {
                let offer_bal = ctx.accounts.offer.to_account_info().lamports();
                if offer_bal > 0 {
                    invoke_signed(
                        &system_instruction::transfer(
                            &ctx.accounts.offer.key(),
                            &battle_key,
                            offer_stake,
                        ),
                        &[
                            ctx.accounts.offer.to_account_info(),
                            battle_account_info.clone(),
                        ],
                        &[],
                    )?;
                }

                let req_bal = ctx.accounts.request.to_account_info().lamports();
                if req_bal > 0 {
                    invoke_signed(
                        &system_instruction::transfer(
                            &ctx.accounts.request.key(),
                            &battle_key,
                            request_stake,
                        ),
                        &[
                            ctx.accounts.request.to_account_info(),
                            battle_account_info.clone(),
                        ],
                        &[],
                    )?;
                }
            }
            Currency::SPL(_) => {
                if ctx.accounts.battle_escrow.as_ref().unwrap().to_account_info().data_is_empty() {
                    let cpi_accounts = associated_token::Create {
                        payer: ctx.accounts.creator.to_account_info(),
                        associated_token: ctx.accounts.battle_escrow.as_ref().unwrap().to_account_info(),
                        authority: battle_account_info.clone(),
                        mint: ctx.accounts.currency_mint.as_ref().unwrap().to_account_info(),
                        system_program: ctx.accounts.system_program.to_account_info(),
                        token_program: ctx.accounts.token_program.to_account_info(),
                    };
                    associated_token::create(CpiContext::new(
                        ctx.accounts.associated_token_program.to_account_info(),
                        cpi_accounts,
                    ))?;
                }

                let offer_amount = ctx.accounts.offer_escrow.as_ref().unwrap().amount;
                if offer_amount > 0 {
                    let offer_creator = ctx.accounts.offer.creator;
                    let offer_nonce_val = ctx.accounts.offer.offer_nonce;
                    let offer_bump_val = ctx.accounts.offer.bump;

                    let cpi_accounts = token::Transfer {
                        from: ctx.accounts.offer_escrow.as_ref().unwrap().to_account_info(),
                        to: ctx.accounts.battle_escrow.as_ref().unwrap().to_account_info(),
                        authority: ctx.accounts.offer.to_account_info(),
                    };
                    let offer_nonce_bytes = offer_nonce_val.to_le_bytes();
                    let offer_bump_array = [offer_bump_val];
                    let signer_seeds = &[&[
                        b"offer",
                        offer_creator.as_ref(),
                        &offer_nonce_bytes,
                        &offer_bump_array,
                    ][..]];
                    token::transfer(
                        CpiContext::new_with_signer(
                            ctx.accounts.token_program.to_account_info(),
                            cpi_accounts,
                            signer_seeds,
                        ),
                        offer_amount,
                    )?;
                }

                let req_amount = ctx.accounts.request_escrow.as_ref().unwrap().amount;
                if req_amount > 0 {
                    let req_challenger = ctx.accounts.request.challenger;
                    let req_bump_val = ctx.accounts.request.bump;
                    let offer_key_val = ctx.accounts.offer.key();

                    let cpi_accounts = token::Transfer {
                        from: ctx.accounts.request_escrow.as_ref().unwrap().to_account_info(),
                        to: ctx.accounts.battle_escrow.as_ref().unwrap().to_account_info(),
                        authority: ctx.accounts.request.to_account_info(),
                    };
                    let req_bump_array = [req_bump_val];
                    let signer_seeds = &[&[
                        b"request",
                        offer_key_val.as_ref(),
                        req_challenger.as_ref(),
                        &req_bump_array,
                    ][..]];
                    token::transfer(
                        CpiContext::new_with_signer(
                            ctx.accounts.token_program.to_account_info(),
                            cpi_accounts,
                            signer_seeds,
                        ),
                        req_amount,
                    )?;
                }
            }
        }

        ctx.accounts.request.status = JoinStatus::Approved;
        ctx.accounts.offer.is_active = false;

        require!(
            ctx.accounts.pool.total_available >= 1,
            GameError::NoEntropyAvailable
        );

        let (seed_bytes, used_index) = ctx.accounts.pool.consume_seed_bytes_return_index(
            &creator_key,
            b"first_mover",
            battle.turn_number as u32,
        )?;

        let pick = derive_u64_from_seed_bytes(&seed_bytes, 0) % 2;
        require!(
            used_index > battle.last_entropy_index,
            GameError::SeedReplay
        );

        battle.last_entropy_index = used_index;
        battle.current_turn = if pick == 0 { 1 } else { 2 };

        emit!(BattleCreated {
            battle: battle_key,
            player1,
            player2,
            first_turn: battle.current_turn,
            stake_total: total_stake
        });
        Ok(())
    }

    pub fn execute_turn(
        ctx: Context<ExecuteTurn>,
        preference: i8,
        use_special: bool,
        use_wildcard: bool,
    ) -> Result<()> {
        let signer_key = ctx.accounts.signer.key();
        let battle_key = ctx.accounts.battle.key();

        require!(
            ctx.accounts.battle.state == BattleState::Active,
            GameError::InvalidBattleState
        );

        let is_player1 = if signer_key == ctx.accounts.battle.player1 {
            true
        } else if signer_key == ctx.accounts.battle.player2 {
            false
        } else {
            return Err(error!(GameError::Unauthorized).into());
        };

        if is_player1 {
            require!(ctx.accounts.battle.current_turn == 1, GameError::NotYourTurn);
        } else {
            require!(ctx.accounts.battle.current_turn == 2, GameError::NotYourTurn);
        }

        require!(
            ctx.accounts.pool.total_available >= MIN_ENTROPY_PER_TURN,
            GameError::NoEntropyAvailable
        );

        let now = Clock::get()?.unix_timestamp;
        ctx.accounts.battle.last_action_ts = now;

        if preference >= 0 {
            let p = preference as u8;
            require!(p <= StanceType::Counter as u8, GameError::InvalidRange);
            if is_player1 {
                ctx.accounts.battle.player1_preference = p;
                ctx.accounts.battle.player1_pref_set = true;
            } else {
                ctx.accounts.battle.player2_preference = p;
                ctx.accounts.battle.player2_pref_set = true;
            }
        }

        let turn_num = ctx.accounts.battle.turn_number;
        let (seed_bytes, used_index) = ctx.accounts.pool.consume_seed_bytes_return_index(
            &signer_key,
            b"turn",
            turn_num as u32
        )?;

        require!(
            used_index > ctx.accounts.battle.last_entropy_index,
            GameError::SeedReplay
        );
        ctx.accounts.battle.last_entropy_index = used_index;

        let min_d = ctx.accounts.attacker_character.base_damage_min as u64;
        let max_d = ctx.accounts.attacker_character.base_damage_max as u64;
        let base_rand = derive_u64_from_seed_bytes(&seed_bytes, 0) % (max_d - min_d + 1) + min_d;
        let crit_roll = derive_u64_from_seed_bytes(&seed_bytes, 1) % 10000;
        let dodge_roll = derive_u64_from_seed_bytes(&seed_bytes, 2) % 10000;
        let wild_roll = derive_u64_from_seed_bytes(&seed_bytes, 3) % 10000;
        let att_stance_pick = derive_u64_from_seed_bytes(&seed_bytes, 4);
        let def_stance_pick = derive_u64_from_seed_bytes(&seed_bytes, 5);

        let att_pref = if is_player1 {
            (ctx.accounts.battle.player1_pref_set, ctx.accounts.battle.player1_preference)
        } else {
            (ctx.accounts.battle.player2_pref_set, ctx.accounts.battle.player2_preference)
        };
        let def_pref = if is_player1 {
            (ctx.accounts.battle.player2_pref_set, ctx.accounts.battle.player2_preference)
        } else {
            (ctx.accounts.battle.player1_pref_set, ctx.accounts.battle.player1_preference)
        };

        let att_stance = sample_stance_with_preference(
            ctx.accounts.attacker_character.base_class,
            att_pref,
            att_stance_pick
        );
        let def_stance = sample_stance_with_preference(
            ctx.accounts.defender_character.base_class,
            def_pref,
            def_stance_pick
        );

        if is_player1 {
            ctx.accounts.battle.player1_stance = att_stance;
            ctx.accounts.battle.player2_stance = def_stance;
        } else {
            ctx.accounts.battle.player2_stance = att_stance;
            ctx.accounts.battle.player1_stance = def_stance;
        }

        let attacker_level = ctx.accounts.attacker_prog.level as u64;
        let base_u128 = (base_rand as u128)
            .checked_add((attacker_level.saturating_sub(1) * 2) as u128)
            .ok_or(GameError::MathOverflow)?;
        let mut damage_fp = base_u128
            .checked_mul(FP_SCALE)
            .ok_or(GameError::MathOverflow)?;

        let stored_crit_fp = ctx.accounts.attacker_character.crit_multiplier_fp as u128;
        let crit_mult_fp = if stored_crit_fp >= DEFAULT_CRIT_FP {
            stored_crit_fp
        } else {
            DEFAULT_CRIT_FP
        };

        let is_crit = (crit_roll as u64) < ctx.accounts.attacker_character.crit_bps as u64;
        if is_crit {
            damage_fp = mul_fp_checked(damage_fp, crit_mult_fp)?;
        }

        let last_base = ctx.accounts.attacker_character.last_base_damage;
        let current_base = base_rand.min(u64::from(u16::MAX)) as u16;

        if last_base == current_base {
            ctx.accounts.attacker_character.combo_count =
                ctx.accounts.attacker_character.combo_count.saturating_add(1);
            if ctx.accounts.attacker_character.combo_count > MAX_COMBO_STACK {
                ctx.accounts.attacker_character.combo_count = MAX_COMBO_STACK;
            }
            let combo_mult_fp = FP_SCALE + (150_000u128 * (ctx.accounts.attacker_character.combo_count as u128));
            damage_fp = mul_fp_checked(damage_fp, combo_mult_fp)?;

            emit!(ComboApplied {
                battle: battle_key,
                attacker: ctx.accounts.attacker_character.nft_mint,
                combo: ctx.accounts.attacker_character.combo_count,
                added: 0
            });
        } else {
            ctx.accounts.attacker_character.combo_count = 0;
        }
        ctx.accounts.attacker_character.last_base_damage = current_base;

        if use_wildcard {
            if is_player1 {
                require!(
                    !ctx.accounts.battle.player1_used_wildcard,
                    GameError::InvalidRequestState
                );
                ctx.accounts.battle.player1_used_wildcard = true;
            } else {
                require!(
                    !ctx.accounts.battle.player2_used_wildcard,
                    GameError::InvalidRequestState
                );
                ctx.accounts.battle.player2_used_wildcard = true;
            }

            apply_wildcard_effect(
                wild_roll,
                &mut damage_fp,
                &mut ctx.accounts.battle,
                &mut ctx.accounts.attacker_character,
                &mut ctx.accounts.defender_character,

            )?;

            emit!(SpecialUsed {
                battle: battle_key,
                attacker: ctx.accounts.attacker_character.nft_mint,
                special: 255
            });
        }

        if use_special {
            if is_player1 {
                require!(!ctx.accounts.battle.player1_special_used, GameError::InvalidRequestState);
                ctx.accounts.battle.player1_special_used = true;
            } else {
                require!(!ctx.accounts.battle.player2_special_used, GameError::InvalidRequestState);
                ctx.accounts.battle.player2_special_used = true;
            }

            require!(
                ctx.accounts.attacker_character.special_cooldown == 0,
                GameError::SpecialOnCooldown
            );

            match ctx.accounts.attacker_character.base_class {
                CharacterClass::Warrior => {
                    damage_fp = mul_fp_checked(damage_fp, FP_SCALE * 3)?;
                    ctx.accounts.attacker_character.special_cooldown = 3;
                }
                CharacterClass::Assassin => {
                    damage_fp = mul_fp_checked(damage_fp, FP_SCALE * 3)?;
                    ctx.accounts.attacker_character.special_cooldown = 4;
                }
                CharacterClass::Mage => {
                    if is_player1 {
                        ctx.accounts.battle.player2_dot_damage = ctx.accounts.battle.player2_dot_damage.saturating_add(5);
                        ctx.accounts.battle.player2_dot_turns = ctx.accounts.battle.player2_dot_turns.saturating_add(3);
                    } else {
                        ctx.accounts.battle.player1_dot_damage = ctx.accounts.battle.player1_dot_damage.saturating_add(5);
                        ctx.accounts.battle.player1_dot_turns = ctx.accounts.battle.player1_dot_turns.saturating_add(3);
                    }
                    ctx.accounts.attacker_character.special_cooldown = 3;
                }
                CharacterClass::Tank => {
                    if is_player1 {
                        ctx.accounts.battle.player1_reflection = ctx.accounts.battle.player1_reflection.saturating_add(50);
                    } else {
                        ctx.accounts.battle.player2_reflection = ctx.accounts.battle.player2_reflection.saturating_add(50);
                    }
                    ctx.accounts.attacker_character.special_cooldown = 4;
                }
                CharacterClass::Trickster => {
                    damage_fp = mul_fp_checked(damage_fp, FP_SCALE * 2)?;
                    ctx.accounts.attacker_character.special_cooldown = 2;
                }
            }

            emit!(SpecialUsed {
                battle: battle_key,
                attacker: ctx.accounts.attacker_character.nft_mint,
                special: ctx.accounts.attacker_character.base_class as u8
            });
        }

        let (att_fp, def_fp, self_bps, counter_bps) = stance_multipliers(att_stance, def_stance);
        damage_fp = mul_fp_checked(damage_fp, att_fp)?;
        damage_fp = mul_fp_checked(damage_fp, def_fp)?;

        let max_allowed = base_u128
            .checked_mul(MAX_TOTAL_MULTIPLIER_FP)
            .ok_or(GameError::MathOverflow)?;
        if damage_fp > max_allowed {
            damage_fp = max_allowed;
            emit!(DamageClamped {
                battle: battle_key,
                attacker: ctx.accounts.attacker_character.nft_mint
            });
        }

        let mut final_damage = fp_to_u64_clamped(damage_fp, GameError::MathOverflow)?;

        if ctx.accounts.defender_character.defense as u64 >= final_damage {
            final_damage = 0;
        } else {
            final_damage = final_damage.saturating_sub(ctx.accounts.defender_character.defense as u64);
        }

        let mut is_dodge = false;
        if (dodge_roll as u64) < ctx.accounts.defender_character.dodge_bps as u64 {
            final_damage = 0;
            is_dodge = true;
            if is_player1 {
                ctx.accounts.battle.player1_miss_count = ctx.accounts.battle.player1_miss_count.saturating_add(1);
            } else {
                ctx.accounts.battle.player2_miss_count = ctx.accounts.battle.player2_miss_count.saturating_add(1);
            }
            emit!(AttackMissed {
                battle: battle_key,
                attacker: ctx.accounts.attacker_character.nft_mint,
                defender: ctx.accounts.defender_character.nft_mint
            });
        }

        let mut reflected_amount = 0u64;
        let mut counter_amount = 0u64;
        let mut self_amount = 0u64;

        if is_player1 {
            ctx.accounts.battle.player2_health = ctx.accounts.battle.player2_health.saturating_sub(final_damage);
            if ctx.accounts.battle.player1_reflection > 0 && final_damage > 0 {
                reflected_amount = final_damage.saturating_mul(ctx.accounts.battle.player1_reflection as u64) / 100;
                ctx.accounts.battle.player1_health = ctx.accounts.battle.player1_health.saturating_sub(reflected_amount);
                emit!(ReflectionApplied {
                    battle: battle_key,
                    defender: ctx.accounts.attacker_character.nft_mint,
                    reflected: reflected_amount
                });
            }
            if counter_bps > 0 && final_damage > 0 {
                counter_amount = final_damage.saturating_mul(counter_bps as u64) / 10000u64;
                ctx.accounts.battle.player1_health = ctx.accounts.battle.player1_health.saturating_sub(counter_amount);
                emit!(CounterApplied {
                    battle: battle_key,
                    player: ctx.accounts.attacker_character.nft_mint,
                    damage: counter_amount
                });
            }
            if self_bps > 0 {
                self_amount = final_damage.saturating_mul(self_bps as u64) / 10000u64;
                ctx.accounts.battle.player1_health = ctx.accounts.battle.player1_health.saturating_sub(self_amount);
                emit!(SelfDamageApplied {
                    battle: battle_key,
                    player: ctx.accounts.attacker_character.nft_mint,
                    damage: self_amount
                });
            }
        } else {
            ctx.accounts.battle.player1_health = ctx.accounts.battle.player1_health.saturating_sub(final_damage);
            if ctx.accounts.battle.player2_reflection > 0 && final_damage > 0 {
                reflected_amount = final_damage.saturating_mul(ctx.accounts.battle.player2_reflection as u64) / 100;
                ctx.accounts.battle.player2_health = ctx.accounts.battle.player2_health.saturating_sub(reflected_amount);
                emit!(ReflectionApplied {
                    battle: battle_key,
                    defender: ctx.accounts.attacker_character.nft_mint,
                    reflected: reflected_amount
                });
            }
            if counter_bps > 0 && final_damage > 0 {
                counter_amount = final_damage.saturating_mul(counter_bps as u64) / 10000u64;
                ctx.accounts.battle.player2_health = ctx.accounts.battle.player2_health.saturating_sub(counter_amount);
                emit!(CounterApplied {
                    battle: battle_key,
                    player: ctx.accounts.attacker_character.nft_mint,
                    damage: counter_amount
                });
            }
            if self_bps > 0 {
                self_amount = final_damage.saturating_mul(self_bps as u64) / 10000u64;
                ctx.accounts.battle.player2_health = ctx.accounts.battle.player2_health.saturating_sub(self_amount);
                emit!(SelfDamageApplied {
                    battle: battle_key,
                    player: ctx.accounts.attacker_character.nft_mint,
                    damage: self_amount
                });
            }
        }

        if ctx.accounts.attacker_character.special_cooldown > 0 {
            ctx.accounts.attacker_character.special_cooldown =
                ctx.accounts.attacker_character.special_cooldown.saturating_sub(1);
        }

        if ctx.accounts.battle.player1_dot_turns > 0 {
            let dot = ctx.accounts.battle.player1_dot_damage;
            if dot > 0 {
                ctx.accounts.battle.player1_health = ctx.accounts.battle.player1_health.saturating_sub(dot);
            }
            ctx.accounts.battle.player1_dot_turns = ctx.accounts.battle.player1_dot_turns.saturating_sub(1);
        }
        if ctx.accounts.battle.player2_dot_turns > 0 {
            let dot = ctx.accounts.battle.player2_dot_damage;
            if dot > 0 {
                ctx.accounts.battle.player2_health = ctx.accounts.battle.player2_health.saturating_sub(dot);
            }
            ctx.accounts.battle.player2_dot_turns = ctx.accounts.battle.player2_dot_turns.saturating_sub(1);
        }

        let p1_health = ctx.accounts.battle.player1_health;
        let p2_health = ctx.accounts.battle.player2_health;

        if p1_health == 0 || p2_health == 0 {
            ctx.accounts.battle.state = BattleState::Finished;
            let winner_opt = if p1_health > p2_health {
                Some(ctx.accounts.battle.player1)
            } else if p2_health > p1_health {
                Some(ctx.accounts.battle.player2)
            } else {
                None
            };
            ctx.accounts.battle.winner = winner_opt;

            if let Some(wpk) = winner_opt {
                if wpk == ctx.accounts.battle.player1 {
                    ctx.accounts.attacker_prog.xp = ctx.accounts.attacker_prog.xp.saturating_add(100);
                    level_up_if_needed(
                        &mut ctx.accounts.attacker_prog,
                        &mut ctx.accounts.attacker_character,
                    )?;
                } else {
                    ctx.accounts.defender_prog.xp = ctx.accounts.defender_prog.xp.saturating_add(100);
                    level_up_if_needed(
                        &mut ctx.accounts.defender_prog,
                        &mut ctx.accounts.defender_character,
                    )?;
                }
            } else {
                ctx.accounts.attacker_prog.xp = ctx.accounts.attacker_prog.xp.saturating_add(25);
                ctx.accounts.defender_prog.xp = ctx.accounts.defender_prog.xp.saturating_add(25);
            }

            emit!(BattleEnded {
                battle: battle_key,
                winner: winner_opt
            });
        } else {
            ctx.accounts.battle.current_turn = if ctx.accounts.battle.current_turn == 1 { 2 } else { 1 };
            ctx.accounts.battle.turn_number = ctx.accounts.battle.turn_number.saturating_add(1);
        }

        emit!(TurnResolved {
            battle: battle_key,
            turn_number: ctx.accounts.battle.turn_number,
            attacker: ctx.accounts.attacker_character.nft_mint,
            defender: ctx.accounts.defender_character.nft_mint,
            used_entropy_index: used_index,
            att_stance: att_stance as u8,
            def_stance: def_stance as u8,
            base_roll: base_rand,
            is_crit,
            crit_roll,
            dodge_roll,
            is_dodge,
            wild_roll,
            used_wild: use_wildcard,
            wildcard_result: 0,
            damage_dealt: final_damage,
            post_health_p1: ctx.accounts.battle.player1_health,
            post_health_p2: ctx.accounts.battle.player2_health,
            combo_count: ctx.accounts.attacker_character.combo_count,
            special_used: use_special,
        });

        Ok(())
    }

    pub fn forfeit_by_timeout(ctx: Context<ForfeitByTimeout>) -> Result<()> {
        let now = Clock::get()?.unix_timestamp;
        let battle_key = ctx.accounts.battle.key();

        require!(
            ctx.accounts.battle.state == BattleState::Active,
            GameError::InvalidBattleState
        );
        require!(
            now.saturating_sub(ctx.accounts.battle.last_action_ts) > ctx.accounts.battle.inactivity_timeout,
            GameError::TimeoutNotReached
        );

        let winner = if ctx.accounts.battle.current_turn == 1 {
            ctx.accounts.battle.player2
        } else {
            ctx.accounts.battle.player1
        };

        ctx.accounts.battle.state = BattleState::Finished;
        ctx.accounts.battle.winner = Some(winner);

        emit!(BattleForfeited {
            battle: battle_key,
            winner
        });
        Ok(())
    }

    pub fn finalize_battle(ctx: Context<FinalizeBattle>) -> Result<()> {
        require!(
            ctx.accounts.battle.state == BattleState::Finished,
            GameError::BattleNotFinished
        );

        let fee_bps = ctx.accounts.config.fee_bps;
        let currency = &ctx.accounts.offer.currency;
        let battle_id = ctx.accounts.battle.battle_id;
        let battle_bump = ctx.accounts.battle.bump;
        let winner = ctx.accounts.battle.winner;
        let player1 = ctx.accounts.battle.player1;
        let player2 = ctx.accounts.battle.player2;

        match currency {
            Currency::SOL => {
                let total = ctx.accounts.battle.to_account_info().lamports();
                let fee = ((total as u128) * (fee_bps as u128) / 10_000u128) as u64;
                let payout = total.saturating_sub(fee);

                if fee > 0 {
                    invoke_signed(
                        &system_instruction::transfer(
                            &ctx.accounts.battle.key(),
                            &ctx.accounts.treasury.key(),
                            fee,
                        ),
                        &[
                            ctx.accounts.battle.to_account_info(),
                            ctx.accounts.treasury.to_account_info(),
                        ],
                        &[&[b"battle", &battle_id.to_le_bytes(), &[battle_bump]]],
                    )?;
                }

                if let Some(winner_pk) = winner {
                    let dest = if winner_pk == player1 {
                        &ctx.accounts.player1_owner
                    } else {
                        &ctx.accounts.player2_owner
                    };
                    invoke_signed(
                        &system_instruction::transfer(
                            &ctx.accounts.battle.key(),
                            &dest.key(),
                            payout,
                        ),
                        &[
                            ctx.accounts.battle.to_account_info(),
                            dest.to_account_info(),
                        ],
                        &[&[b"battle", &battle_id.to_le_bytes(), &[battle_bump]]],
                    )?;
                } else {
                    invoke_signed(
                        &system_instruction::transfer(
                            &ctx.accounts.battle.key(),
                            &ctx.accounts.treasury.key(),
                            payout,
                        ),
                        &[
                            ctx.accounts.battle.to_account_info(),
                            ctx.accounts.treasury.to_account_info(),
                        ],
                        &[&[b"battle", &battle_id.to_le_bytes(), &[battle_bump]]],
                    )?;
                }
            }
            Currency::SPL(_) => {
                let total_tokens = ctx.accounts.battle_escrow.as_ref().unwrap().amount;
                let fee_amt = ((total_tokens as u128) * (fee_bps as u128) / 10_000u128) as u64;
                let payout_amt = total_tokens.saturating_sub(fee_amt);

                if fee_amt > 0 {
                    let cpi_accounts = token::Transfer {
                        from: ctx.accounts.battle_escrow.as_ref().unwrap().to_account_info(),
                        to: ctx.accounts.treasury_ata.as_ref().unwrap().to_account_info(),
                        authority: ctx.accounts.battle.to_account_info(),
                    };
                    let battle_array = &battle_id.to_le_bytes()[..];
                    let battle_bump_array = [battle_bump];
                    let signer_seeds = &[&[b"battle", battle_array, &battle_bump_array][..]];
                    token::transfer(
                        CpiContext::new_with_signer(
                            ctx.accounts.token_program.to_account_info(),
                            cpi_accounts,
                            signer_seeds,
                        ),
                        fee_amt,
                    )?;
                }

                if let Some(winner_pk) = winner {
                    let dest_ata = if winner_pk == player1 {
                        &ctx.accounts.player1_ata
                    } else {
                        &ctx.accounts.player2_ata
                    };
                    let cpi_accounts = token::Transfer {
                        from: ctx.accounts.battle_escrow.as_ref().unwrap().to_account_info(),
                        to: dest_ata.as_ref().unwrap().to_account_info(),
                        authority: ctx.accounts.battle.to_account_info(),
                    };
                    let battle_array = &battle_id.to_le_bytes()[..];
                    let battle_bump_array =[battle_bump];
                    let signer_seeds = &[&[b"battle", battle_array, &battle_bump_array][..]];
                    token::transfer(
                        CpiContext::new_with_signer(
                            ctx.accounts.token_program.to_account_info(),
                            cpi_accounts,
                            signer_seeds,
                        ),
                        payout_amt,
                    )?;
                } else {
                    let cpi_accounts = token::Transfer {
                        from: ctx.accounts.battle_escrow.as_ref().unwrap().to_account_info(),
                        to: ctx.accounts.treasury_ata.as_ref().unwrap().to_account_info(),
                        authority: ctx.accounts.battle.to_account_info(),
                    };
                    let battle_bump_array =[battle_bump];
                    let battle_array = &battle_id.to_le_bytes()[..];
                    let signer_seeds = &[&[b"battle", battle_array, &battle_bump_array][..]];
                    token::transfer(
                        CpiContext::new_with_signer(
                            ctx.accounts.token_program.to_account_info(),
                            cpi_accounts,
                            signer_seeds,
                        ),
                        payout_amt,
                    )?;
                }
            }
        }

        emit!(BattleSettled {
            battle: ctx.accounts.battle.key(),
            total_paid: 0
        });
        Ok(())
    }
}
// ------------------------
// CONTEXTS & ACCOUNTS
// ------------------------

#[derive(Accounts)]
pub struct CreateConfig<'info> {
    #[account(init, payer = admin, space = 8 + Config::INIT_SPACE, seeds = [b"config"], bump)]
    pub config: Account<'info, Config>,
    #[account(mut)]
    pub admin: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct CreateEntropyPool<'info> {
    #[account(init, payer = payer, space = 8 + EntropyPool::INIT_SPACE, seeds = [b"entropy_pool"], bump)]
    pub pool: Account<'info, EntropyPool>,
    #[account(mut)]
    pub payer: Signer<'info>,
    /// CHECK: authority (admin)
    pub authority: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct RefillSeedBatch<'info> {
    #[account(mut, has_one = authority)]
    pub pool: Account<'info, EntropyPool>,
    /// CHECK: refiller (oracle)
    pub refiller: Signer<'info>,
    /// CHECK: authority (for has_one)
    pub authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct CreateCharacterFromNft<'info> {
    pub nft_mint: Account<'info, Mint>,
    #[account(init, payer = payer, space = 8 + Character::INIT_SPACE, seeds = [b"character", nft_mint.key().as_ref()], bump)]
    pub character: Account<'info, Character>,
    #[account(init_if_needed, payer = payer, space = 8 + Progression::INIT_SPACE, seeds = [b"progress", nft_mint.key().as_ref()], bump)]
    pub progression: Account<'info, Progression>,
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(mut)]
    pub nft_ata: Account<'info, TokenAccount>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
}

#[derive(Accounts)]
#[instruction(offer_nonce: u64)]
pub struct CreateBattleOffer<'info> {
    #[account(init, payer = creator, space = 8 + Offer::INIT_SPACE, seeds = [b"offer", creator.key.as_ref(), &offer_nonce.to_le_bytes()], bump)]
    pub offer: Account<'info, Offer>,
    #[account(mut)]
    pub creator: Signer<'info>,
    #[account(mut)]
    pub creator_ata: Option<Account<'info, TokenAccount>>, // if SPL
    #[account(mut)]
    pub offer_escrow: Option<Account<'info, TokenAccount>>, // to be created if SPL
    #[account(mut)]
    pub currency_mint: Option<Account<'info, Mint>>,
    pub config: Account<'info, Config>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct JoinBattleOffer<'info> {
    #[account(mut)]
    pub offer: Account<'info, Offer>,
    #[account(init, payer = challenger, space = 8 + Request::INIT_SPACE, seeds = [b"request", offer.key().as_ref(), challenger.key.as_ref()], bump)]
    pub request: Account<'info, Request>,
    #[account(mut)]
    pub character: Account<'info, Character>,
    #[account(mut)]
    pub progression: Account<'info, Progression>,
    #[account(mut)]
    pub challenger: Signer<'info>,
    #[account(mut)]
    pub challenger_ata: Option<Account<'info, TokenAccount>>,
    #[account(mut)]
    pub request_escrow: Option<Account<'info, TokenAccount>>,
    #[account(mut)]
    pub currency_mint: Option<Account<'info, Mint>>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
    pub config: Account<'info, Config>,
}

#[derive(Accounts)]
pub struct WithdrawRequest<'info> {
    #[account(mut, has_one = challenger)]
    pub request: Account<'info, Request>,
    #[account(mut)]
    pub challenger: Signer<'info>,
    #[account(mut)]
    pub offer: Account<'info, Offer>,
    #[account(mut)]
    pub request_escrow: Option<Account<'info, TokenAccount>>,
    #[account(mut)]
    pub challenger_ata: Option<Account<'info, TokenAccount>>,
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct CancelOffer<'info> {
    #[account(mut, has_one = creator)]
    pub offer: Account<'info, Offer>,
    #[account(mut)]
    pub creator: Signer<'info>,
    #[account(mut)]
    pub offer_escrow: Option<Account<'info, TokenAccount>>,
    #[account(mut)]
    pub creator_ata: Option<Account<'info, TokenAccount>>,
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct ApproveChallenger<'info> {
    #[account(mut, has_one = creator)]
    pub offer: Account<'info, Offer>,
    #[account(mut, has_one = offer)]
    pub request: Account<'info, Request>,
    #[account(init, payer = creator, space = 8 + Battle::INIT_SPACE, seeds = [b"battle", &offer.offer_nonce.to_le_bytes()[..], offer.creator.as_ref(), request.challenger.as_ref()], bump)]
    pub battle: Account<'info, Battle>,
    #[account(mut)]
    pub creator: Signer<'info>,
    #[account(mut)]
    pub pool: Account<'info, EntropyPool>,
    // escrow accounts for SPL flows
    #[account(mut)]
    pub offer_escrow: Option<Account<'info, TokenAccount>>,
    #[account(mut)]
    pub request_escrow: Option<Account<'info, TokenAccount>>,
    #[account(mut)]
    pub battle_escrow: Option<Account<'info, TokenAccount>>,
    #[account(mut)]
    pub currency_mint: Option<Account<'info, Mint>>,
    pub config: Account<'info, Config>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct ExecuteTurn<'info> {
    #[account(mut)]
    pub pool: Account<'info, EntropyPool>,
    #[account(mut)]
    pub battle: Account<'info, Battle>,
    #[account(mut)]
    pub attacker_character: Account<'info, Character>,
    #[account(mut)]
    pub defender_character: Account<'info, Character>,
    #[account(mut)]
    pub attacker_prog: Account<'info, Progression>,
    #[account(mut)]
    pub defender_prog: Account<'info, Progression>,
    #[account(mut)]
    pub attacker_nft_ata: Account<'info, TokenAccount>,
    #[account(mut)]
    pub defender_nft_ata: Account<'info, TokenAccount>,
    #[account(mut)]
    pub player1_character_opt: Option<Account<'info, Character>>,
    #[account(mut)]
    pub player2_character_opt: Option<Account<'info, Character>>,
    pub signer: Signer<'info>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub clock: Sysvar<'info, Clock>,
}

#[derive(Accounts)]
pub struct ForfeitByTimeout<'info> {
    #[account(mut)]
    pub battle: Account<'info, Battle>,
    pub caller: Signer<'info>,
}

#[derive(Accounts)]
pub struct FinalizeBattle<'info> {
    #[account(mut)]
    pub battle: Account<'info, Battle>,
    #[account(mut)]
    pub offer: Account<'info, Offer>,
    /// CHECK: treasury account
    #[account(mut)]
    pub treasury: UncheckedAccount<'info>,

    pub config: Account<'info, Config>,
    // SPL relevant accounts
    #[account(mut)]
    pub battle_escrow: Option<Account<'info, TokenAccount>>,
    #[account(mut)]
    pub treasury_ata: Option<Account<'info, TokenAccount>>,
    #[account(mut)]
    pub player1_ata: Option<Account<'info, TokenAccount>>,
    #[account(mut)]
    pub player2_ata: Option<Account<'info, TokenAccount>>,
    #[account(mut)]
    pub player1_owner: Signer<'info>,
    #[account(mut)]
    pub player2_owner: Signer<'info>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct ApplyTraitBundle<'info> {
    #[account(mut)]
    pub character: Account<'info, Character>,
    pub config: Account<'info, Config>,
    pub trait_authority: Signer<'info>,
}

// ------------------------
// ACCOUNTS / STRUCTS
// ------------------------
#[account]
#[derive(InitSpace)]
pub struct Config {
    pub admin: Pubkey,
    pub fee_bps: u16,
    pub inactivity_timeout: i64,
    #[max_len(8)]
    pub spl_whitelist: Vec<Pubkey>,
    pub trait_authority: Pubkey,
    pub bump: u8,
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

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Default, InitSpace)]
pub struct SeedBatch {
    pub seed: [u8; SEED_LEN],
    pub start: u64,
    pub count: u32,
    pub consumed: u32,
}

#[account]
#[derive(InitSpace)]
pub struct Character {
    pub nft_mint: Pubkey,
    pub base_class: CharacterClass,
    pub max_hp: u32,
    pub current_hp: u32,
    pub base_damage_min: u16,
    pub base_damage_max: u16,
    pub crit_bps: u16,
    pub crit_multiplier_fp: u32,
    pub dodge_bps: u16,
    pub defense: u16,
    pub special_cooldown: u8,
    pub last_base_damage: u16,
    pub combo_count: u8,
    pub lifes: u8,
    // trait modifiers:
    pub mod_attack_bps: i16,
    pub mod_defense_bps: i16,
    pub mod_crit_bps: i16,
    pub rarity: u8,
    pub created_at: i64,
    pub bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct Progression {
    pub nft_mint: Pubkey,
    pub xp: u64,
    pub level: u16,
    pub mmr: u64,
    pub last_played: i64,
    pub bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct Offer {
    pub creator: Pubkey,
    pub offer_nonce: u64,
    pub currency: Currency,
    pub stake_amount: u64,
    pub min_level: u16,
    pub max_level: u16,
    #[max_len(5)]
    pub allowed_classes: Vec<CharacterClass>,
    pub auto_approve: bool,
    pub start_ts: i64,
    pub inactivity_timeout: i64,
    pub created_at: i64,
    pub is_active: bool,
    pub bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct Request {
    pub offer: Pubkey,
    pub challenger: Pubkey,
    pub character: Pubkey,
    pub offered_stake: u64,
    pub created_at: i64,
    pub status: JoinStatus,
    pub bump: u8,
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

// ------------------------
// ENUMS & SMALL TYPES
// ------------------------
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, InitSpace, Debug)]
pub enum CharacterClass {
    Warrior,
    Assassin,
    Mage,
    Tank,
    Trickster,
}

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

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, InitSpace, Debug)]
pub enum JoinStatus {
    Pending,
    Approved,
    Rejected,
    Withdrawn,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq, Eq, InitSpace, Debug)]
pub enum Currency {
    SOL,
    SPL(Pubkey),
}

// Trait bundle
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Default)]
pub struct TraitBundle {
    pub rarity: u8,
    pub attack_bps: i16,
    pub defense_bps: i16,
    pub crit_bps: i16,
    pub nonce: i64,
}

// ------------------------
// EVENTS
// ------------------------
#[event]
pub struct ConfigCreated {
    pub config: Pubkey,
    pub admin: Pubkey,
}
#[event]
pub struct EntropyPoolCreated {
    pub pool: Pubkey,
    pub vrf_oracle: Pubkey,
}
#[event]
pub struct SeedBatchRefilled {
    pub pool: Pubkey,
    pub added: u64,
    pub total_available: u64,
}
#[event]
pub struct ProgressionCreated {
    pub nft_mint: Pubkey,
}
#[event]
pub struct CharacterCreated {
    pub nft_mint: Pubkey,
    pub owner: Pubkey,
}
#[event]
pub struct TraitApplied {
    pub nft_mint: Pubkey,
    pub by: Pubkey,
}
#[event]
pub struct OfferCreated {
    pub offer: Pubkey,
    pub creator: Pubkey,
    pub stake: u64,
}
#[event]
pub struct JoinRequested {
    pub offer: Pubkey,
    pub request: Pubkey,
    pub challenger: Pubkey,
    pub stake: u64,
}
#[event]
pub struct RequestWithdrawn {
    pub request: Pubkey,
    pub by: Pubkey,
}
#[event]
pub struct OfferCancelled {
    pub offer: Pubkey,
    pub by: Pubkey,
}
#[event]
pub struct BattleCreated {
    pub battle: Pubkey,
    pub player1: Pubkey,
    pub player2: Pubkey,
    pub first_turn: u8,
    pub stake_total: u64,
}
#[event]
pub struct BattleForfeited {
    pub battle: Pubkey,
    pub winner: Pubkey,
}
#[event]
pub struct BattleEnded {
    pub battle: Pubkey,
    pub winner: Option<Pubkey>,
}
#[event]
pub struct DamageClamped {
    pub battle: Pubkey,
    pub attacker: Pubkey,
}
#[event]
pub struct ComboApplied {
    pub battle: Pubkey,
    pub attacker: Pubkey,
    pub combo: u8,
    pub added: u64,
}
#[event]
pub struct SpecialUsed {
    pub battle: Pubkey,
    pub attacker: Pubkey,
    pub special: u8,
}
#[event]
pub struct AttackMissed {
    pub battle: Pubkey,
    pub attacker: Pubkey,
    pub defender: Pubkey,
}
#[event]
pub struct ReflectionApplied {
    pub battle: Pubkey,
    pub defender: Pubkey,
    pub reflected: u64,
}
#[event]
pub struct CounterApplied {
    pub battle: Pubkey,
    pub player: Pubkey,
    pub damage: u64,
}
#[event]
pub struct SelfDamageApplied {
    pub battle: Pubkey,
    pub player: Pubkey,
    pub damage: u64,
}
#[event]
pub struct LifeConsumed {
    pub character: Pubkey,
    pub remaining: u8,
}
#[event]
pub struct TurnResolved {
    pub battle: Pubkey,
    pub turn_number: u64,
    pub attacker: Pubkey,
    pub defender: Pubkey,
    pub used_entropy_index: u64,
    pub att_stance: u8,
    pub def_stance: u8,
    pub base_roll: u64,
    pub is_crit: bool,
    pub crit_roll: u64,
    pub dodge_roll: u64,
    pub is_dodge: bool,
    pub wild_roll: u64,
    pub used_wild: bool,
    pub wildcard_result: u8,
    pub damage_dealt: u64,
    pub post_health_p1: u64,
    pub post_health_p2: u64,
    pub combo_count: u8,
    pub special_used: bool,
}
#[event]
pub struct BattleSettled {
    pub battle: Pubkey,
    pub total_paid: u64,
}
#[event]
pub struct ProgressionLevelUp {
    pub nft_mint: Pubkey,
    pub new_level: u16,
}

// ------------------------
// HELPERS: FP math, entropy consumption, levelup
// ------------------------
fn mul_fp_checked(value_fp: u128, mul_fp: u128) -> Result<u128> {
    let prod = value_fp
        .checked_mul(mul_fp)
        .ok_or(GameError::MathOverflow)?;
    Ok(prod.checked_div(FP_SCALE).ok_or(GameError::MathOverflow)?)
}

fn fp_to_u64_clamped(value_fp: u128, err: GameError) -> Result<u64> {
    let val = value_fp.checked_div(FP_SCALE).ok_or(err)?;
    if val > (u64::MAX as u128) {
        return Err(err.into());
    }
    Ok(val as u64)
}

// stance multipliers: returns attacker_fp, defender_fp, self_damage_bps, counter_bps
fn stance_multipliers(att: StanceType, def: StanceType) -> (u128, u128, u16, u16) {
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

// Entropy consumption: read one batch entry, return the 32-byte derived seed + global index
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
        ]).to_bytes();

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

// derive independent u64 from seed bytes using tag
fn derive_u64_from_seed_bytes(seed: &[u8; 32], tag: u8) -> u64 {
    let h = hashv(&[seed, &[tag]]).to_bytes();
    let mut arr = [0u8; 8];
    arr.copy_from_slice(&h[0..8]);
    u64::from_le_bytes(arr)
}

// sample stance with preference bias + class bias
fn sample_stance_with_preference(class: CharacterClass, pref: (bool, u8), rng: u64) -> StanceType {
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
            // slightly favor Berserker and Aggressive
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
        // multiply preference weight by factor
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

// simple wildcard effect application (example weighted outcomes)
fn apply_wildcard_effect(
    wild_roll: u64,
    damage_fp: &mut u128,
    battle: &mut Account<Battle>,
    att: &mut Account<Character>,
    _def: &mut Account<Character>,
) -> Result<u8> {
    // Table:
    // 0..3999 => +25% damage (40%)
    // 4000..6999 => heal attacker 10 hp (30%)
    // 7000..8499 => stun opponent next turn (15%) => we won't implement full stun here; reserved
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
        // reserved for stun: apply as dot to opponent as light penalty (represent stun as small dot for now)
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

// level up logic: simple quadratic XP curve
fn next_level_xp(level: u16) -> u64 {
    // 100 * level^2
    let l = level as u64;
    100u64.saturating_mul(l.saturating_mul(l))
}
fn level_up_if_needed(prog: &mut Account<Progression>, ch: &mut Account<Character>) -> Result<()> {
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

// ------------------------
// ERRORS
// ------------------------
#[error_code]
pub enum GameError {
    #[msg("Unauthorized refill")]
    UnauthorizedRefill,
    #[msg("Seed replay")]
    SeedReplay,
    #[msg("Entropy pool full")]
    EntropyPoolFull,
    #[msg("No entropy available")]
    NoEntropyAvailable,
    #[msg("Invalid index")]
    InvalidIndex,
    #[msg("Invalid range")]
    InvalidRange,
    #[msg("Math overflow")]
    MathOverflow,
    #[msg("Invalid NFT token account")]
    InvalidNftAta,
    #[msg("Not NFT owner")]
    NotNftOwner,
    #[msg("Offer not active")]
    OfferNotActive,
    #[msg("Character fails constraints")]
    CharacterConstraint,
    #[msg("Unauthorized")]
    Unauthorized,
    #[msg("Invalid request state")]
    InvalidRequestState,
    #[msg("Invalid battle state")]
    InvalidBattleState,
    #[msg("Battle already finished")]
    BattleAlreadyFinished,
    #[msg("Not your turn")]
    NotYourTurn,
    #[msg("Special on cooldown")]
    SpecialOnCooldown,
    #[msg("Invalid timestamp")]
    InvalidTimestamp,
    #[msg("Battle not finished")]
    BattleNotFinished,
    #[msg("Auto-approve disabled")]
    AutoApproveDisabled,
    #[msg("SPL not whitelisted")]
    SPLNotWhitelisted,
    #[msg("Timeout not reached")]
    TimeoutNotReached,
}
