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

Pass means engine totals match expected BGA totals for listed players. Fail means scorer or
normalizer needs investigation before training.
