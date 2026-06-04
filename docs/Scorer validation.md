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
- compact `domSnapshot` of board/card/cube/score/result nodes, useful when the final page visually
  updates but `window.gameui.gamedatas` lags by one action
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

If a result-page capture has no `gamedatas` but has `domSnapshot`, convert the visible DOM first:

```powershell
python -m tools.dom_capture_to_snapshot temp\snapshots\final-capture.json --out temp\normalized-final-dom.json
python -m tools.score_qa temp\normalized-final-dom.json --expected 96117860=116 --expected 85751928=113
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

Do not run weight training from these older captures alone. Their `gamedatas` state is stale against
the final result. Use DOM-converted v2 captures or fresh live-table captures for parity.

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
- Spectate match 5 file `1780551928691` (post-game; screenshot label focused-penguin/erick1nm):
  - Red/focused-penguin718/player `99949041`: terrain 9 leaf + 13 mountain + 5 water = 27;
    cubes 10 + 0 + 5 + 4 + 10 + 14 + 16 = 59; total 86.
  - Blue/erick1nm/player `97666166`: terrain 15 leaf + 17 mountain + 5 brick + 2 water = 39;
    cubes 12 + 5 + 3 + 3 + 2 + 11 = 36; total 75.
- Spectate match 6 file `1780551814613` (post-game; screenshot label Sinhtodzau/ausiting01):
  - Red/Sinhtodzau/player `99878348`: terrain 19 leaf + 10 brick + 5 water = 34;
    cubes 19 + 5 + 6 + 4 + 15 = 49; total 83.
  - Green/ausiting01/player `98852546`: terrain 8 leaf + 11 mountain + 5 field + 5 brick + 11 water = 40;
    cubes 16 + 4 + 10 + 15 = 45; total 85.
- Spectate match 7 file `1780552026416` (post-game; screenshot label clever-horse/bonjeanski):
  - Red/clever-horse391/player `97145325`: terrain 24 leaf + 10 field + 2 water = 36;
    cubes 24 + 0 + 0 + 0 + 0 + 17 + 15 = 56; total 92.
  - Yellow/bonjeanski/player `84085655`: terrain 9 leaf + 8 mountain + 5 field + 5 brick + 8 water = 35;
    cubes 18 + 6 + 9 + 14 = 47; total 82.

## New Capture Notes

The v2 capture payloads for matches 5-7 include `storedLatest`, but current and stored states both
show `gameEnd` for post-game captures. BGA also visually displays the final board/card state on the
post-game page. Do not assume all post-game board states are stale anymore. Treat each mismatch as an
open scorer/normalizer bug until proven otherwise.

Current scorer checks against these newer post-game captures:

- `1780551814613` match 6 after odd-q geometry + Spirit 36 fixes: expected `98852546=85`,
  `99878348=83`; current scorer `59`, `46`. Both players have `score_aux` / captured cube-count
  mismatches.
- `1780551928691` match 5 after odd-q geometry + Spirit 36 fixes: expected `99949041=86`,
  `97666166=75`; current scorer `83`, `55`. Blue has `score_aux` / captured cube-count mismatch.
- `1780552026416` match 7 after tall-tree fix: expected `97145325=92`, `84085655=82`;
  current scorer `92`, `76`.

Match 7 is useful: red side is exact after tall-tree score was corrected to 7. After odd-q geometry,
yellow terrain also matches BGA, but the player still misses 6 card points because BGA result says
yellow `score_aux=7` while captured board has only 6 cubes. This points to a capture-state gap for
that player, not a confirmed scorer bug.

- Spectate match 8 file `1780554171467` (post-game DOM-only capture; no `gamedatas` or
  `storedLatest`):
  - Green/Pezonloc0/player `96117860`: terrain 12 leaf + 21 mountain + 10 field + 10 brick +
    19 water = 72; cubes 14 spirit card 42 + 5 + 0 + 13 + 12 = 44; total 116.
  - Red/Emilie91180/player `85751928`: terrain 18 leaf + 14 mountain + 15 field + 10 brick +
    11 water = 68; cubes 16 spirit card 36 + 6 + 10 + 13 = 45; total 113.
  - DOM converter parity passes exactly after two fixes:
    - BGA coordinates use odd-q column offsets, not odd-r row offsets.
    - Spirit 36 scores 3 points for trees of height 1 or 2, and 1 point for height 3.

- Spectate match 9 file `1780575420889` (post-game DOM-only capture):
  - Red/andreamonaldini/player `98816549`: terrain 10 leaf + 13 mountain + 5 field +
    10 brick + 11 water = 49; cubes 13 + 12 + 18 + 12 = 55; total 104.
  - Green/Luegi/player `85116272`: terrain 11 leaf + 17 mountain + 5 field + 19 water = 52;
    cubes 12 + 10 + 4 + 10 = 36; total 88.
  - DOM converter parity passes exactly.

- Spectate match 10 file `1780575835733` (post-game DOM-only capture):
  - Red/ages60/player `99809470`: terrain 13 leaf + 5 mountain + 5 field + 5 brick +
    0 water = 28; cubes 14 + 5 + 14 + 17 + 18 = 68; total 96.
  - Green/cocogil/player `95645953`: terrain 18 leaf + 6 mountain + 5 brick + 5 water = 34;
    cubes 16 + 5 + 13 + 13 + 11 + 12 = 70; total 104.
  - DOM converter parity passes exactly after supported-building scoring:
    - Red placement can start on empty; BGA final board shows legal single Red tokens.
    - A scoring Building landscape requires Red on Red/Brown/Grey plus 3 adjacent top colors.
    - Single Red is legal terrain but scores 0 in the Building category.

Current exact parity gate:

```powershell
python -m tools.score_qa temp\normalized-match8-dom.json --expected 96117860=116 --expected 85751928=113
python -m tools.score_qa temp\normalized-match9-dom.json --expected 98816549=104 --expected 85116272=88
python -m tools.score_qa temp\normalized-match10-dom.json --expected 99809470=96 --expected 95645953=104
```
