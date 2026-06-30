## The **score-system** program

[![Build Status](https://github.com/ukint-vs/score-system/workflows/CI/badge.svg)](https://github.com/ukint-vs/score-system/actions)

Program **score-system** for [⚙️ Gear Protocol](https://github.com/gear-tech/gear) written in [⛵ Sails](https://github.com/gear-tech/sails) framework.

`score-system` records reviewer-attested readiness snapshots for Vara Agent Network actors. A snapshot stores five score dimensions, computed overall score, confidence, evidence hash, reason code, formula version, attester, and timestamp. History is append-only.

Overall score:

```text
proposal_quality * 30%
+ delivery_reliability * 25%
+ integration_count * 20%
+ past_impact_score * 15%
+ community_signal * 10%
```

The program exposes public snapshot/history queries and attester-only writes. It does not custody VARA or run grant payouts.

The program workspace includes the following packages:
- `score-system` is the package allowing to build WASM binary for the program and IDL file for it.
  The package also includes integration tests for the program in the `tests` sub-folder
- `score-system-app` is the package containing business logic for the program represented by the `ScoreSystem` structure.
- `score-system-client` is the package containing the client for the program allowing to interact with it from another program, tests, or off-chain client.

### 🏗️ Building

```bash
rtk cargo build --release --manifest-path programs/score-system/Cargo.toml
```

### ✅ Testing

```bash
rtk cargo test --manifest-path programs/score-system/Cargo.toml
```

### Formatting

```bash
rtk cargo fmt --all --check --manifest-path programs/score-system/Cargo.toml
```

> [!TIP]
> For off-chain integration tests against a running node, add the `gclient` feature:

```bash
rtk cargo add sails-rs --dev --features gclient
```

# License

The source code is licensed under the [MIT license](LICENSE).
