// ============================================================================
// STEAL PROGRAM - A+ RULESET
// ============================================================================

pub mod constants;
pub mod state;
pub mod logic;
pub mod errors;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod insolvency_test;

#[cfg(test)]
mod time_attack_test;

#[cfg(test)]
mod pot_drainage_test;

// Anchor program setup (compiled for Solana builds and IDL generation, not for unit tests)
#[cfg(any(not(test), feature = "idl-build"))]
use anchor_lang::prelude::*;

#[cfg(any(not(test), feature = "idl-build"))]
use anchor_spl::token::{Token, TokenAccount};
#[cfg(any(not(test), feature = "idl-build"))]
use anchor_spl::token::spl_token;

#[cfg(any(not(test), feature = "idl-build"))]
use crate::constants::*;

#[cfg(any(not(test), feature = "idl-build"))]
use crate::state::{GameAccount, GameConfig};

#[cfg(any(not(test), feature = "idl-build"))]
use crate::errors::StealError;

#[cfg(any(not(test), feature = "idl-build"))]
use crate::logic::{
    calculate_hit_a_lick_price, 
    calculate_yield_with_cap,
    calculate_run_it_up_split,
    calculate_hit_a_lick_entry_cost,
    calculate_hit_a_lick_old_king_payout,
    calculate_hit_a_lick_overhead_split,
    calculate_hit_a_lick_end_payouts,
    update_recent_kings,
    compute_min_growth_steals_for_war,
    is_vip
};


#[cfg(any(not(test), feature = "idl-build"))]
declare_id!("CM9y2DreJSMqzoRRrLkWEZzTB9ve5D4gQHcPPxrw8mxg");

// Security.txt for verified builds - enables source verification on explorers
#[cfg(not(feature = "no-entrypoint"))]
use solana_security_txt::security_txt;

#[cfg(not(feature = "no-entrypoint"))]
security_txt! {
    name: "Steal",
    project_url: "https://crownclash.xyz",
    contacts: "discord:crownclash",
    policy: "https://github.com/starstarstarstarstarstarstarstar/steal/blob/main/SECURITY.md",
    preferred_languages: "en",
    source_code: "https://github.com/starstarstarstarstarstarstarstar/steal",
    auditors: "N/A"
}

#[cfg(any(not(test), feature = "idl-build"))]
#[program]
pub mod steal {
    use super::*;

    /// Initialize a new game round
    pub fn initialize_game(
        ctx: Context<InitializeGame>,
        jackpot_seed: u64,
        yield_seed: u64,
        season_start_time: i64,
    ) -> Result<()> {
        // Validate dev and beast wallets are funded (rent-exempt)
        // This prevents InsufficientFundsForRent errors during steals
        require!(
            ctx.accounts.dev_wallet.lamports() >= RENT_EXEMPT_MIN,
            StealError::WalletNotRentExempt
        );
        require!(
            ctx.accounts.beast_wallet.lamports() >= RENT_EXEMPT_MIN,
            StealError::WalletNotRentExempt
        );
        
        let game = &mut ctx.accounts.game;
        let clock = Clock::get()?;
        
        game.current_price = START_PRICE;
        game.hit_a_lick_price = 0;
        game.jackpot_balance = jackpot_seed;
        game.pending_jackpot = 0;
        game.yield_pool = yield_seed;
        game.round_end_time = clock.unix_timestamp + MAX_TIMER as i64;
        game.hit_a_lick_end_time = 0;
        game.is_hit_a_lick_mode = false;
        game.current_king = Pubkey::default();
        game.king_since = 0;
        game.king_entry_price = 0;
        game.king_base_price = 0;
        game.has_king = false;
        game.dev_wallet = ctx.accounts.dev_wallet.key();
        game.beast_wallet = ctx.accounts.beast_wallet.key();
        game.steal_mint = ctx.accounts.steal_mint.key();
        game.round = 1;
        game.total_steals = 0;
        game.total_burned = 0;
        game.beast_sol_pending = 0;
        game.recent_kings = [Pubkey::default(); 3];
        game.recent_kings_count = 0;
        game.last_steal_wallet = Pubkey::default();
        game.last_steal_time = 0;
        game.cooldown_seconds = HIT_A_LICK_COOLDOWN_SECS;
        game.growth_steals = 0;
        game.min_growth_steals_for_war = compute_min_growth_steals_for_war(jackpot_seed);
        game.growth_hard_end_ts = clock.unix_timestamp + GROWTH_MAX_DURATION_SECS;
        game.king_was_vip = false;
        game.season_start_time = season_start_time;
        game.bump = ctx.bumps.game;
        
        // Note: Vault should be funded with at least jackpot_seed + yield_seed before first steal
        // This is verified during the first steal operation with balance checks
        
        msg!("Game initialized! Price: {} lamports, Jackpot: {} lamports", 
             game.current_price, game.jackpot_balance);
        
        Ok(())
    }

    /// Initialize the game config account (canonical address storage)
    pub fn initialize_config(ctx: Context<InitializeConfig>) -> Result<()> {
        // Validate dev and beast wallets are funded (rent-exempt)
        // This prevents InsufficientFundsForRent errors during steals
        require!(
            ctx.accounts.dev_wallet.lamports() >= RENT_EXEMPT_MIN,
            StealError::WalletNotRentExempt
        );
        require!(
            ctx.accounts.beast_wallet.lamports() >= RENT_EXEMPT_MIN,
            StealError::WalletNotRentExempt
        );
        
        let config = &mut ctx.accounts.config;
        config.authority = ctx.accounts.authority.key();
        config.dev_wallet = ctx.accounts.dev_wallet.key();
        config.beast_wallet = ctx.accounts.beast_wallet.key();
        config.steal_mint = ctx.accounts.steal_mint.key();
        config.bump = ctx.bumps.config;
        
        msg!("Config initialized! Authority: {}, Dev: {}, Beast: {}, Mint: {}", 
             config.authority, config.dev_wallet, config.beast_wallet, config.steal_mint);
        
        Ok(())
    }

