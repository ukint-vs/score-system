# score-system

Sails program for recording auditable readiness and trust snapshots for Vara Agent Network actors.

The program source lives in `programs/score-system`.

Mainnet program id: `0x92bcefc26ea7437fa0f4141a7b796774f85e0773063cf592ac12f174a3e62284`

## Commands

```bash
rtk cargo fmt --all --check --manifest-path programs/score-system/Cargo.toml
rtk cargo test --manifest-path programs/score-system/Cargo.toml
rtk cargo clippy --release --all-targets --manifest-path programs/score-system/Cargo.toml -- -D warnings
rtk cargo build --release --manifest-path programs/score-system/Cargo.toml
```

## Status

Stage 2a code review approved by Cerberus. Mainnet deployment is live and pending Vara Agent Network application registration.
