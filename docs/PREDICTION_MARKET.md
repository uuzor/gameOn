# BattleChain Prediction Market System

## 📋 Table of Contents
- [Overview](#overview)
- [System Architecture](#system-architecture)
- [AMM Mechanics](#amm-mechanics)
- [Scheduled Matches](#scheduled-matches)
- [Token Support](#token-support)
- [User Flows](#user-flows)
- [Economic Model](#economic-model)
- [Implementation Phases](#implementation-phases)
- [Technical Specifications](#technical-specifications)

---

## 🎯 Overview

The BattleChain Prediction Market is an **AMM-based binary prediction market** that allows users to bet on battle outcomes. Players can buy YES/NO shares on scheduled matches, with prices determined by a constant product algorithm similar to Uniswap.

### Key Features
- ✅ **AMM-based trading** - Instant liquidity, no order matching needed
- ✅ **Scheduled matches** - Betting period before battles start
- ✅ **Multi-token support** - SOL, USDT, USDC
- ✅ **Auto-resolution** - Results from BattleChain contracts
- ✅ **User-provided liquidity** - Anyone can be a market maker
- ✅ **Provably fair** - All on-chain, no manipulation

---

## 🏗️ System Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    Prediction Market System                  │
├─────────────────────────────────────────────────────────────┤
│                                                               │
│  1. Match Scheduling                                          │
│     ├─ Create scheduled match                                │
│     ├─ Set betting period (e.g., 1 hour before battle)       │
│     └─ Link to BattleChain battle                            │
│                                                               │
│  2. Market Creation                                           │
│     ├─ Initialize YES/NO pools                               │
│     ├─ Set initial odds (50/50 or weighted)                  │
│     └─ Accept SOL/USDT/USDC                                  │
│                                                               │
│  3. Trading (AMM)                                             │
│     ├─ Buy YES shares: x * y = k                             │
│     ├─ Buy NO shares: x * y = k                              │
│     ├─ Dynamic pricing based on pool ratios                  │
│     └─ Slippage protection                                   │
│                                                               │
│  4. Battle Execution                                          │
│     ├─ Trading freezes when battle starts                    │
│     ├─ Battle executes on BattleChain                        │
│     └─ Result recorded on-chain                              │
│                                                               │
│  5. Market Resolution                                         │
│     ├─ Read battle result from BattleChain                   │
│     ├─ Resolve market (YES or NO wins)                       │
│     ├─ Distribute winnings to winners                        │
│     └─ Return liquidity to LPs                               │
│                                                               │
└─────────────────────────────────────────────────────────────┘
```

---

## 🔄 AMM Mechanics

### Constant Product Formula

Our AMM uses the **constant product formula**: `x * y = k`

Where:
- `x` = YES pool reserves
- `y` = NO pool reserves
- `k` = constant product

### Example Trade

**Initial State:**
```
YES pool: 100 SOL
NO pool: 100 SOL
k = 100 * 100 = 10,000
Implied probability: 50% YES, 50% NO
```

**User buys 10 SOL of YES:**
```
User pays: 10 SOL
YES pool becomes: 110 SOL
NO pool becomes: 10,000 / 110 = 90.91 SOL
User receives: 100 - 90.91 = 9.09 NO shares burned
                (equivalent to getting YES position)

New implied probability:
- YES: 110 / (110 + 90.91) = 54.7%
- NO: 90.91 / (110 + 90.91) = 45.3%
```

### Price Impact

The more you buy, the worse your price:
```
Buy Amount | Price Impact | Effective Price
-----------|--------------|----------------
1 SOL      | 0.5%         | 0.505 SOL/share
5 SOL      | 2.4%         | 0.524 SOL/share
10 SOL     | 4.8%         | 0.548 SOL/share
50 SOL     | 22.2%        | 0.722 SOL/share
```

### Slippage Protection

Users set maximum slippage tolerance:
```rust
pub struct TradeParams {
    pub amount_in: u64,
    pub min_shares_out: u64,  // Minimum shares user will accept
    pub max_slippage_bps: u16, // e.g., 100 = 1%
}
```

---

## ⏱️ Scheduled Matches

### Match Lifecycle

```
┌──────────────┐
│   Created    │  Match announced, betting opens
│   [Open]     │  Users can add liquidity, buy shares
└──────┬───────┘
       │ betting_end_time reached
       ▼
┌──────────────┐
│   Locked     │  Betting closed, battle ready
│  [Pending]   │  No trading allowed
└──────┬───────┘
       │ Battle executes
       ▼
┌──────────────┐
│   Resolved   │  Result available
│  [Finished]  │  Winners can claim
└──────────────┘
```

### Timing Configuration

```rust
pub struct ScheduledMatch {
    pub match_id: u64,
    pub player1: Pubkey,
    pub player2: Pubkey,
    pub battle_offer: Pubkey,        // Link to BattleChain offer

    // Timing
    pub created_at: i64,
    pub betting_start: i64,          // When betting opens
    pub betting_end: i64,            // When betting closes
    pub estimated_battle_time: i64,  // When battle is expected

    // Market reference
    pub market: Pubkey,

    // Status
    pub status: MatchStatus,         // Open, Locked, Resolved
}
```

**Example Timeline:**
```
T=0:     Match created, betting opens
T=3600:  Betting closes (1 hour of betting)
T=3601:  Battle can start
T=3900:  Battle completes (5 minutes)
T=3900+: Winners claim rewards
```

---

## 💰 Token Support

### Supported Tokens

| Token | Mint Address | Decimals | Use Case |
|-------|--------------|----------|----------|
| SOL   | Native       | 9        | Primary token |
| USDC  | EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v | 6 | Stablecoin betting |
| USDT  | Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB | 6 | Stablecoin betting |

### Multi-Token Markets

Each market can use ONE currency:
```rust
pub enum Currency {
    SOL,
    USDC,
    USDT,
}

pub struct Market {
    pub currency: Currency,
    pub yes_pool: u64,      // In currency decimals
    pub no_pool: u64,       // In currency decimals
    // ...
}
```

### Token Accounts
```
Market (PDA)
├─ SOL balance (native)
│
├─ USDC vault (ATA)
│  └─ Token Account owned by market PDA
│
└─ USDT vault (ATA)
   └─ Token Account owned by market PDA
```

---

## 👤 User Flows

### Flow 1: Trader (Betting on Outcome)

```
1. Browse Scheduled Matches
   └─ See upcoming battles, current odds, pool sizes

2. Select Match & Outcome
   ├─ Choose YES (Player 1 wins) or NO (Player 2 wins)
   ├─ Enter amount to bet
   └─ See estimated shares and price impact

3. Execute Trade
   ├─ Approve token transfer
   ├─ Transaction processes
   └─ Receive share tokens

4. Wait for Battle
   └─ Watch battle live (optional)

5. Claim Winnings (if correct)
   ├─ Market resolves automatically
   ├─ Claim shares for 1:1 payout
   └─ Receive original currency
```

### Flow 2: Liquidity Provider (Market Maker)

```
1. Find Market
   └─ Look for markets with thin liquidity (high fees)

2. Add Liquidity
   ├─ Deposit equal value to YES and NO pools
   ├─ Receive LP tokens representing ownership
   └─ Earn fees from all trades (0.3%)

3. Wait for Resolution
   └─ Liquidity locked until market resolves

4. Withdraw Liquidity
   ├─ Burn LP tokens
   ├─ Receive pro-rata share of final pool
   └─ Receive accumulated fees
```

### Flow 3: Match Creator

```
1. Schedule Match
   ├─ Set player 1 and player 2
   ├─ Set betting period
   ├─ Choose currency (SOL/USDC/USDT)
   └─ Set initial odds (optional)

2. Create Market
   ├─ Initialize YES/NO pools
   ├─ Add initial liquidity (optional)
   └─ Market opens for trading

3. Start Battle (after betting ends)
   ├─ Betting automatically locks
   ├─ Execute battle on BattleChain
   └─ Result feeds back to market

4. Market Auto-Resolves
   └─ No manual intervention needed
```

---

## 💸 Economic Model

### Fee Structure

```
Trade Volume: 100 SOL
│
├─ Liquidity Providers: 0.30 SOL (0.30%)
├─ Protocol Treasury:    0.05 SOL (0.05%)
├─ Match Creator:        0.05 SOL (0.05%)
└─ Trader Receives:     99.60 SOL in shares

Total Fee: 0.40% per trade
```

### LP Returns

**Scenario: Balanced Market**
```
Initial: 100 SOL YES, 100 SOL NO (200 SOL total)
Trading Volume: 1,000 SOL over betting period
Fees Earned: 1,000 * 0.003 = 3 SOL
LP Return: 3 / 200 = 1.5% return

APY if this repeats daily: (1.015)^365 - 1 = 626% APY
```

**Scenario: Imbalanced Market (Risk)**
```
Initial: 100 SOL YES, 100 SOL NO
Outcome: YES wins
Final Pool: 150 SOL YES, 75 SOL NO

LP Had: 50% ownership (100 SOL value)
LP Gets: 50% of winning side = 75 SOL

Loss: 25 SOL (25% loss despite fees)
```

**Impermanent Loss Protection:**
LPs can choose to:
1. **Standard LP**: Equal YES/NO, subject to IL
2. **Single-Sided**: LP only one outcome, no IL but less fee capture

### Protocol Revenue

With 1,000 matches/month, 100 SOL average volume per match:
```
Monthly Volume: 100,000 SOL
Protocol Fee (0.05%): 50 SOL/month
Annual Revenue: 600 SOL/year (~$60k at $100/SOL)
```

---

## 🔧 Implementation Phases

### **Phase 1: Core AMM Market** (Week 1)

**Deliverables:**
- [ ] Market state accounts (Market, Position, LiquidityPool)
- [ ] AMM implementation (constant product formula)
- [ ] Basic instructions:
  - [ ] `create_market` - Initialize new market
  - [ ] `add_liquidity` - Become LP
  - [ ] `remove_liquidity` - Exit LP position
  - [ ] `buy_shares` - Trade (buy YES or NO)
  - [ ] `sell_shares` - Trade (sell back to pool)
- [ ] SOL-only support (simplify testing)
- [ ] Tests for AMM math

**Success Criteria:**
- ✅ Can create market with 50/50 odds
- ✅ Prices update correctly with trades
- ✅ LPs earn fees proportionally
- ✅ All tests pass

### **Phase 2: BattleChain Integration** (Week 2)

**Deliverables:**
- [ ] Scheduled match system
- [ ] Link to BattleChain battle offers
- [ ] Market lifecycle management:
  - [ ] `create_scheduled_match` - Create match + market
  - [ ] `lock_market` - Close betting when time reached
  - [ ] `resolve_market` - Read battle result and pay winners
  - [ ] `claim_winnings` - Users collect payouts
- [ ] Timing enforcement (betting periods)
- [ ] Auto-resolution from battle results
- [ ] Integration tests with BattleChain

**Success Criteria:**
- ✅ Markets lock at correct time
- ✅ Resolution reads correct battle winner
- ✅ Winners receive 1:1 payout
- ✅ Losers receive nothing
- ✅ LPs get their share + fees

### **Phase 3: Multi-Token Support** (Week 3)

**Deliverables:**
- [ ] USDC market support
- [ ] USDT market support
- [ ] Token vault management (ATAs)
- [ ] Instructions updated for token transfers:
  - [ ] `add_liquidity` with SPL tokens
  - [ ] `buy_shares` with SPL tokens
  - [ ] `claim_winnings` in SPL tokens
- [ ] Tests for each token type

**Success Criteria:**
- ✅ Can create USDC/USDT markets
- ✅ All operations work with stablecoins
- ✅ No loss of precision
- ✅ Gas costs acceptable

### **Phase 4: Advanced Features** (Week 4+)

**Future Enhancements:**
- [ ] Single-sided liquidity provision
- [ ] Market orders with slippage limits
- [ ] Bulk claim (multiple markets at once)
- [ ] LP token (ERC-20 style)
- [ ] Market analytics (volume, returns, etc.)
- [ ] Frontend integration
- [ ] Leaderboards (best predictors)
- [ ] Market maker incentives

---

## 📐 Technical Specifications

### Account Structures

**Market Account** (~400 bytes)
```rust
#[account]
pub struct Market {
    // Identity
    pub market_id: u64,              // 8 bytes
    pub match_id: u64,               // 8 bytes
    pub scheduled_match: Pubkey,     // 32 bytes

    // AMM State
    pub yes_pool: u64,               // 8 bytes
    pub no_pool: u64,                // 8 bytes
    pub k_last: u128,                // 16 bytes (constant product)

    // Liquidity
    pub total_liquidity: u64,        // 8 bytes (LP shares issued)

    // Economics
    pub fee_bps: u16,                // 2 bytes (40 bps = 0.4%)
    pub total_volume: u64,           // 8 bytes
    pub total_fees_collected: u64,   // 8 bytes

    // Currency
    pub currency: Currency,          // 33 bytes (1 + 32 for SPL)
    pub vault: Option<Pubkey>,       // 33 bytes (SPL token vault)

    // Timing
    pub created_at: i64,             // 8 bytes
    pub resolved_at: Option<i64>,    // 9 bytes

    // Status
    pub status: MarketStatus,        // 1 byte
    pub outcome: Option<bool>,       // 2 bytes

    // References
    pub battle_result: Option<Pubkey>, // 33 bytes

    pub bump: u8,                    // 1 byte
}
```

**Position Account** (~150 bytes)
```rust
#[account]
pub struct Position {
    pub market: Pubkey,              // 32 bytes
    pub owner: Pubkey,               // 32 bytes

    // Holdings
    pub yes_shares: u64,             // 8 bytes
    pub no_shares: u64,              // 8 bytes

    // Entry
    pub avg_entry_price: u64,        // 8 bytes (for analytics)
    pub total_invested: u64,         // 8 bytes

    // Lifecycle
    pub created_at: i64,             // 8 bytes
    pub last_trade_at: i64,          // 8 bytes

    // Claimed
    pub claimed: bool,               // 1 byte
    pub payout_amount: u64,          // 8 bytes

    pub bump: u8,                    // 1 byte
}
```

**LiquidityPosition Account** (~150 bytes)
```rust
#[account]
pub struct LiquidityPosition {
    pub market: Pubkey,              // 32 bytes
    pub provider: Pubkey,            // 32 bytes

    // LP Shares
    pub lp_shares: u64,              // 8 bytes

    // Entry State (for IL calculation)
    pub yes_deposited: u64,          // 8 bytes
    pub no_deposited: u64,           // 8 bytes

    // Fees Earned
    pub fees_earned: u64,            // 8 bytes

    // Lifecycle
    pub deposited_at: i64,           // 8 bytes
    pub withdrawn: bool,             // 1 byte

    pub bump: u8,                    // 1 byte
}
```

### PDA Seeds

```rust
// Market PDA
["market", match_id.to_le_bytes()]

// Position PDA
["position", market.key, owner.key]

// Liquidity Position PDA
["liquidity", market.key, provider.key]

// Token Vault PDA (for SPL markets)
["vault", market.key]
```

### Instruction Compute Units

| Instruction | Est. CU | Notes |
|-------------|---------|-------|
| create_market | ~10k | Initialize accounts |
| add_liquidity | ~15k | Math + token transfers |
| remove_liquidity | ~20k | More math, multiple transfers |
| buy_shares | ~25k | AMM calculation + updates |
| sell_shares | ~25k | AMM calculation + updates |
| resolve_market | ~30k | Read external state, distribute |
| claim_winnings | ~15k | Simple payout |

**Total for one user journey (create, trade, claim): ~75k CU**

---

## 🧪 Testing Strategy

### Unit Tests
- [ ] AMM math (constant product)
- [ ] Price impact calculations
- [ ] Fee distribution
- [ ] Slippage limits
- [ ] Multi-token support

### Integration Tests
- [ ] Full market lifecycle
- [ ] Multiple concurrent traders
- [ ] LP fee accumulation
- [ ] Market resolution from BattleChain
- [ ] Edge cases (extreme odds, low liquidity)

### Stress Tests
- [ ] 100+ traders in one market
- [ ] Extreme imbalances (99% YES)
- [ ] Large single trades (pool manipulation)
- [ ] Rapid add/remove liquidity

---

## 🔐 Security Considerations

### Attack Vectors

1. **Front-running**
   - **Risk**: Bots see large trade, front-run to profit
   - **Mitigation**: Slippage limits, private mempool (future)

2. **Price Manipulation**
   - **Risk**: Whale buys all YES, manipulates battle
   - **Mitigation**: Max bet per wallet, battle randomness

3. **Oracle Manipulation**
   - **Risk**: Fake battle results
   - **Mitigation**: Read directly from BattleChain (trusted), signature verification

4. **Rug Pull (Market Creator)**
   - **Risk**: Creator drains liquidity
   - **Mitigation**: Market is program-owned PDA, creator has no special access

5. **Impermanent Loss Gaming**
   - **Risk**: LP adds liquidity, immediately removes if losing
   - **Mitigation**: Time-lock on liquidity (optional), IL is expected

### Audit Checklist

- [ ] Integer overflow/underflow checks
- [ ] Reentrancy protection (Solana: N/A but check CPI)
- [ ] Access control (signers, PDAs)
- [ ] Token vault security
- [ ] AMM formula correctness
- [ ] Fee calculation accuracy
- [ ] Market resolution cannot be replayed

---

## 📊 Success Metrics

### MVP (Phase 1-2)
- ✅ 10+ test markets created
- ✅ 100+ total trades
- ✅ 10+ LPs providing liquidity
- ✅ 0 critical bugs
- ✅ <50k CU per trade

### Production (Phase 3-4)
- 🎯 1,000+ markets/month
- 🎯 $100k+ monthly volume
- 🎯 100+ daily active traders
- 🎯 10%+ average LP returns
- 🎯 <0.1% slippage for typical trades

---

## 🚀 Next Steps

1. **Week 1**: Implement core AMM (Phase 1)
2. **Week 2**: Integrate with BattleChain (Phase 2)
3. **Week 3**: Add USDC/USDT support (Phase 3)
4. **Week 4**: Build frontend + advanced features (Phase 4)

---

## 📚 Additional Resources

- [Uniswap V2 Whitepaper](https://uniswap.org/whitepaper.pdf) - AMM reference
- [Polymarket Docs](https://docs.polymarket.com/) - Prediction market design
- [Solana Cookbook](https://solanacookbook.com/) - Development patterns
- [BattleChain README](../programs/battlechain_v2/README.md) - Integration reference

---

**Last Updated**: 2025-11-12
**Version**: 1.0.0
**Status**: In Development - Phase 1
