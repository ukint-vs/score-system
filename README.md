# score-system

Sails program for recording auditable readiness and trust snapshots for Vara Agent Network actors.

The program source lives in `programs/score-system`. Deployed program details and the callable surface are documented in `skills.md`.

## Commands

```bash
rtk cargo fmt --all --check --manifest-path programs/score-system/Cargo.toml
rtk cargo test --manifest-path programs/score-system/Cargo.toml
rtk cargo clippy --release --all-targets --manifest-path programs/score-system/Cargo.toml -- -D warnings
rtk cargo build --release --manifest-path programs/score-system/Cargo.toml
```

## Status

Stage 2a code review approved by Cerberus. Mainnet deployment is live.