    /// Steal the crown (main game action)
    pub fn steal(ctx: Context<Steal>) -> Result<()> {
        let clock = Clock::get()?;
        let player = ctx.accounts.player.key();
        let vault_bump = ctx.bumps.game_vault;
        let vault_seeds: &[&[u8]] = &[b"vault", &[vault_bump]];
        
        // Block paid steals before season starts
        let game = &ctx.accounts.game;
        require!(
            clock.unix_timestamp >= game.season_start_time || game.season_start_time == 0,
            StealError::SeasonNotStarted
        );
        
        // Validate and deserialize config account if provided (manual validation for backward compatibility)
        // When config is None (null), we skip validation and use game account values
        let config_data: Option<GameConfig> = if let Some(ref config_acc) = ctx.accounts.config {
            // Derive expected config PDA
            let (expected_config_pda, _bump) = Pubkey::find_program_address(
                &[b"steal-config"],
                ctx.program_id,
            );
            
            // Verify the provided account matches the expected PDA
            require!(
                config_acc.key() == expected_config_pda,
                StealError::InvalidAccount
            );
            
            // Verify account is owned by this program
            require!(
                config_acc.owner == ctx.program_id,
                StealError::InvalidAccount
            );
            
            // Deserialize the config account (try_deserialize expects full data with discriminator)
            let data = config_acc.data.borrow();
            let config = GameConfig::try_deserialize(&mut &data[..])
                .map_err(|_| StealError::InvalidAccount)?;
            
            Some(config)
        } else {
            None
        };
        
        // Get expected addresses from config if available, otherwise from game (backward compatibility)
        // Do this before borrowing game as mutable
        let (expected_dev_wallet, expected_beast_wallet, _steal_mint) = if let Some(config) = &config_data {
            (config.dev_wallet, config.beast_wallet, config.steal_mint)
        } else {
            (ctx.accounts.game.dev_wallet, ctx.accounts.game.beast_wallet, ctx.accounts.game.steal_mint)
        };
        
        let game = &mut ctx.accounts.game;
        
        // Verify provided accounts match expected addresses
        require!(
            ctx.accounts.dev_wallet.key() == expected_dev_wallet,
            StealError::InvalidAccount
        );
        require!(
            ctx.accounts.beast_wallet.key() == expected_beast_wallet,
            StealError::InvalidAccount
        );
        
        // Check if round has ended
        if game.is_hit_a_lick_mode {
            // Hit A Lick mode: allow steals even after timer if king hasn't held 3s yet
            if clock.unix_timestamp >= game.hit_a_lick_end_time {
                // Timer expired - check if king has held long enough
                if game.has_king {
                    let held_secs = (clock.unix_timestamp - game.king_since) as u64;
                    if held_secs >= MIN_HIT_A_LICK_HOLD {
                        // King has held 3s+, round is settleable - must call end_round
                        return Err(StealError::RoundEnded.into());
                    }
                    // King hasn't held 3s yet - allow steal to continue (king is snipeable)
                } else {
                    // No king - round ended, must call end_round
                    return Err(StealError::RoundEnded.into());
                }
            }
            // Timer hasn't expired yet, or timer expired but king < 3s hold - allow steal
            
            // Rate limiting: check cooldown in Hit A Lick mode
            if player == game.last_steal_wallet && game.last_steal_time > 0 {
                let time_since_last_steal = clock.unix_timestamp - game.last_steal_time;
                if time_since_last_steal < game.cooldown_seconds as i64 {
                    return Err(StealError::RateLimitExceeded.into());
                }
            }
        } else {
            require!(
                clock.unix_timestamp < game.round_end_time,
                StealError::RoundEnded
            );
        }
        
        // Calculate entry cost based on token holdings
        // Properly verify token account if provided
        let token_balance: u64 = if let Some(ref token_acc) = ctx.accounts.player_token_account {
            // Verify account is owned by Token Program
            require!(
                token_acc.owner == &spl_token::ID,
                StealError::InvalidTokenAccount
            );
            
            // Deserialize token account to verify structure and get balance
            let token_data = TokenAccount::try_deserialize(&mut &token_acc.data.borrow()[..])
                .map_err(|_| StealError::InvalidTokenAccount)?;
            
            // Verify mint matches game's steal_mint
            require!(
                token_data.mint == game.steal_mint,
                StealError::InvalidMint
            );
            
            // Verify token account owner matches player
            require!(
                token_data.owner == player,
                StealError::InvalidTokenAccountOwner
            );
            
            token_data.amount
        } else {
            0
        };
        
        let is_vip = is_vip(token_balance);
        
        // Calculate entry cost based on mode
        // IMPORTANT: This is calculated BEFORE any state updates, so it correctly captures
        // the price the player will pay even if this steal triggers HAL transition
        let entry_cost = if game.is_hit_a_lick_mode {
            // Hit A Lick mode: VIP pays no surcharge, normie pays 12% surcharge
            calculate_hit_a_lick_entry_cost(game.hit_a_lick_price, is_vip)
        } else {
            // Run It Up mode: pay new price (current * 1.12) - VIP and Normie pay same price
            // VIP benefits are in yield, not price
            game.current_price
                .checked_mul(RUN_IT_UP_RATE)
                .and_then(|v| v.checked_div(100))
                .unwrap_or(u64::MAX)
        };
        
        // Safety check: entry_cost should never be 0 if player is paying
        require!(
            entry_cost > 0,
            StealError::InvalidAccount
        );
        
        // Verify player has enough SOL
        require!(
            ctx.accounts.player.lamports() >= entry_cost,
            StealError::InsufficientFunds
        );
        
        // Store old_price before any updates (needed for calculations)
        let old_price = game.current_price;
        
        // Calculate new_price (what the new player will pay)
        // In Run It Up: new_price = old_price * 1.12 (same for VIP and normie)
        // In Hit A Lick: new_price = entry_cost (hit_a_lick_price + surcharge, or just hit_a_lick_price for VIP)
        let new_price = entry_cost;
        
        // Calculate all required payouts BEFORE making any transfers
        // This allows us to verify vault has sufficient balance
        let mut total_required_payouts = 0u64;
        let mut old_king_payout = 0u64;
        let mut yield_payout = 0u64;
        
        if game.has_king && game.current_king != player {
            // Check if old_king account matches the actual current king
            // If mismatch (race condition), we skip the payout - actual king gets paid when next person steals from them
            // This prevents transaction failures during high contention
            let old_king_matches = ctx.accounts.old_king.key() == game.current_king;
            
            // Only calculate and pay the old king if accounts match
            // Race condition (mismatch): skip payout - actual king gets paid when next person steals from them
            if old_king_matches {
                let hold_time = (clock.unix_timestamp - game.king_since) as u64;
                
                // Calculate refund and yield based on mode
                let refund = if game.is_hit_a_lick_mode {
                    // Hit A Lick mode: 90% refund of what the player actually paid
                    // This correctly handles transition case where player paid Run It Up price
                    // CRITICAL: king_entry_price contains what the player actually paid, which may be
                    // the Run It Up price if they triggered the HAL transition
                    let calculated_refund = calculate_hit_a_lick_old_king_payout(game.king_entry_price);
                    // Safety check: refund should never be 0 if player paid something (king_entry_price > 0)
                    // If king_entry_price is 0, that's also an error (player should have paid something)
                    require!(
                        calculated_refund > 0 && game.king_entry_price > 0,
                        StealError::InvalidAccount
                    );
                    calculated_refund
                } else {
                    // Run It Up mode: FULL REFUND of what they paid
                    game.king_entry_price
                };
                
                // Calculate yield (run it up mode only, hit a lick mode has no yield)
                yield_payout = if game.is_hit_a_lick_mode {
                    0
                } else {
                    // Use tracked VIP status for yield calculation
                    let old_king_was_vip = game.king_was_vip;
                    let uncapped_yield = calculate_yield_with_cap(
                        game.yield_pool,
                        game.king_entry_price,
                        new_price,
                        hold_time,
                        old_king_was_vip
                    );
                    uncapped_yield.max(MIN_YIELD.min(game.yield_pool))
                };
                
                // Total: FULL refund + yield
                old_king_payout = refund.saturating_add(yield_payout);
                total_required_payouts = total_required_payouts.saturating_add(old_king_payout);
            } else {
                // SECURITY NOTE: This is intentional race condition handling.
                // During high contention, multiple steals may be submitted with stale old_king.
                // Rather than failing the transaction (poor UX), we skip the payout.
                // The actual current king (game.current_king) will be paid on the next steal.
                // This design prioritizes availability over strict consistency.
                msg!("Race condition detected: old_king={}, current_king={}, skipping payout (actual king paid on next steal)",
                     ctx.accounts.old_king.key(), game.current_king);
            }
        }
        
        // Calculate overhead payments (dev/beast cuts) BEFORE state updates
        // Also calculate yield_add and jackpot_add for pool updates
        let (overhead_dev_cut, overhead_beast_cut, overhead_yield_add, overhead_jackpot_add) = if game.is_hit_a_lick_mode {
            let (d, b, y, j) = calculate_hit_a_lick_overhead_split(game.hit_a_lick_price);
            // Add surcharge to jackpot (entry_cost - hit_a_lick_price for normies, 0 for VIPs)
            let surcharge = entry_cost.saturating_sub(game.hit_a_lick_price);
            (d, b, y, j.saturating_add(surcharge))
        } else {
            // Calculate taxes on profit_delta (new_price - old_price)
            let (_, d, b, y, j) = calculate_run_it_up_split(old_price, new_price);
            (d, b, y, j)
        };
        total_required_payouts = total_required_payouts.saturating_add(overhead_dev_cut).saturating_add(overhead_beast_cut);
        
        // Collect payment from player FIRST (safer transaction order)
        let ix = anchor_lang::solana_program::system_instruction::transfer(
            &ctx.accounts.player.key(),
            &ctx.accounts.game_vault.key(),
            entry_cost,
        );
        anchor_lang::solana_program::program::invoke(
            &ix,
            &[
                ctx.accounts.player.to_account_info(),
                ctx.accounts.game_vault.to_account_info(),
                ctx.accounts.system_program.to_account_info(),
            ],
        )?;
        
        // Check vault balance AFTER collecting entry but BEFORE paying out
        let vault_balance = ctx.accounts.game_vault.lamports();
        require!(
            vault_balance >= total_required_payouts,
            StealError::InsufficientVaultBalance
        );
        
        // Pay the old king if there is one
        if old_king_payout > 0 {
            // Transfer to old king using CPI
            let transfer_ix = anchor_lang::solana_program::system_instruction::transfer(
                &ctx.accounts.game_vault.key(),
                &ctx.accounts.old_king.key(),
                old_king_payout,
            );
            anchor_lang::solana_program::program::invoke_signed(
                &transfer_ix,
                &[
                    ctx.accounts.game_vault.to_account_info(),
                    ctx.accounts.old_king.to_account_info(),
                    ctx.accounts.system_program.to_account_info(),
                ],
                &[vault_seeds],
            )?;
            
            // Deduct yield from pool (only if yield was paid)
            if yield_payout > 0 {
                game.yield_pool = game.yield_pool.saturating_sub(yield_payout);
            }
        }
        
        // Pay dev/beast overhead cuts (using pre-calculated values)
        // Check vault balance again after paying old king
        if overhead_dev_cut > 0 || overhead_beast_cut > 0 {
            let vault_balance_after_old_king = ctx.accounts.game_vault.lamports();
            require!(
                vault_balance_after_old_king >= overhead_dev_cut.saturating_add(overhead_beast_cut),
                StealError::InsufficientVaultBalance
            );
            
            // Transfer dev cut using CPI
            if overhead_dev_cut > 0 {
                let dev_transfer_ix = anchor_lang::solana_program::system_instruction::transfer(
                    &ctx.accounts.game_vault.key(),
                    &ctx.accounts.dev_wallet.key(),
                    overhead_dev_cut,
                );
                anchor_lang::solana_program::program::invoke_signed(
                    &dev_transfer_ix,
                    &[
                        ctx.accounts.game_vault.to_account_info(),
                        ctx.accounts.dev_wallet.to_account_info(),
                        ctx.accounts.system_program.to_account_info(),
                    ],
                    &[vault_seeds],
                )?;
            }
            
            // Transfer beast cut using CPI
            if overhead_beast_cut > 0 {
                let beast_transfer_ix = anchor_lang::solana_program::system_instruction::transfer(
                    &ctx.accounts.game_vault.key(),
                    &ctx.accounts.beast_wallet.key(),
                    overhead_beast_cut,
                );
                anchor_lang::solana_program::program::invoke_signed(
                    &beast_transfer_ix,
                    &[
                        ctx.accounts.game_vault.to_account_info(),
                        ctx.accounts.beast_wallet.to_account_info(),
                        ctx.accounts.system_program.to_account_info(),
                    ],
                    &[vault_seeds],
                )?;
                
                // Track pending SOL for buyback-burn
                game.beast_sol_pending = game.beast_sol_pending.saturating_add(overhead_beast_cut);
            }
        }
        
        // Update game state based on mode
        if game.is_hit_a_lick_mode {
            // Hit A Lick mode: timer RESETS to 30 seconds on each steal, price stays frozen at hit_a_lick_price
            game.hit_a_lick_end_time = clock.unix_timestamp + HIT_A_LICK_TIMER as i64;
            game.current_price = game.hit_a_lick_price;
            
            // Update pools with pre-calculated values
            game.yield_pool = game.yield_pool.saturating_add(overhead_yield_add);
            game.pending_jackpot = game.pending_jackpot.saturating_add(overhead_jackpot_add);
        } else {
            // Run It Up mode: price increases 12% (use checked arithmetic)
            game.current_price = game.current_price
                .checked_mul(RUN_IT_UP_RATE)
                .and_then(|v| v.checked_div(100))
                .unwrap_or(u64::MAX);
            
            // Add time to existing timer (cap at 10 minutes from now)
            let new_end = game.round_end_time + TIMER_ADD as i64;
            let max_end = clock.unix_timestamp + MAX_TIMER as i64;
            game.round_end_time = new_end.min(max_end);
            
            // Update pools with pre-calculated values
            game.yield_pool = game.yield_pool.saturating_add(overhead_yield_add);
            game.pending_jackpot = game.pending_jackpot.saturating_add(overhead_jackpot_add);
            
            // Increment growth steals counter
            game.growth_steals = game.growth_steals.saturating_add(1);
            
            // Check for hit a lick mode transition: price >= 60% of jackpot OR steals >= min
            // Check using NEW price and NEW jackpot (after this steal's contribution)
            let steals_trigger = game.growth_steals >= game.min_growth_steals_for_war;
            
            // Price triggers when NEW price >= 60% of NEW jackpot
            let price_triggers = if game.jackpot_balance > 0 {
                let hit_a_lick_threshold = game.jackpot_balance
                    .checked_mul(HIT_A_LICK_THRESHOLD)
                    .and_then(|v| v.checked_div(100))
                    .unwrap_or(u64::MAX);
                
                // Trigger when current price >= 60% of jackpot (no minimum threshold check)
                let triggers = game.current_price >= hit_a_lick_threshold;
                
                // Debug logging to help diagnose
                msg!("Hit A Lick check: price={} lamports, threshold={} lamports (60% of {}), growth_steals={}/{}, triggers={}", 
                     game.current_price, hit_a_lick_threshold, game.jackpot_balance, game.growth_steals, game.min_growth_steals_for_war, triggers);
                
                triggers
            } else {
                false
            };
            
            // Hit A Lick mode triggers when EITHER price threshold (60% of jackpot) OR minimum steals are met
            // This allows flame bar (price progress) to trigger hit a lick mode when full
            if price_triggers || steals_trigger {
                game.is_hit_a_lick_mode = true;
                // Calculate hit a lick price: 3% of pot, clamped [0.05, 1.50] SOL
                game.hit_a_lick_price = calculate_hit_a_lick_price(game.jackpot_balance);
                // Freeze current_price at hit_a_lick_price (for display/consistency)
                game.current_price = game.hit_a_lick_price;
                game.hit_a_lick_end_time = clock.unix_timestamp + HIT_A_LICK_TIMER as i64;
                // Reset recent kings for new War
                game.recent_kings = [Pubkey::default(); 3];
                game.recent_kings_count = 0;
                msg!("HIT A LICK MODE ACTIVATED! Hit A Lick price: {} lamports (trigger: price={}, steals={})", 
                     game.hit_a_lick_price, price_triggers, steals_trigger);
            }
        }
        
        // Update king
        game.current_king = player;
        game.king_since = clock.unix_timestamp;
        // CRITICAL: Always set king_entry_price to the actual entry_cost the player paid
        // This preserves the Run It Up price even if this steal triggered HAL transition
        // The entry_cost was calculated BEFORE any state updates, so it correctly captures
        // what the player actually paid (Run It Up price), not the HAL price
        game.king_entry_price = entry_cost;  // Total paid (includes lazy tax if normie)
        
        // Safety check: king_entry_price should never be 0 if a player paid
        require!(
            game.king_entry_price > 0,
            StealError::InvalidAccount
        );
        
        // In hit a lick mode, king_base_price is the hit_a_lick_price
        // In run it up mode, use entry_cost (the actual amount paid, excluding lazy tax)
        game.king_base_price = if game.is_hit_a_lick_mode {
            game.hit_a_lick_price
        } else {
            entry_cost
        };
        game.king_was_vip = is_vip;  // Track VIP status for yield calculation
        game.has_king = true;
        game.total_steals += 1;
        
        // Update rate limiting tracking
        game.last_steal_wallet = player;
        game.last_steal_time = clock.unix_timestamp;
        
        // Track recent kings for War mode (3-winner payout)
        if game.is_hit_a_lick_mode {
            let mut recent_kings = game.recent_kings;
            let mut count = game.recent_kings_count;
            update_recent_kings(&mut recent_kings, &mut count, player);
            game.recent_kings = recent_kings;
            game.recent_kings_count = count;
        }
        
        msg!("STEAL! {} stole the crown for {} lamports", player, entry_cost);
        
        Ok(())
    }

