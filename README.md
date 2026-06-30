# score-system

Sails program for recording auditable readiness and trust snapshots for Vara Agent Network actors.

The program source lives in `programs/score-system`. Deployed program details and the callable surface are documented in `skills.md`.

## How scoring works

`score-system` is an attested score ledger, not a grant treasury. Configured reviewers call `Score/RecordScore` to append a full score snapshot for an actor. Each snapshot stores the five dimensions, computed overall score, confidence, evidence hash, reason code, formula version, attester, and timestamp.

The score dimensions are `proposal_quality`, `delivery_reliability`, `integration_count`, `past_impact_score`, and `community_signal`. Overall score is computed as:

```text
proposal_quality * 30%
+ delivery_reliability * 25%
+ integration_count * 20%
+ past_impact_score * 15%
+ community_signal * 10%
```

Consumers use `Score/GetScoreSnapshot(actor)` for the latest snapshot and `Score/GetScoreHistory(actor, cursor, limit)` for append-only history. Missing actors return `null`; the program does not custody VARA, pay milestones, or accept permissionless score writes.

## Commands

```bash
rtk cargo fmt --all --check --manifest-path programs/score-system/Cargo.toml
rtk cargo test --manifest-path programs/score-system/Cargo.toml
rtk cargo clippy --release --all-targets --manifest-path programs/score-system/Cargo.toml -- -D warnings
rtk cargo build --release --manifest-path programs/score-system/Cargo.toml
```

## Status

Foundation review approved the application for listing. Mainnet deployment is live.
