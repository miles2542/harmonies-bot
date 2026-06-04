# Scorer Validation

Primary target: 2-player Side A, Nature Spirit enabled.

Training must wait until scorer parity passes against BGA final scores. If scorer differs, training
optimizes wrong game.

## Needed Fixtures

- Side A 2p midgame, active participant turn, before any turn action.
- Side A 2p near endgame, active participant turn, before any turn action.
- Side A 2p with animal/spirit settlement available (`possibleCards` non-empty if BGA exposes it).
- Side A 2p final/post-game snapshot plus BGA final score for each player.

Spectated snapshots are useful for scorer parity. Participant snapshots are additionally needed for
active-player/perspective fields and extension gating.

## Capture Userscript

Install in ScriptCat/Tampermonkey:

```text
tools/bga_harmonies_capture.user.js
```

On a BGA table it adds a small `Harmonies Capture` panel. Use `Download` after BGA finishes
updating. For final-score parity, capture after the final score is visible.

Payload includes:

- raw `window.gameui.gamedatas`
- board side / active player / remaining tokens
- best-effort `scoreHints` from `gamedatas.players` and score-like DOM nodes

## Score Command

```powershell
cargo run -q -p harmonies-cli -- score temp\snapshots\example.json --catalog docs\cards_database.json
```

Optional raw snapshot perspective override:

```powershell
cargo run -q -p harmonies-cli -- score temp\snapshots\example.json --perspective player_1
```

## Exact Total Check

```powershell
python -m tools.score_qa temp\snapshots\example.json --expected player_1=84 --expected player_2=79
```

If capture `scoreHints` contain matching player ids:

```powershell
python -m tools.score_qa temp\snapshots\final-capture.json --use-capture-scores
```

If BGA result data is present in `gamedatas.gamestate.args.result`:

```powershell
python -m tools.score_qa temp\snapshots\final-capture.json --use-bga-result
```

Pass means engine totals match expected BGA totals for listed players. Fail means scorer or
normalizer needs investigation before training.

## Current Side A Fixture Status

Known failing captures in `temp\snapshots`:

- `harmonies-gamedatas-1780544344340 (real active participant side A post-game result).json`:
  `player_1` expected 101, current 103; `player_2` passes at 93.
- `harmonies-gamedatas-1780545283771 (same spectate game X but captured post-game in result page).json`:
  expected 112 / 103, current 106 / 95.
- `harmonies-gamedatas-1780545388024 (spectate 2p side A nature spirit post-game capture).json`:
  expected 117 / 114, current 112 / 89.

Do not run weight training until these pass or the scorer discrepancy is explained by a documented
capture/result mismatch.
