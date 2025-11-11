# BattleChain V2 - Solana Anchor Smart Contract

A comprehensive turn-based NFT battle game built on Solana using the Anchor framework.

## Overview

BattleChain V2 is a fully on-chain turn-based battle game where players use NFTs as battle characters. The game features:

- **Character System**: 5 unique character classes (Warrior, Assassin, Mage, Tank, Trickster)
- **Battle Mechanics**: Turn-based combat with stances, combos, critical hits, and special abilities
- **Progression**: XP and leveling system for characters
- **Staking**: Battle offers with SOL or SPL token stakes
- **Randomness**: VRF-based entropy pool for deterministic randomness
- **Traits**: NFT trait application system

## Architecture

The contract is organized with separation of concerns:

```
src/
├── lib.rs              # Main program module with instruction handlers
├── constants.rs        # Game configuration constants
├── errors.rs           # Custom error definitions
├── events.rs           # Event definitions for on-chain logging
├── state/              # Account structures
│   ├── config.rs       # Global config account
│   ├── entropy.rs      # Entropy pool for randomness
│   ├── character.rs    # Character and progression data
│   ├── battle.rs       # Battle state management
│   └── offer.rs        # Battle offers and requests
└── utils/              # Helper functions
    ├── math.rs         # Fixed-point arithmetic
    ├── stance.rs       # Stance calculations
    ├── wildcard.rs     # Wildcard effects
    └── levelup.rs      # Character progression
```

## Building

```bash
cargo check
# or with anchor CLI (if available)
anchor build
```

## Character Classes

### Warrior
- HP: 120
- Damage: 8-15
- Crit: 15%
- Special: 3x damage (3 turn cooldown)

### Assassin
- HP: 90
- Damage: 12-20
- Crit: 35%
- Special: 3x damage (4 turn cooldown)

### Mage
- HP: 80
- Damage: 10-18
- Crit: 20%
- Special: Apply DoT (5 damage/turn for 3 turns, 3 turn cooldown)

### Tank
- HP: 150
- Damage: 6-12
- Crit: 10%
- Special: 50% damage reflection (4 turn cooldown)

### Trickster
- HP: 100
- Damage: 8-16
- Crit: 25%
- Special: 2x damage (2 turn cooldown)

## Battle Stances

Players' characters automatically adopt stances each turn based on class preferences and player settings:

- **Balanced**: Standard attack/defense (weight: 40)
- **Aggressive**: +30% attack, +50% damage received (weight: 20)
- **Defensive**: -30% attack, -50% damage received (weight: 20)
- **Berserker**: +100% attack, 25% self-damage (weight: 10)
- **Counter**: -10% attack, 40% damage reflected (weight: 10)

## Game Flow

1. **Character Creation**: Mint an NFT and create a character with a chosen class
2. **Create Offer**: Create a battle offer with stake amount, level requirements, and class restrictions
3. **Join Battle**: Other players join with their characters
4. **Approve & Battle**: Offer creator approves challenger, battle begins
5. **Execute Turns**: Players take turns executing attacks with their characters
6. **Battle End**: Battle ends when a player's health reaches 0
7. **Finalize**: Winner claims the staked tokens (minus protocol fee)

## Key Features

### Entropy System
- VRF-based entropy pool ensures deterministic randomness
- Seed batches refilled by authorized oracle
- Used for damage rolls, crits, dodges, and stance selection

### Fixed-Point Math
- Uses 6-decimal fixed-point arithmetic (FP_SCALE = 1,000,000)
- Prevents floating-point errors on-chain
- Damage calculations with multipliers capped at 10x

### Combo System
- Consecutive equal base damage rolls build combo (max 5)
- Each combo level adds 15% damage multiplier

### Wildcard Effects
- One-time use per battle per player
- 40%: +25% damage
- 30%: Heal 10 HP
- 15%: Apply mini-DoT to opponent
- 10%: No effect
- 5%: Double damage

## Technical Details

- **Anchor Version**: 0.29.0
- **Program ID**: 7rCHo4qDucdmW8FWQTvPCRtg6Y5vNSLj8L7vbuKiQ9Pu
- **Rust Edition**: 2021
- **Features**: init-if-needed (for account initialization)

## Configuration

The global config account stores:
- Admin authority
- Protocol fee (in basis points)
- Inactivity timeout for battles
- SPL token whitelist for staking
- Trait authority for NFT metadata

## Events

All major actions emit events:
- ConfigCreated, EntropyPoolCreated, SeedBatchRefilled
- CharacterCreated, ProgressionCreated, TraitApplied
- OfferCreated, JoinRequested, RequestWithdrawn, OfferCancelled
- BattleCreated, BattleForfeited, BattleEnded
- TurnResolved (detailed turn information)
- Various combat events (ComboApplied, SpecialUsed, AttackMissed, etc.)

## Security Considerations

- Entropy replay protection with monotonic indices
- Authorization checks for all privileged operations
- Escrow pattern for battle stakes
- Inactivity timeout mechanism for abandoned battles
- SPL token whitelist for staking
- Math overflow protection with checked operations

## Future Enhancements

The modular structure supports easy addition of:
- New character classes
- Additional stance types
- More wildcard effects
- Tournament systems
- Leaderboards
- Seasonal rewards

## License

See repository LICENSE file.
