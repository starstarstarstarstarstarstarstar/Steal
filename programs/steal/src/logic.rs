// ============================================================================
// STEAL GAME LOGIC - A+ RULESET
// ============================================================================

use anchor_lang::prelude::*;
use crate::constants::*;
use crate::state::GameState;
use crate::errors::StealError;

// ============================================================================
// ENTRY GATE FUNCTIONS
// ============================================================================

pub fn calculate_entry_cost(token_balance: u64, game_price: u64) -> u64 {
    // VIP and Normie both pay full price - VIP benefits are in yield/surcharge, not price
    game_price
        .checked_mul(NORMIE_RATE)
        .and_then(|v| v.checked_div(100))
        .unwrap_or(0)
}

pub fn is_vip(token_balance: u64) -> bool {
    token_balance >= TOKEN_REQUIREMENT
}

pub fn execute_entry(token_balance: u64, game_price: u64) -> (u64, u64, u64) {
    let amount_due = calculate_entry_cost(token_balance, game_price);
    // VIP and Normie pay same price - VIP gets double yield / no surcharge instead
    (amount_due, 0, 0)
}

pub fn verify_token_mint(user_mint: Pubkey, real_mint: Pubkey) -> Result<()> {
    if user_mint == real_mint {
        Ok(())
    } else {
        Err(StealError::InvalidMint.into())
    }
}

// ============================================================================
// GAME INITIALIZATION
// ============================================================================

pub fn new_game() -> GameState {
    GameState {
        current_price: START_PRICE,
        hit_a_lick_price: 0,
        jackpot_balance: 0,
        pending_jackpot: 0,
        yield_pool: 0,
        time_remaining: MAX_TIMER,
        hit_a_lick_end_time: 0,
        is_hit_a_lick_mode: false,
        current_king: None,
        king_since: 0,
        king_entry_price: 0,
        king_base_price: 0,
        recent_kings: [Pubkey::default(); 3],
        recent_kings_count: 0,
        last_steal_wallet: None,
        last_steal_time: 0,
        cooldown_seconds: HIT_A_LICK_COOLDOWN_SECS,
        growth_steals: 0,
        min_growth_steals_for_war: MIN_GROWTH_STEALS_CLAMP,
        growth_hard_end_ts: 0,
    }
}

// ============================================================================
// HIT A LICK MODE PRICE CALCULATION
// ============================================================================

/// Calculate hit a lick price: 3% of pot, clamped between 0.05 and 1.50 SOL
pub fn calculate_hit_a_lick_price(jackpot: u64) -> u64 {
    // Use checked multiplication to prevent overflow
    let raw_price = jackpot
        .checked_mul(HIT_A_LICK_PCT_OF_POT)
        .and_then(|v| v.checked_div(1000))
        .unwrap_or(u64::MAX); // On overflow, use max value which will clamp to HIT_A_LICK_MAX
    raw_price.clamp(HIT_A_LICK_MIN, HIT_A_LICK_MAX)
}

/// Calculate minimum growth steals required before war can trigger
/// Formula: 35 base + 15 steals per SOL in pot, clamped to [35, 100]
/// This ensures jackpot growth offsets winner payouts (16% of pot) and prevents drainage
pub fn compute_min_growth_steals_for_war(jackpot_lamports: u64) -> u16 {
    // Base steals + (pot_in_lamports * 15) / LAMPORTS_PER_SOL
    // This formula ensures exponential pot growth offsets winner payouts
    let additional_steals = jackpot_lamports
        .saturating_mul(GROWTH_STEALS_PER_SOL)
        .saturating_div(LAMPORTS_PER_SOL);
    let total = GROWTH_STEALS_BASE as u64 + additional_steals;
    (total as u16).max(MIN_GROWTH_STEALS_CLAMP).min(MAX_GROWTH_STEALS_CLAMP)
}

// ============================================================================
// PHASE TRANSITIONS
// ============================================================================

pub fn check_phase_transition(game: &mut GameState) {
    // Only check for hit a lick mode transition if not already in hit a lick mode
    // AND there's actually a jackpot to fight over
    if !game.is_hit_a_lick_mode && game.jackpot_balance > 0 {
        // Use checked arithmetic to prevent overflow
        let hit_a_lick_threshold_price = game.jackpot_balance
            .checked_mul(HIT_A_LICK_THRESHOLD)
            .and_then(|v| v.checked_div(100))
            .unwrap_or(u64::MAX); // On overflow, use max (will never trigger hit a lick)
        
        let price_triggers = game.current_price >= hit_a_lick_threshold_price;
        let steals_trigger = game.growth_steals >= game.min_growth_steals_for_war;
        
        // Hit A Lick mode triggers when EITHER price threshold (60% of jackpot) OR minimum steals are met
        // This allows flame bar (price progress) to trigger hit a lick mode when full
        if price_triggers || steals_trigger {
            trigger_hit_a_lick_mode(game);
        }
    }
}

