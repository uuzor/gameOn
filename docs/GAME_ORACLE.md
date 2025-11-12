# Game Results Oracle Network

## 📋 Table of Contents
- [Overview](#overview)
- [Architecture](#architecture)
- [Game Registration](#game-registration)
- [Result Submission](#result-submission)
- [Data Querying](#data-querying)
- [Economic Model](#economic-model)
- [Reputation System](#reputation-system)
- [Technical Specifications](#technical-specifications)
- [Developer SDK](#developer-sdk)
- [Use Cases](#use-cases)

---

## 🎯 Overview

The **Game Results Oracle Network** is an on-chain data marketplace where game developers submit verified game results, and other applications (like prediction markets, analytics platforms, and tournaments) pay to query that data.

### The Problem

Current gaming ecosystems have fragmented data:
- ❌ Game results locked in centralized databases
- ❌ No way to verify authenticity
- ❌ Developers can't monetize their data
- ❌ Prediction markets rely on manual oracles
- ❌ Cross-game analytics impossible

### The Solution

```
┌──────────────────────────────────────────────────────┐
│          Game Results Oracle Network                  │
├──────────────────────────────────────────────────────┤
│                                                        │
│  Game Developers:                                      │
│  ├─ Register game on-chain                            │
│  ├─ Submit results with cryptographic signature       │
│  ├─ Earn fees every time data is queried              │
│  └─ Build reputation for data quality                 │
│                                                        │
│  Data Consumers (Prediction Markets, Analytics):       │
│  ├─ Query game results (pay per query)                │
│  ├─ Verify authenticity via signatures                │
│  ├─ Integrate via simple API                          │
│  └─ Compose data across multiple games                │
│                                                        │
│  Network Benefits:                                     │
│  ├─ Permissionless: Anyone can submit/query           │
│  ├─ Composable: Any contract can use                  │
│  ├─ Incentivized: Pay per use                         │
│  └─ Verifiable: Cryptographic proofs                  │
│                                                        │
└──────────────────────────────────────────────────────┘
```

### Key Features

✅ **On-Chain Data Marketplace**: Game results stored on Solana
✅ **Pay-Per-Query Model**: Micropayments for data access
✅ **Cryptographic Verification**: Signed results prevent fraud
✅ **Reputation Staking**: Developers stake tokens for credibility
✅ **Permissionless**: Any game can join, any app can query
✅ **Standardized Schema**: Easy integration across games
✅ **Developer Rewards**: 90% of query fees go to game devs
✅ **Cross-Game Analytics**: Query data from multiple games

---

## 🏗️ Architecture

### System Components

```
┌─────────────────────────────────────────────────────┐
│                     Oracle Layer                     │
├─────────────────────────────────────────────────────┤
│                                                       │
│  1. Game Registry                                    │
│     ├─ Register new game                             │
│     ├─ Stake tokens (anti-spam)                      │
│     ├─ Set metadata (name, type, etc.)               │
│     └─ Earn reputation over time                     │
│                                                       │
│  2. Result Storage                                   │
│     ├─ Submit result (signed by developer)           │
│     ├─ Store on-chain (PDA account)                  │
│     ├─ Index by game_id + result_id                  │
│     └─ Emit event for indexers                       │
│                                                       │
│  3. Query Interface                                  │
│     ├─ Query result by ID                            │
│     ├─ Pay query fee (0.001 SOL)                     │
│     ├─ Verify signature                              │
│     └─ Return result data                            │
│                                                       │
│  4. Fee Distribution                                 │
│     ├─ 90% to game developer                         │
│     ├─ 10% to protocol treasury                      │
│     └─ Auto-distributed on query                     │
│                                                       │
│  5. Reputation System                                │
│     ├─ Track total submissions                       │
│     ├─ Track total queries (demand signal)           │
│     ├─ Track disputes                                │
│     └─ Calculate reputation score                    │
│                                                       │
└─────────────────────────────────────────────────────┘
```

### Data Flow

```
Game Developer Side:
1. Battle ends in external game
   └─ Winner determined: Player1 or Player2

2. Game backend signs result
   ├─ Create result struct
   ├─ Sign with developer private key
   └─ Generate 64-byte signature

3. Submit to oracle contract
   ├─ Call submit_result instruction
   ├─ Pay ~0.02 SOL (storage cost)
   └─ Result stored on-chain

4. Event emitted
   ├─ Indexers pick up new result
   └─ Available for querying


Prediction Market Side:
1. Battle scheduled
   ├─ Market created
   └─ Link to external game result ID

2. Players bet on outcome
   ├─ YES = Player1 wins
   └─ NO = Player2 wins

3. Battle completes off-chain
   ├─ Developer submits result
   └─ Result appears on-chain

4. Market resolution
   ├─ Query oracle for result
   ├─ Pay 0.001 SOL fee
   ├─ Developer earns 0.0009 SOL
   └─ Resolve market based on result

5. Distribute winnings
   ├─ Winners claim payout
   └─ Market settles
```

---

## 📝 Game Registration

### Register Your Game

```rust
pub struct RegisterGame {
    pub name: String,           // "Solana Poker"
    pub game_type: GameType,    // Strategy, FPS, Battle, etc.
    pub developer: Pubkey,      // Your wallet
    pub stake_amount: u64,      // Anti-spam stake (min 10 SOL)
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy)]
pub enum GameType {
    Battle,      // 1v1 or team battles
    Strategy,    // Turn-based strategy
    FPS,         // First-person shooter
    MOBA,        // Multiplayer online battle arena
    Racing,      // Racing games
    Sports,      // Sports simulations
    Card,        // Card games (poker, etc.)
    Other,       // Catch-all
}
```

### Example Registration

```typescript
import * as anchor from "@coral-xyz/anchor";

const gameRegistry = await program.methods
  .registerGame(
    "Solana Poker Championship",
    { card: {} }, // GameType enum
    new anchor.BN(10_000_000_000) // 10 SOL stake
  )
  .accounts({
    game: gamePDA,
    developer: developer.publicKey,
    systemProgram: SystemProgram.programId,
  })
  .signers([developer])
  .rpc();

console.log("Game registered:", gameRegistry);
```

### Registration Requirements

```
Minimum Stake: 10 SOL
├─ Prevents spam registrations
├─ Refundable when unregistering
└─ Slashable if fraud detected

Game Metadata:
├─ Name: Max 64 characters
├─ Type: One of 8 categories
├─ Developer: Verified Solana address
└─ Website: Optional URL

Approval:
├─ Permissionless (no approval needed)
├─ Instant registration
└─ Start submitting results immediately
```

---

## 📤 Result Submission

### Standard Result Schema

All games use this standardized format:

```rust
pub struct GameResult {
    pub result_id: u64,          // Unique ID for this result
    pub game_id: u64,            // Your registered game ID
    pub player1: Pubkey,         // Participant 1 wallet
    pub player2: Pubkey,         // Participant 2 wallet
    pub winner: Pubkey,          // Winner's wallet
    pub timestamp: i64,          // When game ended
    pub metadata: Vec<u8>,       // Flexible extra data
    pub signature: [u8; 64],     // Developer signature
}
```

### Metadata Format

The `metadata` field is flexible JSON:

```json
{
  "game_mode": "ranked",
  "duration_seconds": 1837,
  "player1_score": 245,
  "player2_score": 198,
  "map": "desert_storm",
  "rounds_played": 15,
  "mvp": "player1",
  "custom": {
    "kills_player1": 18,
    "kills_player2": 12,
    "accuracy_player1": 0.68,
    "accuracy_player2": 0.54
  }
}
```

### Signing Results

**Step 1: Create Message**
```typescript
const message = {
  result_id: 1001,
  game_id: 42,
  player1: player1Pubkey,
  player2: player2Pubkey,
  winner: player1Pubkey,
  timestamp: Date.now() / 1000,
  metadata: JSON.stringify(metadata),
};
```

**Step 2: Sign with Developer Key**
```typescript
import nacl from "tweetnacl";

const messageBytes = Buffer.from(JSON.stringify(message));
const signature = nacl.sign.detached(messageBytes, developer.secretKey);

// signature is 64 bytes
```

**Step 3: Submit to Oracle**
```typescript
await program.methods
  .submitResult(
    new anchor.BN(1001), // result_id
    player1Pubkey,
    player2Pubkey,
    player1Pubkey, // winner
    metadataBytes,
    Array.from(signature)
  )
  .accounts({
    game: gamePDA,
    result: resultPDA,
    developer: developer.publicKey,
    systemProgram: SystemProgram.programId,
  })
  .signers([developer])
  .rpc();
```

### Submission Costs

```
Storage Cost: ~0.02 SOL per result
├─ Result account: ~300 bytes
├─ Metadata: Variable (up to 1 KB)
└─ Rent-exempt deposit

Network Fees: ~0.000005 SOL
├─ Transaction signature fee
└─ Compute units

Total Cost: ~0.020005 SOL per result

ROI Example:
├─ Submit 1 result: -0.02 SOL
├─ Queried 100 times: +0.09 SOL (100 × 0.001 × 0.9)
├─ Net Profit: +0.07 SOL
└─ Break-even: 23 queries
```

---

## 🔍 Data Querying

### Query a Result

```typescript
// Consumer (Prediction Market) queries result
const queryFee = new anchor.BN(1_000_000); // 0.001 SOL

const resultData = await program.methods
  .queryResult(new anchor.BN(1001)) // result_id
  .accounts({
    game: gamePDA,
    result: resultPDA,
    developer: developerPubkey,
    queryer: market.publicKey,
    systemProgram: SystemProgram.programId,
  })
  .remainingAccounts([
    { pubkey: protocolTreasury, isWritable: true, isSigner: false },
  ])
  .rpc();

// Fee distribution happens automatically:
// - 0.0009 SOL → game developer
// - 0.0001 SOL → protocol treasury
```

### Verify Signature

```typescript
import nacl from "tweetnacl";

function verifyResult(result: GameResult, developerPubkey: PublicKey): boolean {
  // Reconstruct message
  const message = {
    result_id: result.result_id,
    game_id: result.game_id,
    player1: result.player1,
    player2: result.player2,
    winner: result.winner,
    timestamp: result.timestamp,
    metadata: Buffer.from(result.metadata).toString(),
  };

  const messageBytes = Buffer.from(JSON.stringify(message));

  // Verify signature
  return nacl.sign.detached.verify(
    messageBytes,
    result.signature,
    developerPubkey.toBytes()
  );
}

// Usage
if (!verifyResult(result, game.developer)) {
  throw new Error("Invalid signature! Result may be fraudulent.");
}
```

### Batch Queries

Query multiple results efficiently:

```typescript
const results = await Promise.all([
  program.account.gameResult.fetch(resultPDA1),
  program.account.gameResult.fetch(resultPDA2),
  program.account.gameResult.fetch(resultPDA3),
]);

// Process batch
results.forEach((result, index) => {
  console.log(`Result ${index + 1}:`, {
    winner: result.winner.toString(),
    timestamp: new Date(result.timestamp * 1000),
  });
});
```

---

## 💰 Economic Model

### Fee Structure

```
Query Fee: 0.001 SOL (fixed)
├─ Game Developer: 0.0009 SOL (90%)
├─ Protocol Treasury: 0.0001 SOL (10%)
└─ Total: 0.001 SOL

Stake Requirement: 10 SOL minimum
├─ Purpose: Anti-spam, reputation
├─ Refundable: Yes (when unregistering)
└─ Slashable: Yes (if fraudulent)
```

### Developer Revenue Projections

**Small Game (100 results/month, 10 queries each):**
```
Submissions: 100 results × 0.02 SOL = -2 SOL cost
Queries: 1,000 queries × 0.0009 SOL = +0.9 SOL revenue
Net: -1.1 SOL/month (not profitable yet)
```

**Medium Game (500 results/month, 50 queries each):**
```
Submissions: 500 × 0.02 = -10 SOL cost
Queries: 25,000 × 0.0009 = +22.5 SOL revenue
Net: +12.5 SOL/month (~$1,250 at $100/SOL)
```

**Large Game (5,000 results/month, 200 queries each):**
```
Submissions: 5,000 × 0.02 = -100 SOL cost
Queries: 1,000,000 × 0.0009 = +900 SOL revenue
Net: +800 SOL/month (~$80,000 at $100/SOL)
```

**Viral Game (50,000 results/month, 1,000 queries each):**
```
Submissions: 50,000 × 0.02 = -1,000 SOL cost
Queries: 50,000,000 × 0.0009 = +45,000 SOL revenue
Net: +44,000 SOL/month (~$4.4M at $100/SOL)
```

### Protocol Revenue

With 100 registered games averaging 500 results/month, 50 queries each:

```
Monthly Queries: 100 × 500 × 50 = 2,500,000
Protocol Fee (10%): 2,500,000 × 0.0001 SOL = 250 SOL/month
Annual Revenue: 3,000 SOL/year (~$300k at $100/SOL)
```

---

## 🏆 Reputation System

### Reputation Score Calculation

```rust
pub struct GameReputation {
    pub total_submissions: u64,   // Lifetime results submitted
    pub total_queries: u64,        // Lifetime queries received
    pub total_disputes: u64,       // Times flagged as fraudulent
    pub stake_amount: u64,         // Current stake (higher = better)
    pub registration_date: i64,    // Age of account
}

// Reputation formula
fn calculate_reputation(game: &GameReputation) -> u64 {
    let query_score = game.total_queries / 100; // 1 point per 100 queries
    let submission_score = game.total_submissions / 10; // 1 point per 10 submissions
    let stake_score = (game.stake_amount / 1_000_000_000) as u64; // 1 point per SOL
    let age_score = ((Clock::get()?.unix_timestamp - game.registration_date) / 86400) as u64; // 1 point per day
    let dispute_penalty = game.total_disputes * 100; // -100 points per dispute

    (query_score + submission_score + stake_score + age_score)
        .saturating_sub(dispute_penalty)
}
```

**Example Scores:**
```
New Game (Day 1):
├─ Queries: 0 → 0 points
├─ Submissions: 10 → 1 point
├─ Stake: 10 SOL → 10 points
├─ Age: 1 day → 1 point
├─ Disputes: 0 → 0 penalty
└─ Total: 12 reputation

Established Game (1 year):
├─ Queries: 100,000 → 1,000 points
├─ Submissions: 5,000 → 500 points
├─ Stake: 50 SOL → 50 points
├─ Age: 365 days → 365 points
├─ Disputes: 2 → -200 penalty
└─ Total: 1,715 reputation

Trusted Game (5 years):
├─ Queries: 10,000,000 → 100,000 points
├─ Submissions: 500,000 → 50,000 points
├─ Stake: 500 SOL → 500 points
├─ Age: 1,825 days → 1,825 points
├─ Disputes: 5 → -500 penalty
└─ Total: 151,825 reputation
```

### Reputation Benefits

Higher reputation = more trust = more queries:

```
Low Reputation (0-100):
├─ New/unproven games
├─ Prediction markets hesitant to use
└─ Need to build track record

Medium Reputation (100-1,000):
├─ Established games
├─ Used by most prediction markets
└─ Reliable data source

High Reputation (1,000-10,000):
├─ Major games
├─ Preferred by all consumers
└─ Premium data source

Elite Reputation (10,000+):
├─ Industry leaders
├─ Exclusive partnerships
└─ Maximum query volume
```

---

## 🛠️ Technical Specifications

### Account Structures

**GameRegistry Account** (~200 bytes)
```rust
#[account]
pub struct GameRegistry {
    pub game_id: u64,
    pub developer: Pubkey,
    pub name: String,             // Max 64 chars
    pub game_type: GameType,
    pub reputation_score: u64,
    pub total_submissions: u64,
    pub total_queries: u64,
    pub total_fees_earned: u64,
    pub stake_amount: u64,
    pub is_active: bool,
    pub created_at: i64,
    pub bump: u8,
}
```

**GameResult Account** (~300-1,300 bytes)
```rust
#[account]
pub struct GameResult {
    pub result_id: u64,
    pub game_id: u64,
    pub player1: Pubkey,
    pub player2: Pubkey,
    pub winner: Pubkey,
    pub timestamp: i64,
    pub metadata: Vec<u8>,        // Max 1 KB
    pub signature: [u8; 64],
    pub query_count: u64,
    pub total_fees_earned: u64,
    pub submitted_at: i64,
    pub bump: u8,
}
```

### PDA Seeds

```rust
// Game Registry PDA
["game", game_id.to_le_bytes()]

// Game Result PDA
["result", game_id.to_le_bytes(), result_id.to_le_bytes()]
```

### Instructions

```rust
// Register new game
register_game(
    name: String,
    game_type: GameType,
    stake_amount: u64,
)

// Submit game result
submit_result(
    result_id: u64,
    player1: Pubkey,
    player2: Pubkey,
    winner: Pubkey,
    metadata: Vec<u8>,
    signature: [u8; 64],
)

// Query game result (pay fee)
query_result(result_id: u64)

// Unregister game (get stake back)
unregister_game()

// Increase stake (improve reputation)
increase_stake(amount: u64)

// Dispute result (if fraudulent)
dispute_result(result_id: u64, reason: String)
```

---

## 💻 Developer SDK

### JavaScript/TypeScript SDK

```typescript
import { GameOracle } from "@battlechain/game-oracle-sdk";

// Initialize SDK
const oracle = new GameOracle(connection, wallet);

// Register your game
const game = await oracle.registerGame({
  name: "My Awesome Game",
  gameType: "FPS",
  stakeAmount: 10, // SOL
});

// Submit result
const result = await oracle.submitResult({
  gameId: game.gameId,
  resultId: 1001,
  player1: player1Pubkey,
  player2: player2Pubkey,
  winner: player1Pubkey,
  metadata: {
    score_player1: 25,
    score_player2: 18,
    duration: 1200,
  },
});

// Query result (as consumer)
const data = await oracle.queryResult({
  gameId: 42,
  resultId: 1001,
});

console.log("Winner:", data.winner);
console.log("Verified:", data.verified);
```

### Rust SDK

```rust
use game_oracle::prelude::*;

// Submit result from game backend
pub fn submit_game_result(
    ctx: Context<SubmitResult>,
    result: GameResultData,
) -> Result<()> {
    let oracle = GameOracle::new(ctx.accounts.oracle_program);

    oracle.submit_result(
        ctx.accounts.game,
        result.player1,
        result.player2,
        result.winner,
        result.metadata,
        result.signature,
    )?;

    Ok(())
}
```

### Python SDK (Backend)

```python
from game_oracle import OracleClient

# Initialize
oracle = OracleClient(
    rpc_url="https://api.devnet.solana.com",
    private_key=developer_key
)

# Submit result from game server
result = oracle.submit_result(
    game_id=42,
    result_id=1001,
    player1=player1_pubkey,
    player2=player2_pubkey,
    winner=player1_pubkey,
    metadata={
        "kills_p1": 18,
        "kills_p2": 12,
        "map": "dust2",
    }
)

print(f"Result submitted: {result.signature}")
```

---

## 🎯 Use Cases

### 1. Prediction Markets

```
Flow:
1. Prediction market creates bet on external game
   └─ "Will Player1 beat Player2 in Solana Poker?"

2. Users bet YES or NO

3. Game completes off-chain

4. Developer submits result to oracle

5. Market queries oracle
   ├─ Pay 0.001 SOL
   ├─ Get verified result
   └─ Resolve market

6. Winners claim payouts

Benefits:
✅ Instant resolution (no manual oracle)
✅ Provably fair (cryptographic signatures)
✅ Automatic (no human intervention)
✅ Scalable (handle 1000s of markets)
```

### 2. Tournament Platforms

```
Flow:
1. Tournament organizer creates bracket

2. Games played off-chain

3. Each game result submitted to oracle

4. Tournament platform queries results
   ├─ Verify all signatures
   └─ Build tournament bracket

5. Advance winners automatically

6. Distribute prizes based on on-chain data

Benefits:
✅ Trustless (no centralized tournament admins)
✅ Transparent (all results on-chain)
✅ Automated (smart contract handles advancement)
✅ Verifiable (community can audit)
```

### 3. Analytics Platforms

```
Flow:
1. Analytics platform subscribes to oracle events

2. Index all results from multiple games

3. Build cross-game leaderboards
   ├─ Best players across all FPS games
   ├─ Most wins in card games
   └─ Highest win rates by game type

4. Sell premium analytics to users

Benefits:
✅ Cross-game insights (impossible with siloed data)
✅ Real-time (on-chain events)
✅ Verifiable (cryptographic proofs)
✅ Monetizable (charge for insights)
```

### 4. Esports Streaming

```
Flow:
1. Streamer connects wallet to streaming platform

2. Stream shows live matches

3. Results submitted to oracle in real-time

4. Overlay shows on-chain verified results
   ├─ Winner
   ├─ Score
   └─ Game stats

5. Viewers can verify results themselves

Benefits:
✅ Trust (no fake results)
✅ Engagement (viewers verify on-chain)
✅ Monetization (pay for historical data)
```

---

## 🔐 Security & Fraud Prevention

### Signature Verification

All results are cryptographically signed:

```
Signature Chain:
1. Developer creates result data
2. Developer signs with private key
3. Signature stored on-chain with result
4. Consumers verify with developer public key
5. Invalid signatures = flagged as fraud
```

### Dispute Mechanism

If fraudulent results detected:

```
Dispute Flow:
1. Anyone can flag result as fraudulent
   ├─ Submit dispute with evidence
   └─ Pay dispute fee (0.1 SOL)

2. Community votes on validity
   ├─ 48-hour voting period
   ├─ Token-weighted votes
   └─ 51% majority needed

3. If fraud confirmed:
   ├─ Developer stake slashed (50%)
   ├─ Result marked as invalid
   ├─ Reputation penalty applied
   └─ Disputer gets reward

4. If fraud not confirmed:
   ├─ Dispute fee forfeit
   ├─ Developer keeps stake
   └─ Result stays valid
```

### Anti-Spam Measures

```
Stake Requirement:
├─ Minimum 10 SOL to register
├─ Prevents spam registrations
└─ Economic deterrent

Rate Limiting:
├─ Max 1,000 results per game per day
├─ Prevents blockchain bloat
└─ Encourages quality over quantity

Storage Costs:
├─ Developer pays ~0.02 SOL per result
├─ Economic cost to spam
└─ Incentivizes only valuable data
```

---

## 📊 Oracle Economics

### Supply & Demand Dynamics

```
Supply Side (Game Developers):
├─ More submissions = more potential revenue
├─ Higher quality = more queries
├─ Build reputation over time
└─ Earn passive income from historical data

Demand Side (Consumers):
├─ Pay per query (0.001 SOL)
├─ Want reliable data (high reputation games)
├─ Prefer fresh data (recent results)
└─ Value verifiable data (signatures)

Network Effects:
├─ More games = more data = more consumers
├─ More consumers = more revenue = more games
├─ Positive feedback loop
└─ First-mover advantage
```

---

## 🚀 Future Roadmap

### Phase 1: MVP (Current)
- [x] Basic registry
- [x] Result submission
- [x] Query interface
- [x] Signature verification

### Phase 2: Enhanced (Month 2)
- [ ] Reputation system
- [ ] Dispute mechanism
- [ ] Batch queries
- [ ] Historical data API

### Phase 3: Advanced (Month 3-6)
- [ ] Real-time subscriptions
- [ ] Aggregated statistics
- [ ] Cross-game analytics
- [ ] Developer dashboard

### Phase 4: Ecosystem (Month 6+)
- [ ] Partner integrations
- [ ] Indexer network
- [ ] Data marketplace UI
- [ ] DAO governance

---

**Built on Solana**
**Version**: 1.0.0
**Last Updated**: 2025-11-12