    /// End the round and pay the winners (up to 3 in War mode)
    pub fn end_round(ctx: Context<EndRound>) -> Result<()> {
        let clock = Clock::get()?;
        let vault_bump = ctx.bumps.game_vault;
        let vault_seeds: &[&[u8]] = &[b"vault", &[vault_bump]];
        
        // Validate and deserialize config account if provided (manual validation for backward compatibility)
        // When config is None (null), we skip validation and use game account values
        let config_data: Option<GameConfig> = if let Some(ref config_acc) = ctx.accounts.config {
            // Derive expected config PDA
            let (expected_config_pda, _bump) = Pubkey::find_program_address(
                &[b"steal-config"],
                ctx.program_id,
            );
            
            // Verify the provided account matches the expected PDA
            require!(
                config_acc.key() == expected_config_pda,
                StealError::InvalidAccount
            );
            
            // Verify account is owned by this program
            require!(
                config_acc.owner == ctx.program_id,
                StealError::InvalidAccount
            );
            
            // Deserialize the config account (try_deserialize expects full data with discriminator)
            let data = config_acc.data.borrow();
            let config = GameConfig::try_deserialize(&mut &data[..])
                .map_err(|_| StealError::InvalidAccount)?;
            
            Some(config)
        } else {
            None
        };
        
        // Get expected addresses from config if available, otherwise from game (backward compatibility)
        // Do this before borrowing game as mutable
        let (expected_dev_wallet, expected_beast_wallet, _steal_mint) = if let Some(config) = &config_data {
            (config.dev_wallet, config.beast_wallet, config.steal_mint)
        } else {
            (ctx.accounts.game.dev_wallet, ctx.accounts.game.beast_wallet, ctx.accounts.game.steal_mint)
        };
        
        let game = &mut ctx.accounts.game;
        
        // Verify provided accounts match expected addresses
        require!(
            ctx.accounts.dev_wallet.key() == expected_dev_wallet,
            StealError::InvalidAccount
        );
        require!(
            ctx.accounts.beast_wallet.key() == expected_beast_wallet,
            StealError::InvalidAccount
        );
        
        // Check if round has actually ended (hit a lick timer for hit a lick mode)
        if game.is_hit_a_lick_mode {
            require!(
                clock.unix_timestamp >= game.hit_a_lick_end_time,
                StealError::RoundNotEnded
            );
            
            // Hit A Lick mode: check if king has held for minimum 3 seconds
            require!(game.has_king, StealError::NoKing);
            let held_secs = (clock.unix_timestamp - game.king_since) as u64;
            if held_secs < MIN_HIT_A_LICK_HOLD {
                // King hasn't held long enough - extend timer
                game.hit_a_lick_end_time = game.king_since + MIN_HIT_A_LICK_HOLD as i64;
                return Err(StealError::KingMustHoldLonger.into());
            }
        } else {
            require!(
                clock.unix_timestamp >= game.round_end_time,
                StealError::RoundNotEnded
            );
        }
        
        // Must have a king to end
        require!(game.has_king, StealError::NoKing);
        
        // Calculate payouts based on whether hit a lick mode was reached
        let total_pot = game.jackpot_balance;
        
        // For Hit A Lick mode, we pay up to 3 winners from recent_kings
        // For dead rounds, only the current king gets refund + yield
        let (winner1_payout, winner2_payout, winner3_payout, dev_payout, beast_payout, next_jackpot) = if game.is_hit_a_lick_mode {
            // HIT A LICK MODE: Use 3-winner A+ ruleset payouts
            let is_mega = total_pot >= MEGA_POT_THRESHOLD;
            
            // Use calculate_hit_a_lick_end_payouts which returns 6 values now
            let (w1, w2, w3, dev, beast, rollover) = calculate_hit_a_lick_end_payouts(total_pot);
            
            // Only pay winners that exist (skip empty slots)
            // Special case: if no recent kings, current king gets refund only (NO YIELD in hit a lick mode)
            let actual_w1 = if game.recent_kings_count >= 1 { 
                w1 
            } else { 
                // No recent kings: current king gets refund only (what they paid), NO YIELD
                game.king_entry_price
            };
            let actual_w2 = if game.recent_kings_count >= 2 { w2 } else { 0 };
            let actual_w3 = if game.recent_kings_count >= 3 { w3 } else { 0 };
            
            // Unclaimed winner shares go back to rollover
            // If recent_kings_count == 0, we're giving refund to current king, so no unclaimed
            let unclaimed = if game.recent_kings_count == 0 {
                // Current king gets refund, rest goes to rollover
                w1.saturating_add(w2).saturating_add(w3)
            } else {
                (if game.recent_kings_count < 1 { w1 } else { 0 })
                    .saturating_add(if game.recent_kings_count < 2 { w2 } else { 0 })
                    .saturating_add(if game.recent_kings_count < 3 { w3 } else { 0 })
            };
            
            if is_mega {
                msg!("MEGA HIT A LICK! Pot: {} lamports, {} winners!", total_pot, game.recent_kings_count);
            }
            (actual_w1, actual_w2, actual_w3, dev, beast, rollover.saturating_add(unclaimed))
        } else {
            // DEAD ROUND (no hit a lick reached): FULL Refund + yield, pot rolls forward
            let hold_time = (clock.unix_timestamp - game.king_since) as u64;
            
            // Refund: FULL entry price (including lazy tax/surcharge)
            let refund = game.king_entry_price;
            
            // Yield earned (using same formula)
            // Use a nominal "new payment" for yield calc - use current price as approximation
            // Use tracked VIP status for yield calculation
            let winner_was_vip = game.king_was_vip;
            let yield_earned = calculate_yield_with_cap(
                game.yield_pool,
                game.king_entry_price,
                game.current_price,
                hold_time,
                winner_was_vip
            );
            
            let winner_payout = refund + yield_earned;
            
            // Pools roll forward intact (minus yield paid)
            game.yield_pool = game.yield_pool.saturating_sub(yield_earned);
            
            (winner_payout, 0, 0, 0, 0, game.jackpot_balance)
        };
        
        // Pay winner 1 (current king / most recent in recent_kings) using CPI
        if winner1_payout > 0 {
            // SECURITY: Validate winner can receive SOL (owned by System Program or empty)
            // This prevents transfer failures if someone registered a program-owned account as king
            let winner_owner = ctx.accounts.winner.owner;
            require!(
                *winner_owner == anchor_lang::solana_program::system_program::ID || 
                ctx.accounts.winner.lamports() == 0,
                StealError::InvalidAccount
            );
            
            let winner_transfer_ix = anchor_lang::solana_program::system_instruction::transfer(
                &ctx.accounts.game_vault.key(),
                &ctx.accounts.winner.key(),
                winner1_payout,
            );
            anchor_lang::solana_program::program::invoke_signed(
                &winner_transfer_ix,
                &[
                    ctx.accounts.game_vault.to_account_info(),
                    ctx.accounts.winner.to_account_info(),
                    ctx.accounts.system_program.to_account_info(),
                ],
                &[vault_seeds],
            )?;
        }
        
        // Pay winner 2 (if exists and we have a valid account)
        if winner2_payout > 0 {
            if let Some(ref winner2) = ctx.accounts.winner2 {
                // Verify winner2 matches recent_kings[1]
                require!(
                    winner2.key() == game.recent_kings[1],
                    StealError::InvalidWinner
                );
                
                // SECURITY: Validate winner2 can receive SOL
                let winner2_owner = winner2.owner;
                require!(
                    *winner2_owner == anchor_lang::solana_program::system_program::ID || 
                    winner2.lamports() == 0,
                    StealError::InvalidAccount
                );
                
                let w2_transfer_ix = anchor_lang::solana_program::system_instruction::transfer(
                    &ctx.accounts.game_vault.key(),
                    &winner2.key(),
                    winner2_payout,
                );
                anchor_lang::solana_program::program::invoke_signed(
                    &w2_transfer_ix,
                    &[
                        ctx.accounts.game_vault.to_account_info(),
                        winner2.to_account_info(),
                        ctx.accounts.system_program.to_account_info(),
                    ],
                    &[vault_seeds],
                )?;
            }
        }
        
        // Pay winner 3 (if exists and we have a valid account)
        if winner3_payout > 0 {
            if let Some(ref winner3) = ctx.accounts.winner3 {
                // Verify winner3 matches recent_kings[2]
                require!(
                    winner3.key() == game.recent_kings[2],
                    StealError::InvalidWinner
                );
                
                // SECURITY: Validate winner3 can receive SOL
                let winner3_owner = winner3.owner;
                require!(
                    *winner3_owner == anchor_lang::solana_program::system_program::ID || 
                    winner3.lamports() == 0,
                    StealError::InvalidAccount
                );
                
                let w3_transfer_ix = anchor_lang::solana_program::system_instruction::transfer(
                    &ctx.accounts.game_vault.key(),
                    &winner3.key(),
                    winner3_payout,
                );
                anchor_lang::solana_program::program::invoke_signed(
                    &w3_transfer_ix,
                    &[
                        ctx.accounts.game_vault.to_account_info(),
                        winner3.to_account_info(),
                        ctx.accounts.system_program.to_account_info(),
                    ],
                    &[vault_seeds],
                )?;
            }
        }
        
        // Dev and beast fees come from entries only, not from pot
        // dev_payout and beast_payout are always 0 (constants set to 0)
        
        msg!("Round {} ended! Winner1: {} ({} lamports), Winner2: {} lamports, Winner3: {} lamports (hit_a_lick_mode: {})", 
             game.round, game.current_king, winner1_payout, winner2_payout, winner3_payout, game.is_hit_a_lick_mode);
        
        // Set jackpot for next round (rollover amount + pending jackpot from this round)
        game.jackpot_balance = next_jackpot.saturating_add(game.pending_jackpot);
        // Reset pending jackpot for next round
        game.pending_jackpot = 0;
        // Note: yield_pool unchanged in hit a lick mode, already adjusted in dead round
        
        // Reset for next round
        game.current_price = START_PRICE;
        game.hit_a_lick_price = 0;
        game.round_end_time = clock.unix_timestamp + MAX_TIMER as i64;
        game.hit_a_lick_end_time = 0;
        game.is_hit_a_lick_mode = false;
        game.current_king = Pubkey::default();
        game.king_since = 0;
        game.king_entry_price = 0;
        game.king_base_price = 0;
        game.king_was_vip = false;
        game.has_king = false;
        game.recent_kings = [Pubkey::default(); 3];
        game.recent_kings_count = 0;
        game.last_steal_wallet = Pubkey::default();
        game.last_steal_time = 0;
        game.growth_steals = 0;
        game.min_growth_steals_for_war = compute_min_growth_steals_for_war(next_jackpot);
        game.growth_hard_end_ts = clock.unix_timestamp + GROWTH_MAX_DURATION_SECS;
        game.round += 1;
        game.total_steals = 0;
        // Note: total_burned and beast_sol_pending persist across rounds
        
        Ok(())
    }

