# Snapshot QA

Use `tools/snapshot_qa.py` to summarize raw BGA `gameui.gamedatas` captures and normalized
`GameSnapshotV1` JSON from `harmonies-cli normalize`.

Primary fixture priority is 2-player Side A with Nature Spirit enabled. Side B and 3-4 player
captures are useful for compatibility, but should not drive search-performance tradeoffs.

## Commands

```powershell
cargo run -q -p harmonies-cli -- normalize snapshots\raw\turn.json player_1 `
  > snapshots\normalized\turn.json

python tools\snapshot_qa.py snapshots\raw\turn.json snapshots\normalized\turn.json
python tools\snapshot_qa.py --json --compare snapshots\raw\turn.json snapshots\normalized\turn.json
python -m tools.score_qa snapshots\raw\final.json --expected player_1=84 --expected player_2=79
```

Default output is human-readable. `--json` output is intended for fixture logs and `jq`.

## What It Checks

- Snapshot kind: raw BGA object or normalized `schemaVersion` output.
- Top-level counts: players, board hexes/cells, non-empty cells, locked cells, tokens, active cards,
  completed cards, river cards, central token groups.
- Per-player counts: cells, non-empty cells, locked cells, tokens, active cards, completed cards,
  `emptyHexes`.
- Raw-to-normalized comparison when one raw file and one normalized file are provided, or when
  `--compare RAW NORMALIZED` is used.
- Warnings for malformed token stacks and unknown token `type_arg` values.

## Capture Checklist

- Capture exact `window.gameui.gamedatas` after BGA UI finishes updating.
- Record table context separately: date, player count, board side, turn phase, active player,
  notable action just completed.
- Anonymize before sharing or committing fixture data:

```powershell
python tools\snapshot_anonymizer.py snapshots\raw\turn.json snapshots\raw\turn.anonymized.json
```

- Normalize anonymized snapshot with explicit perspective player when possible.
- Run snapshot QA against raw/anonymized input and normalized output.
- Save QA JSON beside fixture or in `logs\snapshot_qa\`.
- Do not commit personal names, avatars, table IDs, chat, or unrelated BGA payload.

## Required Fixture Types

- Setup or first-turn snapshot with empty boards.
- Active-player post-placement snapshot with non-empty `tokensOnBoard`.
- Multi-player snapshot where active player differs from perspective player.
- Snapshot with `animalCubesOnBoard` locking at least one occupied board cell.
- Snapshot with animal cube locations inside `cubesOnAnimalCards` using `cell_...`.
- Snapshot with one or more completed animal cards in `doneAnimalCards`.
- Snapshot with in-progress animal cards and card cubes still on `card_[id]`.
- Nature Spirit snapshot assigned to a player via `spiritsCards`.
- Side B snapshot to verify board-side normalization.
- Numeric BGA player IDs mapped to anonymized `player_1` style IDs.
- Near-endgame snapshot with low `emptyHexes`.
- Negative fixture with missing or malformed optional fields for QA warnings.
- Side A 2p final/post-game snapshot with exact BGA final totals for both players.

## Known Caveats

- Raw active-card count includes player `boardAnimalCards` plus spirit cards whose `location_arg`
  exactly matches player key. Numeric/anonymized ID mismatches can make raw spirit counts lower than
  normalized counts.
- Raw per-player locked-cell count reports `animalCubesOnBoard`; total locked-cell count also
  includes global `cubesOnAnimalCards` entries with `cell_...` locations.
