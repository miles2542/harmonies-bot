# New Chat Handoff - 2026-06-06

## Project Goal

Passive Firefox advisor for BGA Harmonies.

Scope:

- Read active Harmonies table state.
- Analyze best move sequence for current turn.
- Show text + visual overlays on BGA page.
- No BGA action calls, no clicks, no automation.
- Primary optimization target: 2-player Side A, Nature Spirit enabled.
- Side B / 3-4p: correctness-compatible bonus, not performance priority.

Fair-play constraint: read-only local advisor only. No stealth, no ban evasion, no server-action automation.

## Current State

Repo clean at handoff.

Latest commits:

- `639b34b docs: record training smoke status`
- `ef85fd9 docs: update live qa search profile`
- `99d4a83 perf: cache turn frontier scoring`
- `9b00686 fix: harden capture QA workflow`
- `331016f test: validate advisor request fixtures`

Main implemented pieces:

- Rust workspace:
  - `harmonies-core`: rules, model, scoring, card matching, current-turn generator, future search.
  - `harmonies-cli`: JSON request runner, normalize, score, self-play.
  - `harmonies-service`: native localhost service with HTTP + WebSocket progress.
- Firefox extension:
  - Manual `Analyze`.
  - Spectator mode analyzes current active player.
  - Active participant mode analyzes only user's own active turn.
  - DOM-first visible state capture at Analyze click.
  - Collapsible/selectable plan tiers.
  - Central group highlight, board cell markers, settlement card-to-cell arrows.
- QA tools:
  - Snapshot/capture summary.
  - Score parity corpus.
  - Advisor request fixture builder/checker.
  - Advisor plan legality replay.
  - Benchmarks and search-param sweeps.
  - Extension safety scanner.

## Critical Facts Learned

DOM is preferred source where reliable.

BGA `gamedatas` can lag:

- Central groups can be empty/previous group after refill.
- Player board cells can be stale after previous turn.
- Card source state can be stale/misleading in some live cases.

Use DOM for:

- central token groups
- player board cells
- active hand cards
- completed cards
- river cards
- Spirit choices
- visible cube counts

ID meaning:

- BGA DOM `#card_<id>` and `data-card-id` are per-game card instance ids.
- `data-card-type-arg` is persistent card catalog id.
- Engine/advisor `cardId` means per-game instance id.
- Engine/advisor `typeArg` means persistent card catalog id.
- Overlay arrows must locate DOM card by `cardId`, not `typeArg`.

Draft means:

- Optional once per turn.
- Take one visible river/common animal card into active hand.
- Only legal if active hand has fewer than 4 cards at that moment.
- A drafted card can be settled later in same plan only after that explicit draft action.
- River/opponent/completed cards cannot be settled unless drafted in the same plan.

Settlement constraints:

- Only current player's active cards in 4 hand slots.
- Respect card remaining cube count.
- Completed/done cards have 0 cubes, never settlement candidates.
- Cube target cell must match card pattern, be occupied, and not already locked by cube.
- Completion auto-moves card out of hand; frees hand slot.

## Capture Tool

Script: `tools/bga_harmonies_capture.user.js`

Current version: `v0.3.4`.

Expected panel:

- `Harmonies Capture`
- `Copy`
- `Download`
- status includes `v0.3.4`

Fixes:

- Payload includes `scriptVersion`.
- Status updates before heavy JSON build.
- Download leaves visible `Save` fallback link if browser blocks auto-download.

When user tests:

- Update ScriptCat with latest file.
- Refresh BGA page.
- Confirm panel says `v0.3.4`.
- If Download does not open modal, click `Save`.
- If still broken, send exact console error + panel status.

Useful captures still needed:

- Spectator active turn, no action yet, active player hand full 4 cards.
- Spectator active turn with visible settlement opportunity before cube placed.
- Near-end active turn, no action yet.
- Freed-slot case: same player next active turn after card completed and moved down.

Spectate captures preferred over real active matches unless active-player-only behavior is being tested.

## Current Live QA Startup

Use release service:

