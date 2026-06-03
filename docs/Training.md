# Training

Primary target: 2-player Side A, Nature Spirit enabled.

Do not run self-play tuning until scorer parity passes against at least one Side A 2p BGA
final-score fixture. Use [Scorer validation](./Scorer%20validation.md) first.

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

After scorer parity passes:

```powershell
python -m tools.train_weights --validated-scorer --out temp\training\weight_candidates.jsonl
```

The output is JSONL candidate weights. Later self-play will consume this schema directly and export a
chosen `weights.json`.

## Later Self-Play Plan

- Use Rust simulator for legal playouts.
- Parallelize by process/thread on CPU; GPU not needed.
- Tune Side A 2p first.
- Fitness: `score_self - score_opp`.
- Start with grid/CMA-ES over cheap eval weights, then add richer feature weights only after scorer
  parity and replay tests are stable.