    /// Reset a round that ended with no winner
    /// This allows the game to continue when nobody played before timer ran out
    pub fn reset_round(ctx: Context<ResetRound>) -> Result<()> {
        let game = &mut ctx.accounts.game;
        let clock = Clock::get()?;
        
        // Check if round has actually ended
        require!(
            clock.unix_timestamp >= game.round_end_time,
            StealError::RoundNotEnded
        );
        
        // Can only reset if there's NO king (otherwise use end_round)
        require!(!game.has_king, StealError::HasKing);
        
        msg!("Round {} reset - no winner this round", game.round);
        
        // Merge pending jackpot into jackpot balance for next round
        game.jackpot_balance = game.jackpot_balance.saturating_add(game.pending_jackpot);
        game.pending_jackpot = 0;
        
        // Reset for next round (keep pools intact)
        game.current_price = START_PRICE;
        game.hit_a_lick_price = 0;
        game.round_end_time = clock.unix_timestamp + MAX_TIMER as i64;
        game.hit_a_lick_end_time = 0;
        game.is_hit_a_lick_mode = false;
        game.last_steal_wallet = Pubkey::default();
        game.last_steal_time = 0;
        game.growth_steals = 0;
        game.min_growth_steals_for_war = compute_min_growth_steals_for_war(game.jackpot_balance);
        game.growth_hard_end_ts = clock.unix_timestamp + GROWTH_MAX_DURATION_SECS;
        game.round += 1;
        game.total_steals = 0;
        
        Ok(())
    }