```powershell
$env:HARMONIES_WEIGHTS='docs\weights.baseline.json'
$env:HARMONIES_SEARCH_THREADS='12'
$env:HARMONIES_FUTURE_BEAM='10'
$env:HARMONIES_FUTURE_BRANCH='5'
$env:HARMONIES_REFILL_SAMPLES='2'
$env:HARMONIES_CARD_REFILL_SAMPLES='1'
$env:HARMONIES_HARD_STOP_MARGIN_MS='3000'
$env:HARMONIES_MIN_FUTURE_EXPAND_MS='1500'
cargo run --release -p harmonies-service
```

Load extension:

- Firefox `about:debugging#/runtime/this-firefox`
- Load temporary add-on
- Select `extension/manifest.json`

## Immediate Next QA

Use cheap spectated match first.

Check:

- Panel says active player name/id correctly.
- `Analyze` freezes current visible state.
- Central group tokens match visible board.
- Suggested token placements are legal on visible board.
- No field/water on occupied cell.
- No foliage on non-trunk stack except legal foliage-on-trunk.
- Settlement arrows start from active player's hand cards only.
- No arrows from opponent hand, river, completed area.
- If active hand is full, no draft unless plan completes a card first and frees slot.
- `Best so far` plan is the one to follow at a fixed time cutoff.

If mismatch:

- Download `v0.3.4` capture immediately.
- Take screenshot with panel and board visible.
- Note whether active participant or spectator.
- Note expected correction in plain language.

## Existing Verification

Recent gates passed:

```powershell
cargo test
python -m tools.validate_advisor_plan_legality
python -m tools.check_advisor_request_fixtures
python -m tools.extension_safety_check
python -m tools.service_smoke
node --check tools\bga_harmonies_capture.user.js extension\src\visibleState.js extension\src\normalizer.js extension\src\contentScript.js extension\src\overlay.js
node tools\check_extension_normalizer_visible_state.js
python -m py_compile tools\*.py
```

Legality validator can now batch captures:

```powershell
python -m tools.validate_advisor_plan_legality --capture temp\snapshots\a.json temp\snapshots\b.json --time-budget-ms 10000 --max-results 10
```

Latest match14 active captures passed broader legality replay. Post-game capture is skipped as `gameEnd`.

Score parity corpus passes for tracked Side A 2p Nature Spirit finals.

## Search / Performance

Current search progress includes:

- `depthCompleted`
- `nodesEvaluated`
- `rootGenerationMs`
- `rootSequencesGenerated`
- `stoppedEarly`

Extension panel shows root telemetry as:

```text
root <ms>/<seq> seq
```

Important perf fix:

- Current-turn frontier sort now caches scoring.
- Match14 full-hand root generation dropped about `7.1s -> 2.9s`.
- First answer on same fixture dropped about `9.2s -> 3.7s`.

Current best small sweep profile:

- `aggressive_narrow`
- full-hand: about `11.6s`, depth 4, future 139
- near-end: about `4.1s`, depth 4, future 129

Search still not fully CPU-saturating. Likely because fan-out is pruned/narrow, not because Rayon threads are unavailable. Do not chase raw CPU usage until correctness/live QA is clean.

## Training Status

Training is not production-ready.

Current:

- `tools.train_weights` creates simple denial-weight grid candidates.
- `tools.evaluate_weights` runs Rust self-play over request fixtures.
- Short smoke passed after scorer parity.

Pending:

- Real CMA-ES or broader search over feature weights.
- Larger fixture corpus.
- Stable live QA proving state extraction and legality.
- More seeds, longer self-play, and parameter comparisons.

Do not change production weights based only on current smoke.

## Roadmap To Complete Extension

### Phase 1 - Live State Correctness

Goal: prove extension reads exactly what BGA shows.

Tasks:

- Spectator QA with `v0.3.4` captures.
- Active participant QA after spectator QA is clean.
- Confirm DOM card extraction returns `domCards=true` on fresh captures.
- Confirm active hand/river/done parsing by screenshots + capture summaries.
- Fix any mismatch before tuning.

Exit criteria:

- Several spectated active turns pass visual inspection.
- At least one active participant turn passes same checks.
- No invalid placements/settlements observed.
- Captures from mismatches replay legal or expose fixed bugs.

### Phase 2 - Fixture Corpus Expansion

Goal: enough offline data to reduce manual live retesting.

Tasks:

