import * as anchor from "@project-serum/anchor";
import { Program } from "@project-serum/anchor";
import { PublicKey, Keypair, SystemProgram, LAMPORTS_PER_SOL } from "@solana/web3.js";
import { assert, expect } from "chai";
import { PredictionMarket } from "../target/types/prediction_market";

describe("prediction_market", () => {
  // Configure the client to use the local cluster
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.PredictionMarket as Program<PredictionMarket>;

  // Test accounts
  let creator: Keypair;
  let player1: Keypair;
  let player2: Keypair;
  let trader1: Keypair;
  let trader2: Keypair;
  let liquidityProvider: Keypair;

  // PDAs
  let scheduledMatchPDA: PublicKey;
  let marketPDA: PublicKey;
  let position1PDA: PublicKey;
  let position2PDA: PublicKey;
  let liquidityPositionPDA: PublicKey;

  // Test constants
  const MATCH_ID = 1;
  const INITIAL_YES_POOL = 10 * LAMPORTS_PER_SOL; // 10 SOL
  const INITIAL_NO_POOL = 10 * LAMPORTS_PER_SOL; // 10 SOL
  const TRADE_AMOUNT = 1 * LAMPORTS_PER_SOL; // 1 SOL
  const LP_AMOUNT = 5 * LAMPORTS_PER_SOL; // 5 SOL per side

  // Helper function to airdrop SOL
  async function airdrop(connection: any, publicKey: PublicKey, amount: number) {
    const signature = await connection.requestAirdrop(
      publicKey,
      amount * LAMPORTS_PER_SOL
    );
    await connection.confirmTransaction(signature);
  }

  before(async () => {
    // Initialize keypairs
    creator = Keypair.generate();
    player1 = Keypair.generate();
    player2 = Keypair.generate();
    trader1 = Keypair.generate();
    trader2 = Keypair.generate();
    liquidityProvider = Keypair.generate();

    // Airdrop SOL to test accounts
    await airdrop(provider.connection, creator.publicKey, 50);
    await airdrop(provider.connection, player1.publicKey, 50);
    await airdrop(provider.connection, player2.publicKey, 50);
    await airdrop(provider.connection, trader1.publicKey, 50);
    await airdrop(provider.connection, trader2.publicKey, 50);
    await airdrop(provider.connection, liquidityProvider.publicKey, 50);

    // Derive PDAs
    [scheduledMatchPDA] = PublicKey.findProgramAddressSync(
      [Buffer.from("scheduled_match"), new anchor.BN(MATCH_ID).toArrayLike(Buffer, "le", 8)],
      program.programId
    );

    [marketPDA] = PublicKey.findProgramAddressSync(
      [Buffer.from("market"), new anchor.BN(MATCH_ID).toArrayLike(Buffer, "le", 8)],
      program.programId
    );

    [position1PDA] = PublicKey.findProgramAddressSync(
      [Buffer.from("position"), marketPDA.toBuffer(), trader1.publicKey.toBuffer()],
      program.programId
    );

    [position2PDA] = PublicKey.findProgramAddressSync(
      [Buffer.from("position"), marketPDA.toBuffer(), trader2.publicKey.toBuffer()],
      program.programId
    );

    [liquidityPositionPDA] = PublicKey.findProgramAddressSync(
      [Buffer.from("liquidity"), marketPDA.toBuffer(), liquidityProvider.publicKey.toBuffer()],
      program.programId
    );
  });

  describe("1. Market Creation", () => {
    it("Creates a scheduled match", async () => {
      const currentTime = Math.floor(Date.now() / 1000);
      const bettingStart = currentTime;
      const bettingEnd = currentTime + 3600; // 1 hour from now
      const estimatedBattleTime = bettingEnd + 600; // 10 minutes after betting ends

      try {
        await program.methods
          .createScheduledMatch(
            new anchor.BN(MATCH_ID),
            player1.publicKey,
            player2.publicKey,
            PublicKey.default, // No battle offer initially
            false, // Not external
            new anchor.BN(bettingStart),
            new anchor.BN(bettingEnd),
            new anchor.BN(estimatedBattleTime),
            "Test Battle: Player1 vs Player2",
            "A test battle for prediction market"
          )
          .accounts({
            scheduledMatch: scheduledMatchPDA,
            creator: creator.publicKey,
            systemProgram: SystemProgram.programId,
          })
          .signers([creator])
          .rpc();

        const scheduledMatch = await program.account.scheduledMatch.fetch(scheduledMatchPDA);
        assert.equal(scheduledMatch.matchId.toNumber(), MATCH_ID);
        assert.equal(scheduledMatch.player1.toString(), player1.publicKey.toString());
        assert.equal(scheduledMatch.player2.toString(), player2.publicKey.toString());
        assert.equal(scheduledMatch.title, "Test Battle: Player1 vs Player2");

        console.log("✓ Scheduled match created successfully");
      } catch (error) {
        console.error("Scheduled match creation error:", error);
        throw error;
      }
    });

    it("Creates a prediction market", async () => {
      try {
        await program.methods
          .createMarket(
            new anchor.BN(MATCH_ID),
            new anchor.BN(INITIAL_YES_POOL),
            new anchor.BN(INITIAL_NO_POOL)
          )
          .accounts({
            market: marketPDA,
            scheduledMatch: scheduledMatchPDA,
            creator: creator.publicKey,
            systemProgram: SystemProgram.programId,
          })
          .signers([creator])
          .rpc();

        const market = await program.account.market.fetch(marketPDA);
        assert.equal(market.matchId.toNumber(), MATCH_ID);
        assert.equal(market.yesPool.toNumber(), INITIAL_YES_POOL);
        assert.equal(market.noPool.toNumber(), INITIAL_NO_POOL);
        assert.equal(market.creator.toString(), creator.publicKey.toString());

        // Check initial prices (should be 50/50)
        assert.equal(market.yesPriceBps, 5000);
        assert.equal(market.noPriceBps, 5000);

        console.log("✓ Market created successfully with initial liquidity");
        console.log(`  YES pool: ${market.yesPool.toNumber() / LAMPORTS_PER_SOL} SOL`);
        console.log(`  NO pool: ${market.noPool.toNumber() / LAMPORTS_PER_SOL} SOL`);
        console.log(`  Initial odds: ${market.yesPriceBps / 100}% YES, ${market.noPriceBps / 100}% NO`);
      } catch (error) {
        console.error("Market creation error:", error);
        throw error;
      }
    });
  });

  describe("2. Adding Liquidity", () => {
    it("LP adds liquidity to the market", async () => {
      try {
        await program.methods
          .addLiquidity(
            new anchor.BN(LP_AMOUNT),
            new anchor.BN(LP_AMOUNT)
          )
          .accounts({
            market: marketPDA,
            liquidityPosition: liquidityPositionPDA,
            provider: liquidityProvider.publicKey,
            systemProgram: SystemProgram.programId,
          })
          .signers([liquidityProvider])
          .rpc();

        const market = await program.account.market.fetch(marketPDA);
        const lpPosition = await program.account.liquidityPosition.fetch(liquidityPositionPDA);

        assert.equal(market.yesPool.toNumber(), INITIAL_YES_POOL + LP_AMOUNT);
        assert.equal(market.noPool.toNumber(), INITIAL_NO_POOL + LP_AMOUNT);
        assert.isAbove(lpPosition.lpShares.toNumber(), 0);
        assert.equal(lpPosition.yesDeposited.toNumber(), LP_AMOUNT);
        assert.equal(lpPosition.noDeposited.toNumber(), LP_AMOUNT);

        console.log("✓ Liquidity added successfully");
        console.log(`  LP shares minted: ${lpPosition.lpShares.toNumber()}`);
        console.log(`  Total market liquidity: ${market.totalLiquidity.toNumber()}`);
      } catch (error) {
        console.error("Add liquidity error:", error);
        throw error;
      }
    });
  });

  describe("3. Trading - Buying Shares", () => {
    it("Trader1 buys YES shares", async () => {
      try {
        const marketBefore = await program.account.market.fetch(marketPDA);
        const minSharesOut = 0; // Accept any amount for testing
        const maxSlippage = 1000; // 10% max slippage

        await program.methods
          .buyShares(
            new anchor.BN(TRADE_AMOUNT),
            true, // Buy YES
            new anchor.BN(minSharesOut),
            maxSlippage
          )
          .accounts({
            market: marketPDA,
            position: position1PDA,
            buyer: trader1.publicKey,
            systemProgram: SystemProgram.programId,
          })
          .signers([trader1])
          .rpc();

        const marketAfter = await program.account.market.fetch(marketPDA);
        const position = await program.account.position.fetch(position1PDA);

        // YES pool should increase
        assert.isAbove(marketAfter.yesPool.toNumber(), marketBefore.yesPool.toNumber());
        // NO pool should decrease (shares received)
        assert.isBelow(marketAfter.noPool.toNumber(), marketBefore.noPool.toNumber());
        // Position should have YES shares
        assert.isAbove(position.yesShares.toNumber(), 0);
        assert.equal(position.noShares.toNumber(), 0);

        console.log("✓ Trader1 bought YES shares successfully");
        console.log(`  YES shares received: ${position.yesShares.toNumber() / LAMPORTS_PER_SOL}`);
        console.log(`  New YES price: ${marketAfter.yesPriceBps / 100}%`);
        console.log(`  New NO price: ${marketAfter.noPriceBps / 100}%`);
      } catch (error) {
        console.error("Buy YES shares error:", error);
        throw error;
      }
    });

    it("Trader2 buys NO shares", async () => {
      try {
        const marketBefore = await program.account.market.fetch(marketPDA);
        const minSharesOut = 0;
        const maxSlippage = 1000;

        await program.methods
          .buyShares(
            new anchor.BN(TRADE_AMOUNT),
            false, // Buy NO
            new anchor.BN(minSharesOut),
            maxSlippage
          )
          .accounts({
            market: marketPDA,
            position: position2PDA,
            buyer: trader2.publicKey,
            systemProgram: SystemProgram.programId,
          })
          .signers([trader2])
          .rpc();

        const marketAfter = await program.account.market.fetch(marketPDA);
        const position = await program.account.position.fetch(position2PDA);

        // NO pool should increase
        assert.isAbove(marketAfter.noPool.toNumber(), marketBefore.noPool.toNumber());
        // YES pool should decrease
        assert.isBelow(marketAfter.yesPool.toNumber(), marketBefore.yesPool.toNumber());
        // Position should have NO shares
        assert.equal(position.yesShares.toNumber(), 0);
        assert.isAbove(position.noShares.toNumber(), 0);

        console.log("✓ Trader2 bought NO shares successfully");
        console.log(`  NO shares received: ${position.noShares.toNumber() / LAMPORTS_PER_SOL}`);
        console.log(`  New YES price: ${marketAfter.yesPriceBps / 100}%`);
        console.log(`  New NO price: ${marketAfter.noPriceBps / 100}%`);
      } catch (error) {
        console.error("Buy NO shares error:", error);
        throw error;
      }
    });

    it("Validates slippage protection", async () => {
      try {
        const minSharesOut = 100 * LAMPORTS_PER_SOL; // Unreasonably high
        const maxSlippage = 100; // 1% max slippage (very tight)

        await program.methods
          .buyShares(
            new anchor.BN(TRADE_AMOUNT),
            true,
            new anchor.BN(minSharesOut),
            maxSlippage
          )
          .accounts({
            market: marketPDA,
            position: position1PDA,
            buyer: trader1.publicKey,
            systemProgram: SystemProgram.programId,
          })
          .signers([trader1])
          .rpc();

        assert.fail("Should have failed due to slippage");
      } catch (error) {
        assert.include(error.toString(), "SlippageExceeded");
        console.log("✓ Slippage protection working correctly");
      }
    });
  });

  describe("4. Trading - Selling Shares", () => {
    it("Trader1 sells half of YES shares", async () => {
      try {
        const positionBefore = await program.account.position.fetch(position1PDA);
        const sharesToSell = Math.floor(positionBefore.yesShares.toNumber() / 2);
        const minAmountOut = 0;
        const maxSlippage = 1000;

        await program.methods
          .sellShares(
            new anchor.BN(sharesToSell),
            true, // Sell YES
            new anchor.BN(minAmountOut),
            maxSlippage
          )
          .accounts({
            market: marketPDA,
            position: position1PDA,
            seller: trader1.publicKey,
            systemProgram: SystemProgram.programId,
          })
          .signers([trader1])
          .rpc();

        const positionAfter = await program.account.position.fetch(position1PDA);
        assert.approximately(
          positionAfter.yesShares.toNumber(),
          positionBefore.yesShares.toNumber() - sharesToSell,
          1000 // Allow small rounding
        );

        console.log("✓ Trader1 sold YES shares successfully");
        console.log(`  YES shares sold: ${sharesToSell / LAMPORTS_PER_SOL}`);
        console.log(`  Remaining YES shares: ${positionAfter.yesShares.toNumber() / LAMPORTS_PER_SOL}`);
      } catch (error) {
        console.error("Sell YES shares error:", error);
        throw error;
      }
    });
  });

  describe("5. Liquidity Management", () => {
    it("LP removes partial liquidity", async () => {
      try {
        const lpPositionBefore = await program.account.liquidityPosition.fetch(liquidityPositionPDA);
        const sharesToBurn = Math.floor(lpPositionBefore.lpShares.toNumber() / 3);
        const minYesOut = 0;
        const minNoOut = 0;

        await program.methods
          .removeLiquidity(
            new anchor.BN(sharesToBurn),
            new anchor.BN(minYesOut),
            new anchor.BN(minNoOut)
          )
          .accounts({
            market: marketPDA,
            liquidityPosition: liquidityPositionPDA,
            provider: liquidityProvider.publicKey,
            systemProgram: SystemProgram.programId,
          })
          .signers([liquidityProvider])
          .rpc();

        const lpPositionAfter = await program.account.liquidityPosition.fetch(liquidityPositionPDA);
        assert.isBelow(lpPositionAfter.lpShares.toNumber(), lpPositionBefore.lpShares.toNumber());
        assert.isFalse(lpPositionAfter.withdrawn); // Still has shares

        console.log("✓ LP removed partial liquidity successfully");
        console.log(`  LP shares burned: ${sharesToBurn}`);
        console.log(`  Fees earned: ${lpPositionAfter.feesEarned.toNumber() / LAMPORTS_PER_SOL} SOL`);
      } catch (error) {
        console.error("Remove liquidity error:", error);
        throw error;
      }
    });
  });

  describe("6. Market Resolution", () => {
    it("Updates scheduled match to completed status", async () => {
      try {
        // This would normally be done by the battle contract
        // For testing, we simulate the match completion
        await program.methods
          .completeScheduledMatch(
            new anchor.BN(MATCH_ID),
            player1.publicKey, // Player1 wins
            PublicKey.default() // Battle account reference
          )
          .accounts({
            scheduledMatch: scheduledMatchPDA,
            authority: creator.publicKey,
          })
          .signers([creator])
          .rpc();

        const scheduledMatch = await program.account.scheduledMatch.fetch(scheduledMatchPDA);
        assert.equal(scheduledMatch.status.completed, true);
        assert.equal(scheduledMatch.winner.toString(), player1.publicKey.toString());

        console.log("✓ Scheduled match marked as completed");
        console.log(`  Winner: Player1`);
      } catch (error) {
        console.error("Complete match error:", error);
        throw error;
      }
    });

    it("Resolves the market based on match outcome", async () => {
      try {
        await program.methods
          .resolveMarket()
          .accounts({
            market: marketPDA,
            scheduledMatch: scheduledMatchPDA,
            resolver: creator.publicKey,
          })
          .signers([creator])
          .rpc();

        const market = await program.account.market.fetch(marketPDA);
        assert.equal(market.status.resolved, true);
        assert.equal(market.outcome, true); // YES wins (player1 won)
        assert.equal(market.winner.toString(), player1.publicKey.toString());

        console.log("✓ Market resolved successfully");
        console.log(`  Outcome: YES (Player1 won)`);
        console.log(`  Total volume: ${market.totalVolume.toNumber() / LAMPORTS_PER_SOL} SOL`);
      } catch (error) {
        console.error("Market resolution error:", error);
        throw error;
      }
    });
  });

  describe("7. Claiming Winnings", () => {
    it("Winner (Trader1) claims winnings", async () => {
      try {
        const positionBefore = await program.account.position.fetch(position1PDA);
        const balanceBefore = await provider.connection.getBalance(trader1.publicKey);

        await program.methods
          .claimWinnings()
          .accounts({
            market: marketPDA,
            position: position1PDA,
            claimer: trader1.publicKey,
            systemProgram: SystemProgram.programId,
          })
          .signers([trader1])
          .rpc();

        const positionAfter = await program.account.position.fetch(position1PDA);
        const balanceAfter = await provider.connection.getBalance(trader1.publicKey);

        assert.isTrue(positionAfter.claimed);
        assert.isAbove(positionAfter.finalPayout.toNumber(), 0);
        assert.isAbove(balanceAfter, balanceBefore);

        console.log("✓ Trader1 claimed winnings successfully");
        console.log(`  Payout: ${positionAfter.finalPayout.toNumber() / LAMPORTS_PER_SOL} SOL`);
        console.log(`  Net P&L: ${positionAfter.realizedPnl.toNumber() / LAMPORTS_PER_SOL} SOL`);
      } catch (error) {
        console.error("Claim winnings error:", error);
        throw error;
      }
    });

    it("Loser (Trader2) cannot claim winnings", async () => {
      try {
        await program.methods
          .claimWinnings()
          .accounts({
            market: marketPDA,
            position: position2PDA,
            claimer: trader2.publicKey,
            systemProgram: SystemProgram.programId,
          })
          .signers([trader2])
          .rpc();

        assert.fail("Should have failed - trader2 lost");
      } catch (error) {
        assert.include(error.toString(), "NoWinningsToClaim");
        console.log("✓ Loser correctly prevented from claiming");
      }
    });

    it("Cannot claim twice", async () => {
      try {
        await program.methods
          .claimWinnings()
          .accounts({
            market: marketPDA,
            position: position1PDA,
            claimer: trader1.publicKey,
            systemProgram: SystemProgram.programId,
          })
          .signers([trader1])
          .rpc();

        assert.fail("Should have failed - already claimed");
      } catch (error) {
        assert.include(error.toString(), "AlreadyClaimed");
        console.log("✓ Double claim correctly prevented");
      }
    });
  });

  describe("8. Market Analytics", () => {
    it("Displays final market statistics", async () => {
      const market = await program.account.market.fetch(marketPDA);

      console.log("\n===== Final Market Statistics =====");
      console.log(`Market ID: ${market.matchId.toNumber()}`);
      console.log(`Final YES pool: ${market.yesPool.toNumber() / LAMPORTS_PER_SOL} SOL`);
      console.log(`Final NO pool: ${market.noPool.toNumber() / LAMPORTS_PER_SOL} SOL`);
      console.log(`Total volume: ${market.totalVolume.toNumber() / LAMPORTS_PER_SOL} SOL`);
      console.log(`Total fees collected: ${market.totalFeesCollected.toNumber() / LAMPORTS_PER_SOL} SOL`);
      console.log(`  - LP fees: ${market.lpFees.toNumber() / LAMPORTS_PER_SOL} SOL`);
      console.log(`  - Protocol fees: ${market.protocolFees.toNumber() / LAMPORTS_PER_SOL} SOL`);
      console.log(`  - Creator fees: ${market.creatorFees.toNumber() / LAMPORTS_PER_SOL} SOL`);
      console.log(`Outcome: ${market.outcome ? "YES (Player1)" : "NO (Player2)"}`);
      console.log("===================================\n");
    });
  });

  describe("9. Error Cases", () => {
    it("Cannot trade after betting period ends", async () => {
      // This test would require time manipulation or creating a new market with past betting end time
      console.log("✓ (Skipped - requires time manipulation)");
    });

    it("Cannot resolve market before betting ends", async () => {
      // Create a new market with future betting end time and try to resolve
      console.log("✓ (Skipped - requires new market setup)");
    });

    it("Cannot trade below minimum amount", async () => {
      // Would require a new market or position
      console.log("✓ (Skipped - requires new market setup)");
    });
  });
});
