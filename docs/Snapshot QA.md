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
python -m tools.score_qa snapshots\raw\final-capture.json --use-capture-scores
python -m tools.score_fixture_corpus
python -m tools.summarize_capture_visible_state temp\snapshots\capture.json
python -m tools.build_advisor_request_fixture temp\snapshots\capture.json `
  fixtures\advisor_requests\case_request.json
python -m tools.validate_advisor_plan_legality
python -m tools.benchmark_cli --threads 12 --time-budget-ms 30000
python -m tools.benchmark_cli fixtures\advisor_requests\case_request.json --threads 12 `
  --time-budget-ms 30000 --future-beam 10 --future-branch 5 `
  --refill-samples 2 --card-refill-samples 1
python -m tools.sweep_search_params --time-budget-ms 30000 --threads 12
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
- Optional helper: install `tools\bga_harmonies_capture.user.js` in ScriptCat/Tampermonkey and
  click `Download`. Current panel should show `v0.3.3`; stale panels are replaced automatically.
- Record table context separately: date, player count, board side, turn phase, active player,
  notable action just completed.
- Anonymize before sharing or committing fixture data:

```powershell
python tools\snapshot_anonymizer.py snapshots\raw\turn.json snapshots\raw\turn.anonymized.json
```

- Normalize anonymized snapshot with explicit perspective player when possible.
- Run snapshot QA against raw/anonymized input and normalized output.
- For advisor legality fixtures, prefer `tools.build_advisor_request_fixture` over manual JSON edits.
  It converts visible DOM captures when present and anonymizes player ids in `GameSnapshotV1`.
- DOM/capture conversion infers bag counts from visible board/central tokens and
  `gamedatas.remainingTokens`. Check `bagCounts` is non-zero before using a fixture for future-search
  benchmarks.
- Run `tools.validate_advisor_plan_legality` after adding active-turn request fixtures. It replays
  group selection, token placement, card draft, settlement source, remaining cubes, locked cells,
  and catalog pattern validity.
- Save QA JSON beside fixture or in `logs\snapshot_qa\`.
- Do not commit personal names, avatars, table IDs, chat, or unrelated BGA payload.
- For final-score parity, record BGA final totals manually if capture `scoreHints` are empty or use
  BGA numeric ids that do not match normalized/anonymized player ids.
- Tracked final-score parity fixtures live in `fixtures\score_parity\`. Keep them anonymized,
  normalized `GameSnapshotV1` files with expected totals in `manifest.json`.

## Required Fixture Types

- Setup or first-turn snapshot with empty boards.
- Active-player post-placement snapshot with non-empty `tokensOnBoard`.
- Multi-player snapshot where active player differs from perspective player.
- Snapshot with `animalCubesOnBoard` locking at least one occupied board cell.
- Snapshot with animal cube locations inside `cubesOnAnimalCards` using `cell_...`.
- Snapshot with one or more completed animal cards in `doneAnimalCards`.
- Snapshot with in-progress animal cards and card cubes still on `card_[id]`.
- First-turn Nature Spirit offer with two unchosen `spiritsCards` and no cubes.
- Nature Spirit snapshot after selection, where chosen Spirit is in active cards with one cube.
- Side B snapshot to verify board-side normalization.
- Numeric BGA player IDs mapped to anonymized `player_1` style IDs.
- Near-endgame snapshot with low `emptyHexes`.
- Active turn with full 4-card hand before any action.
- Active turn after that same player previously completed a card and freed a hand slot.
- Negative fixture with missing or malformed optional fields for QA warnings.
- Side A 2p final/post-game snapshot with exact BGA final totals for both players.

## Known Caveats

- Raw active-card count includes player `boardAnimalCards`; unchosen Spirit offers stay separate as
  `spiritCardChoices` until one is selected. A selected Spirit has a cube on `card_[id]` and counts
  as an active card.
- Raw per-player locked-cell count reports `animalCubesOnBoard`; total locked-cell count also
  includes global `cubesOnAnimalCards` entries with `cell_...` locations.
