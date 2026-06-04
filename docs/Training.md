# Training

Primary target: 2-player Side A, Nature Spirit enabled.

Do not run self-play tuning until scorer parity passes against Side A 2p BGA final-score fixtures.
Current gate: five tracked Side A 2p Nature Spirit BGA finals in `fixtures/score_parity`, covering
high/low river use, completed animal cards, and Spirit scores.

```powershell
python -m tools.score_fixture_corpus
```

## Current Weight Schema

Baseline:

```powershell
gc docs\weights.baseline.json
```

Fields:

- `selfScorePercent`: weight for our projected score.
- `opponentDenialPercent`: weight for visible opponent value denied by taking a central group.

Utility:

```text
(selfScore * selfScorePercent + opponentDenial * opponentDenialPercent) / 100
```

## Candidate Generation

```powershell
python -m tools.train_weights --out temp\training\weight_candidates.jsonl
```

After fixture corpus passes:

```powershell
python -m tools.train_weights --validated-scorer --out temp\training\weight_candidates.jsonl
```

The output is JSONL candidate weights. Later self-play will consume this schema directly and export a
chosen `weights.json`.

## Candidate Evaluation

Smoke:

```powershell
python -m tools.evaluate_weights --skip-score-gate --max-candidates 1 --max-turns 1 --turn-budget-ms 100 --out temp\training\evaluation-smoke.json
```

Longer CPU run:

```powershell
python -m tools.evaluate_weights `
  --candidates temp\training\weight_candidates.jsonl `
  --seeds 1,2,3,4,5 `
  --max-turns 20 `
  --turn-budget-ms 250 `
  --out temp\training\evaluation.json
```

Fitness is `perspective player score - mean(opponents)` from Rust self-play reports. Current fixtures
are active Side A 2p Nature Spirit snapshots, including first-turn Spirit choice and late active turn.

## Self-Play Smoke

The Rust simulator can replay from a normalized or raw BGA snapshot. Use this only for smoke tests
until scorer parity passes.

```powershell
cargo run -q -p harmonies-cli -- self-play snapshots\raw\side-a-near-end.json `
  --catalog docs\cards_database.json `
  --weights docs\weights.baseline.json `
  --max-turns 4 `
  --turn-budget-ms 250
```

After fixture corpus passes, add `--validated-scorer` and raise turn budget / turn cap for tuning.

## Later Training Plan

- Use Rust simulator for legal playouts.
- Parallelize by process/thread on CPU; GPU not needed.
- Tune Side A 2p first.
- Fitness: `score_self - score_opp`.
- Start with grid evaluation over cheap eval weights, then add CMA-ES/richer feature weights only
  after replay tests are stable.

## Hardware Notes

Intel 12600K / 32 GB RAM is enough for background CPU self-play. Use most P-cores first, keep memory
bounded by streaming match summaries to JSONL, and checkpoint candidate weights regularly. AMD GPU is
not useful for current tree-search/eval-weight tuning plan.