pub fn trigger_hit_a_lick_mode(game: &mut GameState) {
    game.is_hit_a_lick_mode = true;
    // Calculate hit a lick price: 3% of pot, clamped [0.05, 1.50] SOL
    game.hit_a_lick_price = calculate_hit_a_lick_price(game.jackpot_balance);
    game.hit_a_lick_end_time = HIT_A_LICK_TIMER;
    // Reset recent kings for new War
    game.recent_kings = [Pubkey::default(); 3];
    game.recent_kings_count = 0;
}

// ============================================================================
// RECENT KINGS TRACKING (War Mode)
// ============================================================================

/// Update the recent_kings array with a new king
/// - New king goes to front (index 0)
/// - If king already in list, move to front (no duplicates)
/// - Array maintains last 3 unique kings
pub fn update_recent_kings(recent_kings: &mut [Pubkey; 3], count: &mut u8, new_king: Pubkey) {
    // Check if new_king already exists in the array
    let mut existing_idx: Option<usize> = None;
    for i in 0..(*count as usize) {
        if recent_kings[i] == new_king {
            existing_idx = Some(i);
            break;
        }
    }
    
    if let Some(idx) = existing_idx {
        // King already in list - shift everything from 0..idx right, put new_king at front
        for i in (1..=idx).rev() {
            recent_kings[i] = recent_kings[i - 1];
        }
        recent_kings[0] = new_king;
        // Count stays the same
    } else {
        // New king not in list - shift everything right, insert at front
        // If we already have 3, the oldest (index 2) gets pushed out
        recent_kings[2] = recent_kings[1];
        recent_kings[1] = recent_kings[0];
        recent_kings[0] = new_king;
        // Increment count up to max 3
        if *count < 3 {
            *count += 1;
        }
    }
}

// ============================================================================
// RUN IT UP MODE ECONOMICS
// ============================================================================

/// Calculate run it up mode split - returns (refund, dev, beast, yield_add, jackpot)
/// Note: Old king gets REFUND ONLY (no profit), yield is calculated separately
/// Taxes are calculated on profit_delta (new_price - old_price), not old_price
pub fn calculate_run_it_up_split(old_price: u64, new_price: u64) -> (u64, u64, u64, u64, u64) {
    let refund = old_price;
    
    // Calculate profit delta (the 12% increase)
    let profit_delta = new_price.saturating_sub(old_price);
    
    // Use checked arithmetic to prevent overflow
    // Taxes are now based on profit_delta, not old_price
    let dev = profit_delta
        .checked_mul(SURPLUS_DEV)
        .and_then(|v| v.checked_div(1000))
        .unwrap_or(0);
    let beast = profit_delta
        .checked_mul(SURPLUS_BEAST)
        .and_then(|v| v.checked_div(1000))
        .unwrap_or(0);
    let yield_add = profit_delta
        .checked_mul(SURPLUS_YIELD)
        .and_then(|v| v.checked_div(1000))
        .unwrap_or(0);
    let jackpot = profit_delta
        .checked_mul(SURPLUS_JACKPOT)
        .and_then(|v| v.checked_div(1000))
        .unwrap_or(0);
    
    (refund, dev, beast, yield_add, jackpot)
}

// ============================================================================
// HIT A LICK MODE ECONOMICS
// ============================================================================

/// Calculate hit a lick entry cost: hit_a_lick_price + 12% surcharge (VIP pays no surcharge)
pub fn calculate_hit_a_lick_entry_cost(hit_a_lick_price: u64, is_vip: bool) -> u64 {
    if is_vip && VIP_NO_SURCHARGE {
        // VIP pays no surcharge
        hit_a_lick_price
    } else {
        let surcharge = hit_a_lick_price
            .checked_mul(HIT_A_LICK_SURCHARGE)
            .and_then(|v| v.checked_div(1000))
            .unwrap_or(0);
        hit_a_lick_price.saturating_add(surcharge)
    }
}

/// Calculate hit a lick old king payout: 90% refund of hit_a_lick_price
pub fn calculate_hit_a_lick_old_king_payout(hit_a_lick_price: u64) -> u64 {
    hit_a_lick_price
        .checked_mul(HIT_A_LICK_OLD_KING_REFUND)
        .and_then(|v| v.checked_div(1000))
        .unwrap_or(0)
}

