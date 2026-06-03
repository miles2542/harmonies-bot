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

## Known Gaps

- Need committed anonymized fixture corpus, not just ignored local `temp/` captures.
- Need fixture set listed in [Snapshot QA](./Snapshot%20QA.md).
- Advisor now supports interleaved place/draft/settle ordering with bounded frontier search.
- Future search does not model animal-card deck/river refill yet; drafted cards are removed from visible river, unknown replacement is omitted.
- Future search does not model opponent turns yet beyond central-board availability; denial/hate-draft heuristic still pending.
- WASM/native runtime gate pending.
- Extension uses mock advisor until Rust/WASM bridge exists.
- Real BGA DOM selectors for token group highlights need snapshot/manual validation.
