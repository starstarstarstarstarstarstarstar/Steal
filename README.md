# Steal

A competitive on-chain game on Solana where players compete to hold the crown and win the pot.

**Website:** [crownclash.xyz](https://crownclash.xyz)

## Official Links

| Platform | Link |
|----------|------|
| Website | https://crownclash.xyz |
| Discord | crownclash |

## Deployed Program

| Field | Value |
|-------|-------|
| Network | Solana Mainnet |
| Program ID | [`CM9y2DreJSMqzoRRrLkWEZzTB9ve5D4gQHcPPxrw8mxg`](https://solscan.io/account/CM9y2DreJSMqzoRRrLkWEZzTB9ve5D4gQHcPPxrw8mxg) |
| ProgramData | [`DuvWyyrSbpSW8Mvsz1FpYP67nK4z11e8kot2ufuMB97L`](https://solscan.io/account/DuvWyyrSbpSW8Mvsz1FpYP67nK4z11e8kot2ufuMB97L) |

## Build Requirements

| Dependency | Version |
|------------|---------|
| Rust | 1.75.0+ |
| Solana CLI | 1.18.0+ |
| Anchor | 0.31.0 |

## Build Instructions

1. Clone the repository:
```bash
git clone https://github.com/starstarstarstarstarstarstarstar/steal.git
cd steal
```

2. Install dependencies:
```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install Solana CLI
sh -c "$(curl -sSfL https://release.anza.xyz/stable/install)"

# Install Anchor
cargo install --git https://github.com/coral-xyz/anchor avm --locked
avm install 0.31.0
avm use 0.31.0
```

3. Build the program:
```bash
anchor build
```

4. The compiled program will be at:
```
target/deploy/steal.so
```

## Verification

Verify the deployed program matches this source code using [solana-verify](https://github.com/Ellipsis-Labs/solana-verifiable-build):

```bash
solana-verify verify-from-repo \
  --program-id CM9y2DreJSMqzoRRrLkWEZzTB9ve5D4gQHcPPxrw8mxg \
  https://github.com/starstarstarstarstarstarstarstar/steal \
  --mount-path programs/steal
```

## Program Overview

Steal is a competitive timer-based game with two phases:

### Run It Up Phase
- Players pay an increasing price to steal the crown
- Each steal adds time to the countdown
- Price increases 12% per steal
- Crown holder earns yield based on hold time

### Hit A Lick Phase (War Mode)
- Triggered when price reaches 60% of pot or minimum steals reached
- Fixed entry price (3% of pot)
- 10-second timer resets on each steal
- Last 3 crown holders split the pot when timer expires

### Instructions

| Instruction | Description |
|-------------|-------------|
| `initialize_game` | Initialize a new game round |
| `initialize_config` | Set up program configuration |
| `steal` | Steal the crown from current holder |
| `end_round` | End round and pay winners |
| `reset_round` | Reset a round with no winner |

## Security

For security concerns, please review our [Security Policy](SECURITY.md).

**Responsible Disclosure:**
- Discord: crownclash
- Do not open public issues for vulnerabilities

## License

All rights reserved. This source code is provided for verification purposes only.
