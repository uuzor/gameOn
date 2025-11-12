# BattleChain: On-Chain NFT Battle System

## 📋 Table of Contents
- [Overview](#overview)
- [Game Mechanics](#game-mechanics)
- [Character System](#character-system)
- [Battle System](#battle-system)
- [Rounds-Based Execution](#rounds-based-execution)
- [Entropy & Randomness](#entropy--randomness)
- [Economic Model](#economic-model)
- [Technical Architecture](#technical-architecture)
- [User Flows](#user-flows)
- [Smart Contract API](#smart-contract-api)

---

## 🎮 Overview

**BattleChain** is a fully on-chain, turn-based battle game built on Solana where players pit their NFT characters against each other in strategic combat. Every battle is deterministic yet unpredictable, provably fair, and gas-optimized for the Solana blockchain.

### Key Features

✅ **NFT-Based Characters**: Each character is a unique Solana NFT with verifiable stats
✅ **5 Character Classes**: Warrior, Assassin, Mage, Tank, Trickster - each with unique abilities
✅ **5 Battle Stances**: Dynamic combat system with rock-paper-scissors-style counters
✅ **Rounds-Based Execution**: 3 turns per round, max 3 rounds per battle (9 total turns)
✅ **VRF Entropy System**: Cryptographically secure randomness for fair combat
✅ **Fully On-Chain**: All battle logic executes on Solana - no off-chain computation
✅ **Gas Optimized**: 67% fewer transactions, 25% lower compute units vs traditional approach
✅ **Stake-Based Matches**: Players wager SOL, USDC, or USDT on battle outcomes

### Game Loop

```
1. Create Character (NFT)
   └─ Choose class, set base stats, apply traits

2. Challenge Opponent
   └─ Create battle offer with stake amount

3. Accept Challenge
   └─ Join battle offer, lock in characters

4. Execute Battle (3 Rounds)
   └─ Round 1: Turns 1-3
   └─ Round 2: Turns 4-6
   └─ Round 3: Turns 7-9

5. Determine Winner
   └─ Last character standing or highest HP after 9 turns

6. Distribute Rewards
   └─ Winner gets stake + opponent's stake (minus fee)
```

---

## ⚔️ Game Mechanics

### Core Combat Loop

Each turn follows this sequence:

```
1. Stance Selection
   ├─ AI picks stance based on class + preference
   └─ Affected by previous combat history

2. Damage Calculation
   ├─ Base damage from character stats
   ├─ Stance multipliers applied
   ├─ Critical hit roll (RNG)
   └─ Dodge roll (RNG)

3. Special Abilities
   ├─ Class-specific powers
   ├─ Cooldown management
   └─ Strategic timing

4. Wildcard Effects
   ├─ Random power-ups/debuffs
   ├─ Game-changing moments
   └─ 5% chance per turn

5. Status Effects
   ├─ Damage over time (DoT)
   ├─ Reflection damage
   └─ Counter attacks

6. Health Update
   └─ Check for knockout
```

### Damage Formula

```rust
Base Damage = Random(min_damage, max_damage)

// Apply stance multipliers
Damage *= attacker_stance_multiplier
Damage /= defender_stance_multiplier

// Apply trait bonuses
Damage *= (1 + attack_bonus_bps / 10000)

// Critical hit check
if random() < crit_chance {
    Damage *= crit_multiplier
    combo_count += 1
}

// Dodge check
if random() < dodge_chance {
    Damage = 0
    miss_count += 1
}

// Apply defense reduction
Final Damage = Damage - defense
Final Damage = max(Final Damage, 1) // Minimum 1 damage
```

### Turn Resolution

```
Attacker's Turn:
1. Select stance (AI or player preference)
2. Roll for critical hit
3. Roll for defender dodge
4. Calculate damage
5. Apply special ability (if used)
6. Apply wildcard (if triggered)
7. Deal damage to defender
8. Apply DoT/reflection/counter
9. Update battle state
10. Switch to defender's turn
```

---

## 🦸 Character System

### Character Classes

Each class has unique base stats and special abilities:

#### 1. **Warrior** ⚔️
```
Base Stats:
├─ HP: 120 (High survivability)
├─ Min Damage: 8
├─ Max Damage: 15
├─ Crit Chance: 15%
├─ Crit Multiplier: 1.8x
├─ Dodge Chance: 8%
└─ Defense: 10

Special Ability: "Battle Fury"
├─ Cooldown: 3 turns
├─ Effect: +50% damage for 2 turns
└─ Best Used: Mid-battle when momentum is high

Playstyle: Balanced bruiser, sustains through battles
Strengths: High HP, consistent damage
Weaknesses: Low mobility, predictable
```

#### 2. **Assassin** 🗡️
```
Base Stats:
├─ HP: 90 (Glass cannon)
├─ Min Damage: 12
├─ Max Damage: 20
├─ Crit Chance: 35% (Highest!)
├─ Crit Multiplier: 2.2x
├─ Dodge Chance: 15%
└─ Defense: 5

Special Ability: "Shadow Strike"
├─ Cooldown: 2 turns
├─ Effect: Guaranteed critical hit + ignore 50% defense
└─ Best Used: Finish move when opponent is low

Playstyle: High-risk, high-reward burst damage
Strengths: Massive crits, high dodge
Weaknesses: Low HP, inconsistent without crits
```

#### 3. **Mage** 🔮
```
Base Stats:
├─ HP: 80 (Lowest HP)
├─ Min Damage: 10
├─ Max Damage: 18
├─ Crit Chance: 20%
├─ Crit Multiplier: 2.0x
├─ Dodge Chance: 10%
└─ Defense: 6

Special Ability: "Arcane Blast"
├─ Cooldown: 3 turns
├─ Effect: 25 fixed damage (ignores all defenses)
└─ Best Used: Against high-defense targets

Playstyle: Consistent magic damage, strategic timing
Strengths: Ignores defense, reliable damage
Weaknesses: Very fragile, needs protection
```

#### 4. **Tank** 🛡️
```
Base Stats:
├─ HP: 150 (Highest!)
├─ Min Damage: 6
├─ Max Damage: 12
├─ Crit Chance: 10%
├─ Crit Multiplier: 1.5x
├─ Dodge Chance: 5%
└─ Defense: 18 (Highest!)

Special Ability: "Iron Wall"
├─ Cooldown: 4 turns
├─ Effect: Reflect 100% damage for 1 turn
└─ Best Used: When opponent uses special ability

Playstyle: Outlast opponents through sheer durability
Strengths: Massive HP pool, high defense
Weaknesses: Low damage output, slow wins
```

#### 5. **Trickster** 🃏
```
Base Stats:
├─ HP: 100 (Balanced)
├─ Min Damage: 8
├─ Max Damage: 16
├─ Crit Chance: 25%
├─ Crit Multiplier: 1.9x
├─ Dodge Chance: 12%
└─ Defense: 8

Special Ability: "Wild Card"
├─ Cooldown: 2 turns
├─ Effect: Activate wildcard effect (guaranteed)
└─ Best Used: When you need a lucky break

Playstyle: Unpredictable chaos, relies on RNG
Strengths: Versatile, high dodge, fun
Weaknesses: Inconsistent, RNG-dependent
```

### Character Progression

Characters gain experience and level up after battles:

```rust
// XP Curve (quadratic)
XP_Required(level) = 100 * level²

Level 1 → 2: 100 XP
Level 2 → 3: 400 XP
Level 3 → 4: 900 XP
Level 10 → 11: 10,000 XP
```

**Stat Growth per Level:**
```
HP: +5% of base HP (rounded up, min +1)
Min Damage: +10% (rounded up, min +1)
Max Damage: +10% (rounded up, min +1)

Example: Warrior Level 1 → 10
├─ HP: 120 → 174 (+54)
├─ Min Damage: 8 → 19 (+11)
└─ Max Damage: 15 → 35 (+20)
```

### Trait System

NFTs can have trait modifiers applied by the trait authority:

```rust
pub struct TraitBundle {
    pub rarity: u8,           // 1-5 stars
    pub attack_bps: i16,      // +500 = +5% attack
    pub defense_bps: i16,     // +300 = +3% defense
    pub crit_bps: i16,        // +200 = +2% crit chance
    pub nonce: i64,           // Unique ID
}
```

**Rarity Tiers:**
```
★☆☆☆☆ Common: -10% to +10% stats
★★☆☆☆ Uncommon: +5% to +15% stats
★★★☆☆ Rare: +10% to +25% stats
★★★★☆ Epic: +20% to +40% stats
★★★★★ Legendary: +35% to +60% stats
```

---

## 🎯 Battle System

### Battle Stances

Stances create a dynamic rock-paper-scissors meta-game:

#### 1. **Balanced** (Default)
```
Modifiers:
├─ Attack: 100% (no change)
├─ Defense: 100% (no change)
├─ Self-Damage: 0
└─ Counter: 0

Use When:
- Unknown opponent strategy
- Playing it safe
- Testing the waters
```

#### 2. **Aggressive** 🔥
```
Modifiers:
├─ Attack: 130% (+30% damage dealt)
├─ Defense: 150% (+50% damage taken)
├─ Self-Damage: 0
└─ Counter: 0

Use When:
- Opponent is defensive
- You have HP advantage
- Need to finish quickly

Countered By: Defensive, Counter
```

#### 3. **Defensive** 🛡️
```
Modifiers:
├─ Attack: 70% (-30% damage dealt)
├─ Defense: 50% (-50% damage taken)
├─ Self-Damage: 0
└─ Counter: 0

Use When:
- Low HP, need to survive
- Opponent is aggressive
- Waiting for cooldowns

Countered By: Aggressive (slowly)
```

#### 4. **Berserker** 💀
```
Modifiers:
├─ Attack: 200% (+100% damage dealt!)
├─ Defense: 100% (normal)
├─ Self-Damage: 25% of damage dealt
└─ Counter: 0

Use When:
- All-in finish move
- High HP, low opponent HP
- Feeling lucky

Risk: Self-damage can backfire
Countered By: Counter, Defensive
```

#### 5. **Counter** ⚡
```
Modifiers:
├─ Attack: 90% (-10% damage dealt)
├─ Defense: 100% (normal)
├─ Self-Damage: 0
└─ Counter: 40% (reflect 40% damage back)

Use When:
- Opponent is berserker
- You have lifesteal
- Baiting aggressive plays

Countered By: Defensive, Balanced
```

### Stance Selection (AI)

The game uses weighted random selection based on:

```rust
// Base weights
Balanced: 40%
Aggressive: 20%
Defensive: 20%
Berserker: 10%
Counter: 10%

// Class biases
Warrior: +5% Berserker, +3% Aggressive
Assassin: +6% Aggressive, +2% Counter
Mage: +4% Defensive, +3% Balanced
Tank: +8% Defensive, +2% Counter
Trickster: +4% Aggressive, +4% Balanced

// Player preference (if set)
Preferred Stance: 3x weight multiplier
```

**Example: Warrior with Aggressive preference**
```
Balanced: 40
Aggressive: 20 + 3 (class) = 23 * 3 (pref) = 69
Defensive: 20
Berserker: 10 + 5 (class) = 15
Counter: 10

Total: 154
Aggressive: 69/154 = 44.8% chance
```

### Special Abilities & Wildcards

**Wildcard System** (5% chance per turn):
```
40% - Damage Boost: +25% damage this turn
30% - Heal: Restore 10 HP
15% - Stun: Apply 2 DoT for 1 turn to opponent
10% - Nothing: No effect
5% - Critical Boost: 2x damage this turn
```

**Example Wildcard:**
```
Turn 5: Warrior attacks Mage
├─ Base Damage: 12
├─ Wildcard Triggered! (5% chance)
├─ Effect: Damage Boost (+25%)
├─ Final Damage: 12 * 1.25 = 15
└─ Mage takes 15 damage
```

---

## 🔄 Rounds-Based Execution

### Why Rounds?

**Gas Optimization:**
- Old approach: 8 turns = 8 transactions = ~1.2M compute units
- New approach: 3 rounds = 3 transactions = ~900k compute units
- **Savings: 67% fewer transactions, 25% lower CU**

### Round Structure

```
Battle (9 turns total)
│
├─ Round 1 (Turns 1-3)
│  ├─ Turn 1: Player A attacks
│  ├─ Turn 2: Player B attacks
│  └─ Turn 3: Player A attacks
│
├─ Round 2 (Turns 4-6)
│  ├─ Turn 4: Player B attacks
│  ├─ Turn 5: Player A attacks
│  └─ Turn 6: Player B attacks
│
└─ Round 3 (Turns 7-9)
   ├─ Turn 7: Player A attacks
   ├─ Turn 8: Player B attacks
   └─ Turn 9: Player A attacks (if needed)
```

### Round Execution

**Input:**
```rust
pub struct RoundMoves {
    pub moves: Vec<TurnMove>, // Length = 3
}

pub struct TurnMove {
    pub preference: i8,       // Stance preference (0-4)
    pub use_special: bool,    // Use special ability?
    pub use_wildcard: bool,   // Force wildcard? (Trickster)
}
```

**Example:**
```javascript
// Round 1 moves
const round1 = {
  moves: [
    { preference: 1, use_special: false, use_wildcard: false }, // Turn 1: Aggressive
    { preference: 2, use_special: false, use_wildcard: false }, // Turn 2: Defensive
    { preference: 0, use_special: true, use_wildcard: false },  // Turn 3: Balanced + Special
  ]
};

await program.methods
  .executeRound(round1)
  .accounts({ /* ... */ })
  .rpc();
```

**Execution Flow:**
```
1. Validate round state (max 3 rounds)
2. Validate moves (must have 3)
3. Consume 1 entropy seed (derive 3 random values)
4. Execute turn 1
   ├─ If battle ends, stop early
   └─ Update state
5. Execute turn 2 (if alive)
   └─ Update state
6. Execute turn 3 (if alive)
   └─ Update state
7. Increment round counter
8. Emit RoundCompleted event
```

**Early Termination:**
```
Round 1, Turn 2: Assassin crits for 45 damage
├─ Mage HP: 80 → 35
Round 1, Turn 3: Assassin crits again for 50 damage
├─ Mage HP: 35 → 0 (knockout!)
└─ Battle ends, Turn 4-9 skipped
```

### Battle State Tracking

```rust
pub struct Battle {
    // Round tracking
    pub current_round: u8,        // 1, 2, or 3
    pub rounds_completed: u8,     // 0, 1, or 2
    pub turns_in_current_round: u8, // 0, 1, 2, or 3

    // Legacy (still used for turn order)
    pub current_turn: u8,         // 1 or 2 (whose turn)
    pub turn_number: u64,         // Global turn count

    // Health
    pub player1_health: u64,
    pub player2_health: u64,

    // Status effects
    pub player1_dot_damage: u64,
    pub player1_dot_turns: u8,
    // ...
}
```

---

## 🎲 Entropy & Randomness

### VRF-Based Entropy System

BattleChain uses a **Verifiable Random Function (VRF)** oracle to provide cryptographically secure randomness:

```
┌─────────────────────────────────────┐
│         Entropy Pool                │
├─────────────────────────────────────┤
│                                     │
│  VRF Oracle submits seed batches    │
│  ├─ Seed: 32-byte random value     │
│  ├─ Start Index: 0, 100, 200...    │
│  ├─ Count: 100 values               │
│  └─ Signature: Proof of randomness  │
│                                     │
│  Battle consumes entropy:           │
│  ├─ 1 seed per turn (old method)   │
│  ├─ 1 seed per round (new method)  │
│  └─ Derive multiple values from 1  │
│                                     │
└─────────────────────────────────────┘
```

### Entropy Consumption (Optimized)

**Old Method (4 calls per turn):**
```rust
Turn 1:
├─ Consume seed #1 for stance
├─ Consume seed #2 for crit roll
├─ Consume seed #3 for dodge roll
└─ Consume seed #4 for wildcard roll
Total: 4 seeds per turn × 9 turns = 36 seeds per battle
```

**New Method (1 call per turn):**
```rust
Turn 1:
├─ Consume 1 seed
├─ Derive value 1 (tag=0): stance selection
├─ Derive value 2 (tag=1): crit roll
├─ Derive value 3 (tag=2): dodge roll
└─ Derive value 4 (tag=3): wildcard roll
Total: 1 seed per turn × 9 turns = 9 seeds per battle

Savings: 75% reduction in entropy consumption
```

### Derivation Function

```rust
fn derive_u64_from_seed_bytes(seed: &[u8; 32], tag: u8) -> u64 {
    // Hash seed with tag to get unique value
    let hash = hashv(&[seed, &[tag]]);

    // Convert first 8 bytes to u64
    let mut bytes = [0u8; 8];
    bytes.copy_from_slice(&hash[0..8]);
    u64::from_le_bytes(bytes)
}

// Usage
let seed = consume_entropy();
let stance_rng = derive_u64(seed, 0);
let crit_rng = derive_u64(seed, 1);
let dodge_rng = derive_u64(seed, 2);
let wildcard_rng = derive_u64(seed, 3);
```

---

## 💰 Economic Model

### Battle Stakes

Players wager tokens on battle outcomes:

```
Player 1 Stakes: 1 SOL
Player 2 Stakes: 1 SOL
Total Pool: 2 SOL

Outcome:
├─ Winner: 1.9 SOL (95%)
├─ Protocol Fee: 0.1 SOL (5%)
└─ Loser: 0 SOL
```

**Fee Structure:**
```
5% platform fee on stakes
├─ Treasury: 100%
└─ Used for: development, liquidity, rewards
```

### Supported Currencies

```
1. SOL (Native)
   ├─ Decimals: 9
   ├─ Min Stake: 0.1 SOL
   └─ Max Stake: 100 SOL

2. USDC (SPL Token)
   ├─ Decimals: 6
   ├─ Min Stake: 10 USDC
   └─ Max Stake: 10,000 USDC

3. USDT (SPL Token)
   ├─ Decimals: 6
   ├─ Min Stake: 10 USDT
   └─ Max Stake: 10,000 USDT
```

### Character Pricing

NFT characters can be minted or traded:

```
Minting (via program):
├─ Cost: Free (just gas)
├─ Limit: 1 per NFT
└─ Stats: Random within class bounds

Secondary Market (via marketplace):
├─ Price: User-determined
├─ Rarity affects value
└─ Level affects value

Estimated Values:
├─ Level 1, Common: 0.1-0.5 SOL
├─ Level 5, Rare: 1-5 SOL
├─ Level 10, Epic: 5-20 SOL
└─ Level 20, Legendary: 20-100+ SOL
```

---

## 🏗️ Technical Architecture

### Smart Contract Structure

```
programs/battlechain_v2/
├── src/
│   ├── lib.rs                 # Main program, 1800 lines
│   ├── constants.rs           # Game constants, FP_SCALE, etc.
│   ├── errors.rs              # 20+ error types
│   ├── events.rs              # 15+ event types
│   │
│   ├── state/
│   │   ├── config.rs          # Global config
│   │   ├── entropy.rs         # VRF entropy pool
│   │   ├── character.rs       # NFT character data
│   │   ├── battle.rs          # Battle state
│   │   └── offer.rs           # Match offers
│   │
│   ├── utils/
│   │   ├── math.rs            # Fixed-point math
│   │   ├── stance.rs          # Stance logic
│   │   ├── wildcard.rs        # Wildcard effects
│   │   └── levelup.rs         # XP and leveling
│   │
│   └── instructions/
│       └── round_execution.rs # Batched turn execution
│
└── Cargo.toml                 # Anchor 0.29.0
```

### Account Sizes

```
Config: ~100 bytes
EntropyPool: ~3,200 bytes (8 batches × 400 bytes)
Character: ~200 bytes
Progression: ~80 bytes
Offer: ~250 bytes
Request: ~150 bytes
Battle: ~350 bytes
```

### Compute Unit Budget

```
Instruction               | CU Usage | Notes
--------------------------|----------|------------------
create_config             | ~10k     | One-time setup
create_character          | ~20k     | NFT + stats init
create_battle_offer       | ~15k     | Escrow setup
join_battle_offer         | ~20k     | Escrow + validation
approve_challenger        | ~30k     | Battle init
execute_turn (single)     | ~100k    | Full combat logic
execute_round (3 turns)   | ~280k    | 3x combat + overhead
finalize_battle           | ~25k     | Payout distribution

Total per battle (rounds): ~900k CU (well under 1.4M limit)
```

---

## 👥 User Flows

### Flow 1: Create Character

```
1. Mint NFT (any Solana NFT)
   └─ Use Metaplex, Tensor, or custom minter

2. Call create_character_from_nft
   ├─ Input: NFT mint address, chosen class
   ├─ Program reads NFT ownership
   ├─ Initializes Character account
   └─ Initializes Progression account

3. (Optional) Apply Trait Bundle
   ├─ Requires trait authority signature
   ├─ Modifies base stats
   └─ Sets rarity tier

4. Character Ready
   └─ Can now create/join battles
```

### Flow 2: Battle Lifecycle (Rounds)

```
1. Player 1: Create Battle Offer
   ├─ Set stake amount (e.g., 1 SOL)
   ├─ Set constraints (level range, classes)
   ├─ Stake goes into escrow
   └─ Offer listed publicly

2. Player 2: Join Battle
   ├─ Find matching offer
   ├─ Meet requirements
   ├─ Stake same amount
   └─ Request created

3. Player 1: Approve Request
   ├─ Review challenger
   ├─ Approve or reject
   ├─ If approved, battle initializes
   └─ Starting player chosen randomly

4. Execute Battle (3 Rounds)
   ├─ Round 1: Submit 3 moves, execute
   ├─ Round 2: Submit 3 moves, execute
   └─ Round 3: Submit 3 moves, execute

   OR use AI (future):
   ├─ Set preferences once
   └─ AI plays for you

5. Battle Resolves
   ├─ Winner determined automatically
   ├─ Loser's character loses 1 life
   └─ Both gain XP

6. Claim Rewards
   ├─ Winner claims stake (1.9 SOL)
   ├─ Loser claims nothing
   └─ Protocol keeps fee (0.1 SOL)
```

### Flow 3: Character Progression

```
After Each Battle:
├─ Gain XP (50 for loss, 150 for win)
├─ Check if level up
└─ If level up:
    ├─ Increase HP (+5%)
    ├─ Increase damage (+10%)
    ├─ Reset HP to max
    └─ Emit level up event

After Every 3 Losses:
├─ Character loses 1 life
├─ If 0 lives remaining:
│   └─ Character "dies" (can be revived later)
└─ Else: continue playing
```

---

## 📚 Smart Contract API

### Instructions

#### Character Management

```rust
// Create character from NFT
create_character_from_nft(class: CharacterClass)

// Apply trait modifiers
apply_trait_bundle(bundle: TraitBundle)
```

#### Battle Offers

```rust
// Create battle offer
create_battle_offer(
    offer_nonce: u64,
    currency: Currency,
    stake_amount: u64,
    min_level: u16,
    max_level: u16,
    allowed_classes: Vec<CharacterClass>,
    auto_approve: bool,
    start_ts: i64,
    inactivity_timeout: i64,
)

// Join battle offer
join_battle_offer(stake_amount: u64)

// Approve or reject request
approve_challenger()
reject_request()

// Withdraw offer
withdraw_offer()
```

#### Battle Execution

```rust
// Execute single turn (legacy)
execute_turn(
    preference: i8,
    use_special: bool,
    use_wildcard: bool,
)

// Execute round (3 turns, optimized)
execute_round(round_moves: RoundMoves)

// Finalize and distribute rewards
finalize_battle()

// Claim timeout if opponent inactive
claim_timeout()
```

#### Administration

```rust
// Create global config
create_config(
    fee_bps: u16,
    inactivity_timeout: i64,
    spl_whitelist: Vec<Pubkey>,
    trait_authority: Pubkey,
)

// Create entropy pool
create_entropy_pool(vrf_oracle: Pubkey)

// Refill entropy
refill_seed_batch(seed: [u8; 32], start: u64, count: u32)
```

---

## 🎯 Strategy Guide

### Class Matchups

```
Warrior vs Assassin:
├─ Warrior Advantage: Higher HP survives burst
├─ Assassin Advantage: Can one-shot with crits
└─ Winner: 50/50, depends on RNG

Warrior vs Mage:
├─ Warrior Advantage: More HP, consistent damage
├─ Mage Advantage: Arcane blast ignores defense
└─ Winner: Warrior 60/40

Warrior vs Tank:
├─ Warrior Advantage: Higher damage
├─ Tank Advantage: Outlasts in long battle
└─ Winner: Tank 55/45 (slow grind)

Assassin vs Mage:
├─ Assassin Advantage: One-shot potential
├─ Mage Advantage: Consistent damage, arcane blast
└─ Winner: Assassin 65/35 (glass cannon vs glass cannon)

Tank vs Trickster:
├─ Tank Advantage: High defense, HP
├─ Trickster Advantage: Wildcards can turn tide
└─ Winner: Tank 70/30 (slow but steady)
```

### Meta Strategies

**Early Game (Turns 1-3):**
- Scout opponent's stance preference
- Save special abilities
- Build combo count (for crits)
- Avoid berserker (too risky)

**Mid Game (Turns 4-6):**
- Use special abilities strategically
- Counter opponent's stance
- Manage HP carefully
- Consider using wildcard if Trickster

**Late Game (Turns 7-9):**
- Calculate lethal damage
- Use berserker if you can afford self-damage
- Defensive if low HP
- All-in with specials

---

## 🔮 Future Roadmap

### Planned Features

**Season 2: Team Battles**
- 2v2 or 3v3 matches
- Combo abilities between teammates
- Shared HP pool or individual

**Season 3: Tournament Mode**
- Bracket-style tournaments
- Entry fees, prize pools
- Leaderboards

**Season 4: PvE Campaigns**
- Story mode vs AI bosses
- Earn rare items
- Unlock new abilities

**Season 5: Cross-Chain**
- Bridge to Ethereum, Polygon
- Multi-chain tournaments
- Unified leaderboards

---

## 📊 Analytics & Stats

### Track Your Performance

```
Character Stats:
├─ Win Rate: 60% (12W-8L)
├─ Average Damage Per Turn: 18
├─ Crit Rate: 32% (actual vs expected)
├─ Dodge Rate: 14%
└─ Most Used Stance: Aggressive (45%)

Battle History:
├─ Total Battles: 20
├─ Wins: 12
├─ Losses: 8
├─ Total Earnings: 8.5 SOL
└─ Current Streak: 3W

Leaderboard Rank:
├─ Global: #142 (Top 5%)
├─ Class (Warrior): #23
└─ Level Tier (6-10): #8
```

---

## 🔒 Security & Fairness

### Provably Fair Combat

✅ **All RNG on-chain**: VRF oracle provides verifiable randomness
✅ **Deterministic outcomes**: Same inputs = same outputs
✅ **No hidden state**: All battle state visible on-chain
✅ **Immutable logic**: Smart contract code cannot change mid-battle
✅ **Cryptographic signatures**: VRF proofs prevent manipulation

### Anti-Cheat Measures

✅ **NFT ownership verified**: Must own NFT to use character
✅ **Escrow system**: Stakes locked until battle ends
✅ **Timeout protection**: Inactive players auto-lose
✅ **No front-running**: Moves committed in batches
✅ **Stat validation**: Character stats validated on-chain

---

## 📞 Support & Community

- **Documentation**: [docs.battlechain.io](https://battlechain.io)
- **Discord**: [discord.gg/battlechain](https://discord.gg)
- **Twitter**: [@BattleChainSol](https://twitter.com)
- **GitHub**: [github.com/battlechain](https://github.com)

---

**Built with ⚔️ on Solana**
**Version**: 2.0.0
**Last Updated**: 2025-11-12
