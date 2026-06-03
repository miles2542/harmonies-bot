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

## Known Gaps

- Need committed anonymized fixture corpus, not just ignored local `temp/` captures.
- Need fixture set listed in [Snapshot QA](./Snapshot%20QA.md).
- Advisor evaluates legal current-turn branches with draft + settlement, but not multi-turn stochastic beam search yet.
- WASM/native runtime gate pending.
- Extension uses mock advisor until Rust/WASM bridge exists.
- Real BGA DOM selectors for token group highlights need snapshot/manual validation.
