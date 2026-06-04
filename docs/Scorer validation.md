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
- latest live-table `storedLatest.gamedatas` cached in session storage, useful if result page
  `gamedatas` is stale or missing after redirect
- board side / active player / remaining tokens
- best-effort `scoreHints` from `gamedatas.players` and score-like DOM nodes
- best-effort `visibleScoreText` snippets from result/score DOM nodes

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

Known result-page captures in `temp\snapshots` whose `gamestate.args.result` totals do not match
their embedded board/card state:

- `harmonies-gamedatas-1780544344340 (real active participant side A post-game result).json`:
  expected 101 / 93, current 87 / 90 from embedded board.
- `harmonies-gamedatas-1780545283771 (same spectate game X but captured post-game in result page).json`:
  expected 112 / 103, current 91 / 87 from embedded board.
- `harmonies-gamedatas-1780545388024 (spectate 2p side A nature spirit post-game capture).json`:
  expected 117 / 114, current 99 / 67 from embedded board.
- `harmonies-gamedatas-1780550171318 (spectate match 3 side A 2p post game).json`:
  expected `player_2=117` red / `player_1=93` green, current 76 / 62 from embedded board.
- `harmonies-gamedatas-1780550221825 (spectate match 4 side A 2p post game).json`:
  expected `player_1=116` green / `player_2=86` red, current 69 / 56 from embedded board.

Do not run weight training until these pass or the scorer discrepancy is explained by a documented
capture/result mismatch.

Result-page captures can have final `gamestate.args.result` totals while the embedded board/card
`gamedatas` still reflects a pre-final state. When category totals imply impossible board facts
(example: Spirit 42 says 10 water tokens but captured board contains 9, or `score_aux` placed-cube
count is higher than completed plus active visible cubes), treat that capture as stale for scorer
parity and collect either a live-table final board capture, replay-board final capture, or the
userscript `storedLatest` payload.

## Manual Breakdown Notes

- Real active participant post-game (`1780544344340`):
  - Red/improver6/player_1: terrain 3 leaf + 14 mountain + 5 field + 19 water = 41;
    cubes 14 + 2 + 5 + 4 + 9 + 15 + 11 = 60; total 101.
  - Green/__bunny/player_2: terrain 23 leaf + 5 water = 28;
    cubes 37 + 6 + 0 + 5 + 17 = 65; total 93.
- Spectate match 1 (`1780545283771`):
  - Green/Lugorion/player_1: terrain 9 leaf + 13 mountain + 5 field + 15 brick + 5 water = 47;
    cubes 14 spirit card 41 + 0 + 5 + 6 + 8 + 16 + 16 = 65; total 112.
  - Red/atticusk13/player_2: terrain 11 leaf + 5 field + 10 brick + 5 water = 31;
    cubes 14 spirit card 42 + 6 + 8 + 16 + 17 + 11 = 72; total 103.
- Spectate match 2 (`1780545388024`):
  - Red/karlapvaleri/player_1: terrain 24 leaf + 8 mountains + 5 field + 5 brick + 8 water = 50;
    cubes 22 + 6 + 4 + 0 + 17 + 18 = 67; total 117.
  - Green/CuriousBoots/player_2: terrain 10 leaf + 10 field + 30 bricks + 8 water = 58;
    cubes 18 + 0 + 0 + 5 + 5 + 13 + 15 = 56; total 114.
- Spectate match 3 (`1780550171318`):
  - Red/truelylove/player_2: terrain 9 leaf + 4 mountain + 5 field + 15 brick + 31 water = 64;
    cubes 20 spirit + 5 + 15 + 13 = 53; total 117.
  - Green/elmerfrances/player_1: terrain 12 leaf + 10 mountain + 10 field + 5 brick + 2 water = 39;
    cubes 22 spirit + 4 + 5 + 11 + 12 = 54; total 93.
- Spectate match 4 (`1780550221825`):
  - Green/LucaSkullz/player_1: terrain 4 leaf + 14 mountain + 10 field + 10 brick + 23 water = 61;
    cubes 16 spirit + 0 + 12 + 11 + 16 = 55; total 116.
  - Red/Erin_26/player_2: terrain 10 leaf + 15 field + 15 brick + 5 water = 45;
    cubes 22 spirit + 0 + 10 + 0 + 9 = 41; total 86.