/// Calculate hit a lick overhead split - returns (dev, beast, yield, jackpot)
/// New rules: 5% dev, 5% jackpot, 0% beast, 0% yield
pub fn calculate_hit_a_lick_overhead_split(hit_a_lick_price: u64) -> (u64, u64, u64, u64) {
    // Use checked arithmetic to prevent overflow
    let dev = hit_a_lick_price
        .checked_mul(HIT_A_LICK_DEV)
        .and_then(|v| v.checked_div(1000))
        .unwrap_or(0);
    let beast = 0; // No beast fee in Hit A Lick mode
    let yield_add = 0; // No yield pool in Hit A Lick mode
    let jackpot = hit_a_lick_price
        .checked_mul(HIT_A_LICK_JACKPOT)
        .and_then(|v| v.checked_div(1000))
        .unwrap_or(0);
    (dev, beast, yield_add, jackpot)
}

// ============================================================================
// YIELD MECHANICS (Run It Up Mode Only)
// ============================================================================

/// Calculate time bonus: 1.0 → 2.0 over 30 seconds
/// Returns value out of 1000 (1000 = 1.0x, 2000 = 2.0x)
pub fn calculate_time_bonus(hold_time_seconds: u64) -> u64 {
    // Linear interpolation: 1.0 at 0s, 2.0 at 30s, capped at 2.0
    let bonus_factor = hold_time_seconds.min(YIELD_TIME_BONUS_CAP_SECS) * 1000 / YIELD_TIME_BONUS_CAP_SECS;
    1000 + bonus_factor  // 1000 (1.0x) + up to 1000 (another 1.0x)
}

/// Calculate yield based on new payment and time held
/// raw_yield = 1% of new_payment × time_bonus
/// VIP gets double yield (multiplied by VIP_YIELD_MULTIPLIER)
pub fn calculate_yield(new_payment: u64, hold_time_seconds: u64, is_vip: bool) -> u64 {
    let time_bonus = calculate_time_bonus(hold_time_seconds);
    // raw_yield = 1% of new_payment × time_bonus
    // = new_payment × 0.01 × (1.0 to 2.0)
    // = new_payment × YIELD_BASE_RATE / 1000 × time_bonus / 1000
    // Use checked arithmetic to prevent overflow
    let base_yield = new_payment
        .checked_mul(YIELD_BASE_RATE)
        .and_then(|v| v.checked_div(1000))
        .and_then(|v| v.checked_mul(time_bonus))
        .and_then(|v| v.checked_div(1000))
        .unwrap_or(0); // On overflow, return 0 (safe fallback)
    
    // VIP gets double yield
    if is_vip {
        base_yield
            .checked_mul(VIP_YIELD_MULTIPLIER)
            .unwrap_or(base_yield) // On overflow, return base yield
    } else {
        base_yield
    }
}

/// Calculate yield with all caps applied
pub fn calculate_yield_with_cap(
    yield_pool: u64,
    entry_price: u64,
    new_payment: u64,
    hold_time_seconds: u64,
    is_vip: bool
) -> u64 {
    let raw_yield = calculate_yield(new_payment, hold_time_seconds, is_vip);
    
    // Cap 1: max 50% ROI on entry price (use checked arithmetic)
    let cap_by_entry = entry_price
        .checked_mul(YIELD_CAP)
        .and_then(|v| v.checked_div(1000))
        .unwrap_or(0); // On overflow, cap at 0 (safe fallback)
    
    // Cap 2: can't exceed yield pool
    let cap_by_pool = yield_pool;
    
    raw_yield.min(cap_by_entry).min(cap_by_pool)
}

// ============================================================================
// STEAL EXECUTION
// ============================================================================