- Add more anonymized `AdvisorRequestV1` fixtures:
  - full hand
  - settlement available
  - near-end
  - freed slot
  - first-turn Spirit choice
  - possibly Side B bonus
- Add exact screenshots/notes for tricky captures in `dev_docs` or docs.
- Run:
  - `tools.build_advisor_request_fixture`
  - `tools.check_advisor_request_fixtures`
  - `tools.validate_advisor_plan_legality`

Exit criteria:

- Fixture suite covers main live failure modes.
- Legality replay passes on all.

### Phase 3 - Search Tuning

Goal: pick stable search parameters for 30s/50s/100s budgets.

Tasks:

- Expand `tools.sweep_search_params` candidates.
- Measure p50/p95 over fixture corpus.
- Track top-group/top-plan stability.
- Decide default live QA profile and maybe expose panel settings later.

Exit criteria:

- Stable first answer <= 10-15s.
- Better/deeper plan by ~30s.
- Final best-so-far by ~50-100s.
- No obvious plan oscillation for same frozen state except better utility replacement.

### Phase 4 - Evaluation Weights / Training

Goal: tune utility weights after correctness is trusted.

Tasks:

- Expand weight features beyond current simple self score + denial.
- Add CMA-ES or similar optimizer.
- Run self-play over validated fixtures/seeds.
- Compare against baseline with same search knobs.
- Do not optimize against bad/incomplete simulation data.

Exit criteria:

- Candidate weights beat baseline across fixture/seeds.
- No overfit to tiny corpus.
- Score/rule parity remains green.

### Phase 5 - Extension UX Polish

Goal: usable during real BGA fast games.

Tasks:

- Better panel layout/settings.
- Plan selection UX refinements.
- Clear current active player name/id.
- Better card labels for settlement/draft.
- Optional hide/show visual layers.
- Better error states for service unavailable / stale state / not own turn.

Exit criteria:

- User can follow plan without DevTools.
- Overlay never blocks key BGA info.
- Manual Analyze/Stop flow clear.

### Phase 6 - Runtime Packaging

Goal: decide native service vs WASM extension-only.

Current:

- Native service exists and works.
- WASM runtime gate pending.

Tasks:

- Benchmark WASM core if implemented.
- Compare against native on same fixture corpus.
- If WASM cannot hit p95 budget, keep optional native service.

Exit criteria:

- Chosen runtime documented.
- Setup steps reliable.
- No unsupported hidden dependency.

### Phase 7 - Final QA

Goal: call extension complete.

Required:

- Rule/scorer tests green.
- Score parity corpus green.
- Advisor legality corpus green.
- Extension safety scanner green.
- Service smoke green.
- Live spectator QA clean.
- Live active participant QA clean.
- Performance acceptable on target hardware.
- Documentation complete enough to reinstall/run from clean checkout.

## Docs To Read In New Chat

Read in this order:

1. `docs/New chat handoff - 2026-06-06.md`
2. `docs/Implementation status.md`
3. `docs/orion_handoff.md`
4. `docs/Snapshot QA.md`
5. `extension/README.md`
6. `docs/Game rules.md` only when touching rules/scoring.
7. `docs/BGA game structure.md` only when touching extractor/parser.

## Files Most Likely To Touch Next

- `tools/bga_harmonies_capture.user.js`
- `extension/src/visibleState.js`
- `extension/src/normalizer.js`
- `extension/src/contentScript.js`
- `extension/src/overlay.js`
- `crates/harmonies-core/src/turn.rs`
- `crates/harmonies-core/src/search.rs`
- `tools/validate_advisor_plan_legality.py`
- `tools/dom_capture_to_snapshot.py`
- `tools/sweep_search_params.py`
- `tools/evaluate_weights.py`

## Guardrails

- Do not tune weights/search until live state correctness passes.
- Do not trust raw `gamedatas` over reliable visible DOM.
- Do not use DOM `card_<id>` as persistent catalog id.
- Do not settle cards outside active hand unless explicitly drafted in same plan.
- Do not mutate BGA DOM nodes for overlays.
- Do not add automation/click/action calls.
- Keep commits small and focused.
- Update this handoff or `docs/orion_handoff.md` before context-heavy work ends.
