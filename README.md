# Steal

On-chain Solana program for [crownclash.xyz](https://crownclash.xyz)

## Program ID

```
CM9y2DreJSMqzoRRrLkWEZzTB9ve5D4gQHcPPxrw8mxg
```

## Verification

This repository contains the source code for verifying the deployed program.

```bash
solana-verify verify-from-repo \
  --program-id CM9y2DreJSMqzoRRrLkWEZzTB9ve5D4gQHcPPxrw8mxg \
  https://github.com/starstarstarstarstarstarstarstar/steal \
  --mount-path programs/steal
```

## Build

```bash
anchor build
```

## License

All rights reserved.