    /// Beast buyback and burn - swaps SOL for $STEAL and burns
    /// For devnet: uses mock swap rate (1 SOL = 100,000 $STEAL)
    pub fn beast_buyback_burn(ctx: Context<BeastBuybackBurn>, amount_sol: u64) -> Result<()> {
        let game = &mut ctx.accounts.game;
        
        // Verify there's enough pending SOL
        require!(
            amount_sol <= game.beast_sol_pending,
            StealError::InsufficientFunds
        );
        
        // Mock swap rate: 1 SOL = 100,000 $STEAL (for devnet testing)
        // With 9 decimals: 1 SOL = 100,000 * 10^9 = 100_000_000_000_000 base units
        const MOCK_RATE: u64 = 100_000_000_000_000; // tokens per SOL (with decimals)
        let tokens_to_burn = (amount_sol as u128 * MOCK_RATE as u128 / LAMPORTS_PER_SOL as u128) as u64;
        
        // Transfer SOL from beast wallet (for real swap, this would go to DEX)
        // For mock, we just update the tracking
        game.beast_sol_pending -= amount_sol;
        
        // Burn tokens from burn pool
        let cpi_accounts = anchor_spl::token::Burn {
            mint: ctx.accounts.steal_mint.to_account_info(),
            from: ctx.accounts.burn_pool.to_account_info(),
            authority: ctx.accounts.burn_authority.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
        anchor_spl::token::burn(cpi_ctx, tokens_to_burn)?;
        
        // Update total burned
        game.total_burned += tokens_to_burn;
        
        msg!("BEAST BURN! {} SOL converted to {} $STEAL tokens and burned! Total burned: {}", 
             amount_sol, tokens_to_burn, game.total_burned);
        
        Ok(())
    }
}

// ============================================================================
// ACCOUNT CONTEXTS
// ============================================================================

#[cfg(any(not(test), feature = "idl-build"))]
#[derive(Accounts)]
pub struct InitializeConfig<'info> {
    #[account(
        init,
        payer = authority,
        space = GameConfig::SIZE,
        seeds = [b"steal-config"],
        bump
    )]
    pub config: Account<'info, GameConfig>,
    
    #[account(mut)]
    pub authority: Signer<'info>,
    
    /// CHECK: Dev wallet address
    pub dev_wallet: UncheckedAccount<'info>,
    
    /// CHECK: Beast wallet address
    pub beast_wallet: UncheckedAccount<'info>,
    
    /// CHECK: STEAL token mint
    pub steal_mint: UncheckedAccount<'info>,
    
    pub system_program: Program<'info, System>,
}