pub fn execute_steal(game: &mut GameState) -> Result<()> {
    if game.is_hit_a_lick_mode {
        // Hit A Lick mode: price frozen, timer reset to 30 seconds
        game.hit_a_lick_end_time = HIT_A_LICK_TIMER;
        
        // Add yield and jackpot from overhead split
        let (_, _, yield_add, jackpot_add) = calculate_hit_a_lick_overhead_split(game.hit_a_lick_price);
        game.yield_pool += yield_add;
        game.pending_jackpot += jackpot_add;
    } else {
        // Run It Up mode: store old price before updating
        let old_price = game.current_price;
        
        // Increase price by 12% (use checked arithmetic)
        game.current_price = game.current_price
            .checked_mul(RUN_IT_UP_RATE)
            .and_then(|v| v.checked_div(100))
            .unwrap_or(u64::MAX); // On overflow, cap at max (unlikely but safe)
        
        // Add 10 seconds (cap at 10 minutes)
        game.time_remaining += TIMER_ADD;
        if game.time_remaining > MAX_TIMER {
            game.time_remaining = MAX_TIMER;
        }
        
        // Calculate new price
        let new_price = game.current_price;
        
        // Update jackpot and yield pool from run it up split
        let (_, _, _, yield_add, jackpot_add) = calculate_run_it_up_split(old_price, new_price);
        game.yield_pool += yield_add;
        game.pending_jackpot += jackpot_add;
        
        // Increment growth steals counter
        game.growth_steals = game.growth_steals.saturating_add(1);
        
        // Check if we should transition to hit a lick mode
        // Use current jackpot (after this steal's contribution) for price threshold check
        if game.jackpot_balance > 0 {
            // Use checked arithmetic to prevent overflow
            let hit_a_lick_threshold_price = game.jackpot_balance
                .checked_mul(HIT_A_LICK_THRESHOLD)
                .and_then(|v| v.checked_div(100))
                .unwrap_or(u64::MAX); // On overflow, use max (will never trigger hit a lick)
            
            // Price triggers when current price >= 60% of current jackpot (no minimum threshold)
            let price_triggers = game.current_price >= hit_a_lick_threshold_price;
            // Steals trigger when growth steals >= minimum required
            let steals_trigger = game.growth_steals >= game.min_growth_steals_for_war;
            
            // Hit A Lick mode triggers when EITHER price threshold (60% of jackpot) OR minimum steals are met
            // This allows flame bar (price progress) to trigger hit a lick mode when full
            if price_triggers || steals_trigger {
                trigger_hit_a_lick_mode(game);
            }
        }
    }
    
    // Set a new king (use a deterministic pubkey for testing)
    game.current_king = Some(Pubkey::default());
    game.king_since = 0; // Would be set to current timestamp in real implementation
    
    Ok(())
}

pub fn execute_dethrone(
    game: &mut GameState,
    current_time: u64,
    new_payment: u64,
    old_king_was_vip: bool
) -> (u64, u64) {
    // Calculate hold time
    let hold_time = current_time.saturating_sub(game.king_since);
    
    // Calculate refund and yield based on mode
    if game.is_hit_a_lick_mode {
        // Hit A Lick mode: 90% refund of what the player actually paid (king_entry_price)
        // This correctly handles transition case where player paid Run It Up price
        // CRITICAL: Use king_entry_price, not hit_a_lick_price, because the player
        // may have paid the Run It Up price if they triggered the HAL transition
        let refund = calculate_hit_a_lick_old_king_payout(game.king_entry_price);
        (refund, 0)
    } else {
        // Run It Up mode: refund of entry + yield
        // Refund is old_price (what they paid)
        let refund = game.king_base_price;
        
        // Calculate yield based on new payment and hold time
        // VIP old king gets double yield
        let yield_payout = calculate_yield_with_cap(
            game.yield_pool,
            game.king_entry_price,
            new_payment,
            hold_time,
            old_king_was_vip
        );
        
        // Deduct yield from pool
        if yield_payout > 0 {
            game.yield_pool = game.yield_pool.saturating_sub(yield_payout);
        }
        
        (refund, yield_payout)
    }
}

pub fn execute_steal_with_accounting(game: &mut GameState) -> (u64, u64) {
    // Store old price before executing steal
    let old_price = game.current_price;
    
    // Calculate what will come in (new price after steal)
    // Note: This is a test function, so we assume normie (not VIP)
    let incoming = if game.is_hit_a_lick_mode {
        calculate_hit_a_lick_entry_cost(game.hit_a_lick_price, false)
    } else {
        // Use checked arithmetic
        game.current_price
            .checked_mul(RUN_IT_UP_RATE)
            .and_then(|v| v.checked_div(100))
            .unwrap_or(u64::MAX)
    };
    
    let total_outgoing = if game.current_king.is_some() {
        if game.is_hit_a_lick_mode {
            calculate_hit_a_lick_old_king_payout(game.hit_a_lick_price)
        } else {
            // Refund is old_price
            old_price
        }
    } else {
        0
    };
    
    // Execute the steal (this updates price and pools)
    let _ = execute_steal(game);
    
    let contract_delta = incoming.saturating_sub(total_outgoing);
    (total_outgoing, contract_delta)
}

