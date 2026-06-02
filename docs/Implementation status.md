# Implementation Status

## Done

- Project-local Rust workspace scaffold.
- `harmonies-core` model, placement rules, card parser, pattern matcher, scoring, baseline advisor.
- `harmonies-cli` JSON request runner.
- Firefox extension scaffold: page bridge, content script, overlay, mock advisor.
- Python utility scripts for snapshot anonymization and CLI benchmark.

## Known Gaps

- BGA snapshot normalization from raw `gameui.gamedatas` into `GameSnapshotV1` still pending.
- Advisor uses greedy legal baseline, not beam search yet.
- WASM/native runtime gate pending.
- Extension uses mock advisor until Rust/WASM bridge exists.
- Real BGA DOM selectors for token group highlights need snapshot/manual validation.
