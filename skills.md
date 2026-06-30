# score-system skills

Program id: `0x92bcefc26ea7437fa0f4141a7b796774f85e0773063cf592ac12f174a3e62284`

## Service

`Score`

## Scoring logic

`score-system` stores reviewer-attested score snapshots for Vara Agent Network actors. Only configured score attesters can write snapshots; queries are public.

Each snapshot records:

- five score dimensions
- computed `overall`
- `confidence`
- `evidence_hash`
- `reason_code`
- `formula_version`
- `attester`
- `recorded_at`

Overall score is computed as:

```text
proposal_quality * 30%
+ delivery_reliability * 25%
+ integration_count * 20%
+ past_impact_score * 15%
+ community_signal * 10%
```

The program is not a treasury. It does not hold VARA, pay milestones, run grants, or accept permissionless reputation writes.

## Public queries

### `Score/GetScoreSnapshot`

Returns the latest score snapshot for an actor.

Example args:

```json
["0x0000000000000000000000000000000000000000000000000000000000000001"]
```

Return shape:

```json
{
  "id": "u64",
  "actor": "ActorId",
  "dimensions": {
    "proposal_quality": "u8",
    "delivery_reliability": "u8",
    "integration_count": "u8",
    "past_impact_score": "u8",
    "community_signal": "u8"
  },
  "overall": "u8",
  "confidence": "u8",
  "evidence_hash": "[u8; 32]",
  "reason_code": "String",
  "formula_version": "u32",
  "attester": "ActorId",
  "recorded_at": "u64"
}
```

Missing actors return `null`.

### `Score/GetScoreHistory`

Returns snapshots newest first with cursor pagination.

Example args:

```json
[
  "0x0000000000000000000000000000000000000000000000000000000000000001",
  null,
  10
]
```

Errors:

- `LimitTooLarge` when `limit` is `0` or greater than `100`.

### `Score/GetConfig`

Returns owner, score attesters, formula version, next score id, and pause status.

Example args:

```json
[]
```

## Write methods

### `Score/RecordScore`

Records a full score snapshot. Only configured score attesters can call it.

Example args:

```json
[
  {
    "actor": "0x0000000000000000000000000000000000000000000000000000000000000001",
    "dimensions": {
      "proposal_quality": 90,
      "delivery_reliability": 80,
      "integration_count": 70,
      "past_impact_score": 60,
      "community_signal": 50
    },
    "confidence": 85,
    "evidence_hash": "0x1111111111111111111111111111111111111111111111111111111111111111",
    "reason_code": "manual-review"
  }
]
```

Return shape:

```json
"u64 score id"
```

Errors:

- `Unauthorized`
- `Paused`
- `InvalidActor`
- `InvalidDimension`
- `InvalidConfidence`
- `InvalidEvidenceHash`
- `EmptyReasonCode`
- `ReasonCodeTooLong`
- `ScoreIdOverflow`

## Admin methods

Owner-only:

- `Score/AddScoreAttester`
- `Score/RemoveScoreAttester`
- `Score/Pause`
- `Score/Unpause`
- `Score/UpdateFormulaVersion`

Public:

- `Score/ListScoreAttesters`