#[cfg(any(not(test), feature = "idl-build"))]
#[derive(Accounts)]
pub struct InitializeGame<'info> {
    #[account(
        init,
        payer = authority,
        space = GameAccount::SIZE,
        seeds = [b"game"],
        bump
    )]
    pub game: Account<'info, GameAccount>,
    
    #[account(mut)]
    pub authority: Signer<'info>,
    
    /// CHECK: Dev wallet address
    pub dev_wallet: UncheckedAccount<'info>,
    
    /// CHECK: Beast wallet address
    pub beast_wallet: UncheckedAccount<'info>,
    
    /// CHECK: STEAL token mint
    pub steal_mint: UncheckedAccount<'info>,
    
    pub system_program: Program<'info, System>,
}

#[cfg(any(not(test), feature = "idl-build"))]
#[derive(Accounts)]
pub struct Steal<'info> {
    #[account(
        mut,
        seeds = [b"game"],
        bump = game.bump
    )]
    pub game: Account<'info, GameAccount>,
    
    #[account(mut)]
    pub player: Signer<'info>,
    
    /// CHECK: Player's STEAL token account (optional - for VIP check)
    /// We use UncheckedAccount to allow non-existent accounts for non-VIP players
    pub player_token_account: Option<UncheckedAccount<'info>>,
    
    /// Game vault PDA (owned by System Program)
    #[account(
        mut,
        seeds = [b"vault"],
        bump
    )]
    pub game_vault: SystemAccount<'info>,
    
    /// CHECK: Old king's wallet (for payout)
    #[account(mut)]
    pub old_king: UncheckedAccount<'info>,
    
    /// CHECK: Config account PDA (optional - manually validated if provided)
    /// When provided, must be the correct PDA derived from seeds [b"steal-config"]
    pub config: Option<UncheckedAccount<'info>>,
    
    /// CHECK: Dev wallet (must match config.dev_wallet if config exists, else game.dev_wallet)
    #[account(mut)]
    pub dev_wallet: UncheckedAccount<'info>,
    
    /// CHECK: Beast wallet (must match config.beast_wallet if config exists, else game.beast_wallet)
    #[account(mut)]
    pub beast_wallet: UncheckedAccount<'info>,
    
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
}

