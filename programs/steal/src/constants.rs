// ============================================================================
// STEAL CONSTANTS - A+ RULESET
// ============================================================================

pub const LAMPORTS_PER_SOL: u64 = 1_000_000_000;

// ----------------------------------------------------------------------------
// INITIAL STATE
// ----------------------------------------------------------------------------
pub const START_PRICE: u64 = 20_000_000;                   // 0.02 SOL

// ----------------------------------------------------------------------------
// RUN IT UP MODE
// ----------------------------------------------------------------------------
pub const RUN_IT_UP_RATE: u64 = 112;                       // 12% = multiply by 1.12
pub const MAX_TIMER: u64 = 600;                            // 10 minutes
pub const TIMER_ADD: u64 = 30;                             // +30 seconds per steal

// Run It Up surplus splits (out of 1000 for precision, based on profit_delta)
// Total overhead = 100% of profit_delta (new_price - old_price)
pub const SURPLUS_DEV: u64 = 60;                           // 6% of profit_delta
pub const SURPLUS_BEAST: u64 = 5;                          // 0.5% of profit_delta
pub const SURPLUS_YIELD: u64 = 200;                        // 20% of profit_delta
pub const SURPLUS_JACKPOT: u64 = 735;                      // 73.5% of profit_delta
// Note: Old king gets FULL REFUND only (no profit), plus yield from yield_pool

// Run It Up minimum steals for war trigger
// Formula updated to prevent pot drainage: ensures jackpot growth offsets winner payouts
pub const MIN_GROWTH_STEALS_CLAMP: u16 = 35;              // Minimum growth steals (clamp floor) - prevents drainage
pub const MAX_GROWTH_STEALS_CLAMP: u16 = 100;             // Maximum growth steals (clamp ceiling)
pub const GROWTH_STEALS_BASE: u16 = 35;                   // Base steals (for small pots)
pub const GROWTH_STEALS_PER_SOL: u64 = 15;                // Additional steals per SOL in pot (prevents drainage)
pub const GROWTH_MAX_DURATION_SECS: i64 = 180;            // 3 minutes hard cap for growth phase

// ----------------------------------------------------------------------------
// HIT A LICK MODE TRIGGER
// ----------------------------------------------------------------------------
pub const HIT_A_LICK_THRESHOLD: u64 = 60;                 // 60% of jackpot triggers hit a lick

// ----------------------------------------------------------------------------
// HIT A LICK MODE PRICE
// ----------------------------------------------------------------------------
pub const HIT_A_LICK_PCT_OF_POT: u64 = 30;                 // 3% of pot (out of 1000)
pub const HIT_A_LICK_MIN: u64 = 50_000_000;                // 0.05 SOL minimum
pub const HIT_A_LICK_MAX: u64 = 1_500_000_000;            // 1.50 SOL maximum
pub const HIT_A_LICK_TIMER: u64 = 10;                      // 10 seconds in hit a lick
pub const MIN_HIT_A_LICK_HOLD: u64 = 3;                    // Minimum seconds king must hold to win in Hit A Lick
pub const HIT_A_LICK_COOLDOWN_SECS: u64 = 2;               // Default cooldown in seconds (1.5 rounded up to 2)

// Hit A Lick overhead splits (out of 1000, based on hit_a_lick_price)
// Old king gets 90% refund, 5% to dev, 5% to jackpot
pub const HIT_A_LICK_SURCHARGE: u64 = 120;                 // 12% surcharge (out of 1000) - for normies
pub const HIT_A_LICK_OLD_KING_REFUND: u64 = 900;           // 90% refund to old king (out of 1000)
pub const HIT_A_LICK_DEV: u64 = 50;                        // 5% dev cut
pub const HIT_A_LICK_BEAST: u64 = 0;                       // 0% beast cut (not used in Hit A Lick)
pub const HIT_A_LICK_YIELD: u64 = 0;                       // 0% to yield pool (not used in Hit A Lick)
pub const HIT_A_LICK_JACKPOT: u64 = 50;                    // 5% to jackpot

// ----------------------------------------------------------------------------
// HIT A LICK END / ROUND END
// ----------------------------------------------------------------------------
pub const MEGA_POT_THRESHOLD: u64 = 50_000_000_000;        // 50 SOL = mega hit a lick

// Normal Hit A Lick (pot < 50 SOL) - out of 1000
// 3 winners: 15% / 6% / 4% = 25% total
pub const END_WINNER_1: u64 = 150;                         // 15% to 1st place (most recent king)
pub const END_WINNER_2: u64 = 60;                          // 6% to 2nd place
pub const END_WINNER_3: u64 = 40;                          // 4% to 3rd place
pub const END_DEV: u64 = 0;                                // 0% to dev (all dev fees from entries)
pub const END_BEAST: u64 = 0;                              // 0% to beast (all beast fees from entries)
pub const END_ROLLOVER: u64 = 750;                         // 75% stays in pot

// Mega Hit A Lick (pot >= 50 SOL) - out of 1000
// 3 winners: 25% / 10% / 5% = 40% total
pub const END_WINNER_1_MEGA: u64 = 250;                    // 25% to 1st place (most recent king)
pub const END_WINNER_2_MEGA: u64 = 100;                    // 10% to 2nd place
pub const END_WINNER_3_MEGA: u64 = 50;                     // 5% to 3rd place
pub const END_DEV_MEGA: u64 = 0;                           // 0% to dev (all dev fees from entries)
pub const END_BEAST_MEGA: u64 = 0;                         // 0% to beast (all beast fees from entries)
pub const END_ROLLOVER_MEGA: u64 = 600;                    // 60% stays in pot

// ----------------------------------------------------------------------------
// YIELD (Run It Up Mode Only)
// ----------------------------------------------------------------------------
pub const YIELD_BASE_RATE: u64 = 50;                       // 5% of new_payment (was 1%)
pub const YIELD_TIME_BONUS_CAP_SECS: u64 = 30;             // Time bonus caps at 30 seconds
pub const YIELD_CAP: u64 = 500;                            // 50% of entry max ROI (out of 1000)
pub const MIN_YIELD: u64 = 10_000;                         // 0.00001 SOL minimum yield (covers tx fees)

// ----------------------------------------------------------------------------
// VIP / PRICING
// ----------------------------------------------------------------------------
pub const TOKEN_REQUIREMENT: u64 = 1_000_000_000_000;      // 1,000 $STEAL for VIP (with 9 decimals: 1000 * 10^9)
pub const VIP_YIELD_MULTIPLIER: u64 = 2;                   // VIP gets 2x yield (double yield)
pub const VIP_NO_SURCHARGE: bool = true;                   // VIP pays no surcharge in Hit A Lick mode
pub const NORMIE_RATE: u64 = 100;                          // Normie pays 100% of base price (1.0x)

// ----------------------------------------------------------------------------
// RENT EXEMPTION
// ----------------------------------------------------------------------------
pub const RENT_EXEMPT_MIN: u64 = 890_880;                  // 0.00089088 SOL minimum for rent exemption

