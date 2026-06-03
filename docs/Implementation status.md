# Implementation Status

## Done

- Project-local Rust workspace scaffold.
- `harmonies-core` model, placement rules, card parser, pattern matcher, scoring, legal one-turn advisor.
- Raw BGA `gameui.gamedatas` normalizer into `GameSnapshotV1`.
- `harmonies-cli` JSON request runner plus `normalize` command.
- Firefox extension scaffold: page bridge, JS normalizer, content script, overlay, mock advisor.
- Python utility scripts for snapshot anonymization and CLI benchmark.
- Snapshot QA utility and capture checklist for raw/normalized fixture validation.
- Real Side B near-end snapshot validated locally: raw/normalized counts match.
- Current-turn advisor benchmarks under 30s first-answer target on rich Side B local fixture.
- Snapshot model now includes inferred `bagCounts` from official token distribution minus visible board/central tokens.
- Advisor now applies bounded stochastic future-turn search with deterministic token-refill samples (`K=50`, `N=3`, `M=10` target shape) after legal current-turn generation.
- Rich Side B local benchmark: 30s budget returns in ~18s; 48s budget returns in ~36s with future depth progress.
- `harmonies-service` native localhost service exposes `/health`, `/advise`, and `/ws`.
- Firefox extension streams `/ws` progress from local Rust service, with HTTP/mock fallback.
- Rich Side B WebSocket smoke: first streamed response ~8s, final 30s-budget response ~17.5s.
- Future search now refills drafted river slots by sampling unseen standard animal cards from catalog minus visible river/player cards.
- Optimization target clarified: 2-player Side A with Nature Spirit enabled. Side B/3-4p remain correctness-compatible bonuses, not search-performance priorities.
- `harmonies-cli score` and `tools.score_qa` added for scorer-vs-BGA final score parity checks.
- Advisor now computes a cheap visible-opponent denial estimate per central token group and ranks by utility.
- Eval weights are now externalized via `docs/weights.baseline.json`; service can load `HARMONIES_WEIGHTS`.
- `tools.train_weights` generates candidate weight JSONL for later validated self-play tuning.

## Known Gaps

- Need committed anonymized fixture corpus, not just ignored local `temp/` captures.
- Need fixture set listed in [Snapshot QA](./Snapshot%20QA.md).
- Need Side A 2p final-score parity fixture before training/tuning.
- Self-play simulator/CMA-ES evaluator still pending; only weight schema and candidate generation exist.
- Advisor now supports interleaved place/draft/settle ordering with bounded frontier search.
- Future search does not know exact hidden animal-card deck order; river replacements are sampled from unseen standard cards.
- Opponent handling is v1 heuristic only: visible current-board value for central token groups, not full opponent future search.
- WASM runtime gate pending; native service path exists.
- Extension uses native service when running, streaming WebSocket first; mock fallback otherwise.
- Real BGA DOM selectors for token group highlights need snapshot/manual validation.