#[cfg(any(not(test), feature = "idl-build"))]
#[derive(Accounts)]
pub struct EndRound<'info> {
    #[account(
        mut,
        seeds = [b"game"],
        bump = game.bump
    )]
    pub game: Account<'info, GameAccount>,
    
    /// Game vault PDA (owned by System Program)
    #[account(
        mut,
        seeds = [b"vault"],
        bump
    )]
    pub game_vault: SystemAccount<'info>,
    
    /// CHECK: Winner's wallet (must be current king / recent_kings[0])
    #[account(mut, constraint = winner.key() == game.current_king)]
    pub winner: UncheckedAccount<'info>,
    
    /// CHECK: Config account PDA (optional - manually validated if provided)
    /// When provided, must be the correct PDA derived from seeds [b"steal-config"]
    pub config: Option<UncheckedAccount<'info>>,
    
    /// CHECK: 2nd place winner (optional - must match recent_kings[1] if provided)
    #[account(mut)]
    pub winner2: Option<UncheckedAccount<'info>>,
    
    /// CHECK: 3rd place winner (optional - must match recent_kings[2] if provided)
    #[account(mut)]
    pub winner3: Option<UncheckedAccount<'info>>,
    
    /// CHECK: Dev wallet (must match config.dev_wallet if config exists, else game.dev_wallet)
    #[account(mut)]
    pub dev_wallet: UncheckedAccount<'info>,
    
    /// CHECK: Beast wallet (must match config.beast_wallet if config exists, else game.beast_wallet)
    #[account(mut)]
    pub beast_wallet: UncheckedAccount<'info>,
    
    pub system_program: Program<'info, System>,
}

#[cfg(any(not(test), feature = "idl-build"))]
#[derive(Accounts)]
pub struct ResetRound<'info> {
    #[account(
        mut,
        seeds = [b"game"],
        bump = game.bump
    )]
    pub game: Account<'info, GameAccount>,
}

#[cfg(any(not(test), feature = "idl-build"))]
#[derive(Accounts)]
pub struct BeastBuybackBurn<'info> {
    #[account(
        mut,
        seeds = [b"game"],
        bump = game.bump
    )]
    pub game: Account<'info, GameAccount>,
    
    /// STEAL token mint (for burning)
    #[account(mut, constraint = steal_mint.key() == game.steal_mint)]
    pub steal_mint: Account<'info, anchor_spl::token::Mint>,
    
    /// Token account holding tokens to burn (burn pool)
    #[account(mut)]
    pub burn_pool: Account<'info, TokenAccount>,
    
    /// Authority that can burn from burn_pool
    pub burn_authority: Signer<'info>,
    
    /// CHECK: Beast wallet
    #[account(mut, constraint = beast_wallet.key() == game.beast_wallet)]
    pub beast_wallet: UncheckedAccount<'info>,
    
    pub token_program: Program<'info, Token>,
}
