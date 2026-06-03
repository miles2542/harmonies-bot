# Implementation Status

## Done

- Project-local Rust workspace scaffold.
- `harmonies-core` model, placement rules, card parser, pattern matcher, scoring, baseline advisor.
- Raw BGA `gameui.gamedatas` normalizer into `GameSnapshotV1`.
- `harmonies-cli` JSON request runner plus `normalize` command.
- Firefox extension scaffold: page bridge, content script, overlay, mock advisor.
- Python utility scripts for snapshot anonymization and CLI benchmark.

## Known Gaps

- Need real active-player post-turn snapshot with non-empty `tokensOnBoard` for regression fixture.
- Advisor uses greedy legal baseline, not beam search yet.
- WASM/native runtime gate pending.
- Extension uses mock advisor until Rust/WASM bridge exists.
- Real BGA DOM selectors for token group highlights need snapshot/manual validation.
