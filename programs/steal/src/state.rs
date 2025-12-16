// ============================================================================
// STEAL STATE - A+ RULESET
// ============================================================================

#[cfg(any(not(test), feature = "idl-build"))]
use anchor_lang::prelude::*;

/// On-chain game account stored as PDA
#[cfg(any(not(test), feature = "idl-build"))]
#[account]
#[derive(Default)]
pub struct GameAccount {
    /// Current run it up price to steal the crown
    pub current_price: u64,
    /// Hit A Lick mode ticket price (3% of pot, clamped)
    pub hit_a_lick_price: u64,
    /// Jackpot pool balance
    pub jackpot_balance: u64,
    /// Pending jackpot (accumulated during current round, feeds into next round)
    pub pending_jackpot: u64,
    /// Yield pool for king earnings
    pub yield_pool: u64,
    /// Timestamp when round ends (Unix timestamp)
    pub round_end_time: i64,
    /// Hit A Lick-specific end time (3 minute countdown)
    pub hit_a_lick_end_time: i64,
    /// Whether game is in hit a lick mode
    pub is_hit_a_lick_mode: bool,
    /// Current king's wallet (None if no king yet)
    pub current_king: Pubkey,
    /// Timestamp when current king took the crown
    pub king_since: i64,
    /// Price the current king paid to enter (includes lazy tax if normie)
    pub king_entry_price: u64,
    /// Base price king paid (without lazy tax, for correct refund calculation)
    pub king_base_price: u64,
    /// Whether there is a king (since Pubkey can't be Option in Anchor)
    pub has_king: bool,
    /// Dev wallet for fee collection
    pub dev_wallet: Pubkey,
    /// Beast wallet for buyback/burn
    pub beast_wallet: Pubkey,
    /// STEAL token mint address
    pub steal_mint: Pubkey,
    /// Round number
    pub round: u64,
    /// Total steals this round
    pub total_steals: u64,
    /// Total $STEAL tokens burned by beast (in smallest units)
    pub total_burned: u64,
    /// SOL accumulated in beast wallet pending buyback
    pub beast_sol_pending: u64,
    /// Last 3 unique kings during War (Hit A Lick) mode - index 0 is most recent
    pub recent_kings: [Pubkey; 3],
    /// Number of valid entries in recent_kings (0-3)
    pub recent_kings_count: u8,
    /// Wallet that last stole (for rate limiting)
    pub last_steal_wallet: Pubkey,
    /// Timestamp of last steal (for rate limiting)
    pub last_steal_time: i64,
    /// Cooldown duration in seconds (for rate limiting in Hit A Lick mode)
    pub cooldown_seconds: u64,
    /// Counter of steals in Run It Up mode
    pub growth_steals: u16,
    /// Dynamic per-round minimum steals required before war can trigger
    pub min_growth_steals_for_war: u16,
    /// Timestamp when growth phase must end (hard cap)
    pub growth_hard_end_ts: i64,
    /// Whether current king was VIP (for yield calculation)
    pub king_was_vip: bool,
    /// Season 1 start timestamp (Unix). Paid steals blocked until this time.
    pub season_start_time: i64,
    /// Bump seed for PDA
    pub bump: u8,
}

#[cfg(any(not(test), feature = "idl-build"))]
impl GameAccount {
    /// Size of the account in bytes
    pub const SIZE: usize = 8 + // discriminator
        8 + // current_price
        8 + // hit_a_lick_price (NEW)
        8 + // jackpot_balance
        8 + // pending_jackpot
        8 + // yield_pool
        8 + // round_end_time
        8 + // hit_a_lick_end_time (NEW)
        1 + // is_hit_a_lick_mode
        32 + // current_king
        8 + // king_since
        8 + // king_entry_price
        8 + // king_base_price
        1 + // has_king
        32 + // dev_wallet
        32 + // beast_wallet
        32 + // steal_mint
        8 + // round
        8 + // total_steals
        8 + // total_burned
        8 + // beast_sol_pending
        (32 * 3) + // recent_kings[3]
        1 + // recent_kings_count
        32 + // last_steal_wallet
        8 + // last_steal_time
        8 + // cooldown_seconds
        2 + // growth_steals
        2 + // min_growth_steals_for_war
        8 + // growth_hard_end_ts
        1 + // king_was_vip
        8 + // season_start_time
        1; // bump
}

/// On-chain config account stored as PDA
#[cfg(any(not(test), feature = "idl-build"))]
#[account]
pub struct GameConfig {
    /// Authority that can update this config
    pub authority: Pubkey,
    /// Dev wallet for fee collection
    pub dev_wallet: Pubkey,
    /// Beast wallet for buyback/burn
    pub beast_wallet: Pubkey,
    /// STEAL token mint address
    pub steal_mint: Pubkey,
    /// Bump seed for PDA
    pub bump: u8,
}

#[cfg(any(not(test), feature = "idl-build"))]
impl GameConfig {
    /// Size of the account in bytes
    pub const SIZE: usize = 8 + // discriminator
        32 + // authority
        32 + // dev_wallet
        32 + // beast_wallet
        32 + // steal_mint
        1; // bump
}

/// In-memory game state for logic calculations (used by tests)
#[cfg(test)]
use anchor_lang::prelude::Pubkey;

#[derive(Debug, Clone, Default)]
pub struct GameState {
    pub current_price: u64,
    pub hit_a_lick_price: u64,
    pub jackpot_balance: u64,
    pub pending_jackpot: u64,
    pub yield_pool: u64,
    pub time_remaining: u64,
    pub hit_a_lick_end_time: u64,
    pub is_hit_a_lick_mode: bool,
    pub current_king: Option<Pubkey>,
    pub king_since: u64,
    pub king_entry_price: u64,
    pub king_base_price: u64,
    /// Last 3 unique kings during War mode - index 0 is most recent
    pub recent_kings: [Pubkey; 3],
    /// Number of valid entries in recent_kings (0-3)
    pub recent_kings_count: u8,
    /// Wallet that last stole (for rate limiting)
    pub last_steal_wallet: Option<Pubkey>,
    /// Timestamp of last steal (for rate limiting)
    pub last_steal_time: u64,
    /// Cooldown duration in seconds (for rate limiting in Hit A Lick mode)
    pub cooldown_seconds: u64,
    /// Counter of steals in Run It Up mode
    pub growth_steals: u16,
    /// Dynamic per-round minimum steals required before war can trigger
    pub min_growth_steals_for_war: u16,
    /// Timestamp when growth phase must end (hard cap)
    pub growth_hard_end_ts: i64,
}