// ============================================================================
// ROUND END FUNCTIONS
// ============================================================================

/// Check if this is a mega hit a lick (pot >= 50 SOL)
pub fn is_mega_hit_a_lick(total_pot: u64) -> bool {
    total_pot >= MEGA_POT_THRESHOLD
}

/// Calculate hit a lick end payouts for 3 winners
/// Returns (winner1, winner2, winner3, dev, beast, rollover)
/// - winner1: 8% normal / 20% mega (1st place - most recent king)
/// - winner2: 3% normal / 7% mega (2nd place)
/// - winner3: 1% normal / 3% mega (3rd place)
pub fn calculate_hit_a_lick_end_payouts(total_pot: u64) -> (u64, u64, u64, u64, u64, u64) {
    if is_mega_hit_a_lick(total_pot) {
        // Mega Hit A Lick: 20/7/3% winners, 3% dev, 1% beast, 66% rollover
        // Use checked arithmetic to prevent overflow
        let winner1 = total_pot
            .checked_mul(END_WINNER_1_MEGA)
            .and_then(|v| v.checked_div(1000))
            .unwrap_or(0);
        let winner2 = total_pot
            .checked_mul(END_WINNER_2_MEGA)
            .and_then(|v| v.checked_div(1000))
            .unwrap_or(0);
        let winner3 = total_pot
            .checked_mul(END_WINNER_3_MEGA)
            .and_then(|v| v.checked_div(1000))
            .unwrap_or(0);
        let dev = total_pot
            .checked_mul(END_DEV_MEGA)
            .and_then(|v| v.checked_div(1000))
            .unwrap_or(0);
        let beast = total_pot
            .checked_mul(END_BEAST_MEGA)
            .and_then(|v| v.checked_div(1000))
            .unwrap_or(0);
        let rollover = total_pot
            .checked_mul(END_ROLLOVER_MEGA)
            .and_then(|v| v.checked_div(1000))
            .unwrap_or(0);
        (winner1, winner2, winner3, dev, beast, rollover)
    } else {
        // Normal Hit A Lick: 15/6/4% winners, 0% dev, 0% beast, 75% rollover
        // Use checked arithmetic to prevent overflow
        let winner1 = total_pot
            .checked_mul(END_WINNER_1)
            .and_then(|v| v.checked_div(1000))
            .unwrap_or(0);
        let winner2 = total_pot
            .checked_mul(END_WINNER_2)
            .and_then(|v| v.checked_div(1000))
            .unwrap_or(0);
        let winner3 = total_pot
            .checked_mul(END_WINNER_3)
            .and_then(|v| v.checked_div(1000))
            .unwrap_or(0);
        let dev = total_pot
            .checked_mul(END_DEV)
            .and_then(|v| v.checked_div(1000))
            .unwrap_or(0);
        let beast = total_pot
            .checked_mul(END_BEAST)
            .and_then(|v| v.checked_div(1000))
            .unwrap_or(0);
        let rollover = total_pot
            .checked_mul(END_ROLLOVER)
            .and_then(|v| v.checked_div(1000))
            .unwrap_or(0);
        (winner1, winner2, winner3, dev, beast, rollover)
    }
}

pub fn execute_round_end(game: &mut GameState) {
    // Merge pending jackpot into jackpot balance for next round
    game.jackpot_balance = game.jackpot_balance.saturating_add(game.pending_jackpot);
    game.pending_jackpot = 0;
    
    // Reset game state
    game.current_price = START_PRICE;
    game.hit_a_lick_price = 0;
    game.is_hit_a_lick_mode = false;
    game.time_remaining = MAX_TIMER;
    game.hit_a_lick_end_time = 0;
    game.current_king = None;
    game.king_since = 0;
    game.king_entry_price = 0;
    game.king_base_price = 0;
    game.recent_kings = [Pubkey::default(); 3];
    game.recent_kings_count = 0;
    game.growth_steals = 0;
    game.min_growth_steals_for_war = compute_min_growth_steals_for_war(game.jackpot_balance);
    game.growth_hard_end_ts = 0;  // Will be set on next round init
}

// ============================================================================
// SAFETY FUNCTIONS
// ============================================================================

pub fn safe_add_to_jackpot(game: &mut GameState, amount: u64) -> Result<()> {
    match game.jackpot_balance.checked_add(amount) {
        Some(new_balance) => {
            game.jackpot_balance = new_balance;
            Ok(())
        }
        None => Err(StealError::Overflow.into())
    }
}

