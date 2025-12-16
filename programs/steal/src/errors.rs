// ============================================================================
// STEAL ERRORS
// ============================================================================

use anchor_lang::prelude::*;

#[error_code]
pub enum StealError {
    #[msg("Invalid token mint")]
    InvalidMint,
    #[msg("Integer overflow")]
    Overflow,
    #[msg("Round has ended - call end_round")]
    RoundEnded,
    #[msg("Round has not ended yet")]
    RoundNotEnded,
    #[msg("Insufficient funds to steal")]
    InsufficientFunds,
    #[msg("No king to pay out")]
    NoKing,
    #[msg("Round has a king - use end_round instead")]
    HasKing,
    #[msg("Invalid old king account - must match current king")]
    InvalidOldKing,
    #[msg("King must hold crown for 3 seconds to win Hit A Lick")]
    KingMustHoldLonger,
    #[msg("Invalid winner account - must match recent_kings")]
    InvalidWinner,
    #[msg("Must wait before stealing again")]
    RateLimitExceeded,
    #[msg("Vault has insufficient balance for payout")]
    InsufficientVaultBalance,
    #[msg("Invalid account provided")]
    InvalidAccount,
    #[msg("Invalid token account - must be owned by Token Program")]
    InvalidTokenAccount,
    #[msg("Invalid token account owner - must match player")]
    InvalidTokenAccountOwner,
    #[msg("Dev or beast wallet must be funded with at least rent-exempt minimum (0.00089 SOL)")]
    WalletNotRentExempt,
    #[msg("Season has not started yet")]
    SeasonNotStarted,
}
