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
- Extension Stop button sends WebSocket stop command; Rust returns best-so-far at cancellation checkpoints.
- `tools/bga_harmonies_capture.user.js` adds one-click BGA snapshot download/copy for Side A scorer fixtures.
- `tools.score_qa --use-capture-scores` can compare against capture `scoreHints` when ids match.
- Golden scorer tests now cover Side A branching river, fields, mountains, buildings, Side B islands,
  and spirit scoring.
- DOM-converted Side A 2p Nature Spirit finals for matches 8, 9, 10, and real participant match 11
  now match BGA totals exactly.
- Red placement/scoring split verified from BGA: single Red placement is legal, but Building scoring
  requires Red on Red/Brown/Grey and 3 adjacent top colors.
- DOM converter handles BGA result-page own-board tokens with `token-*` ids and multiple `level-*`
  classes.
- Nature Spirit first-turn choice is now explicit in snapshots/advisor plans: unchosen offers live in
  `spiritCardChoices`, selected spirits live in `activeCards`, and plans emit `chooseSpirit` before
  token-group actions.
- Real participant match 12 raw active-turn captures validate `unsafeWindow.gameui.gamedatas`
  capture, river cards, central groups, bag counts, and first-turn Spirit-choice parsing.
- Card matcher tests now cover all six rotations, no mirror match, building alias, and catalog cube-target invariants.
- Extension read-only safety checker now scans JS, local endpoints, and manifest permissions.
- Rust self-play simulator can replay from raw/normalized BGA snapshots and apply advisor plans with
  token/card refills for smoke testing and future tuning.
- Tracked scorer parity corpus exists in `fixtures/score_parity`: five anonymized Side A 2p Nature
  Spirit BGA finals with exact expected totals. Run with `python -m tools.score_fixture_corpus`.
- Tracked active-turn advisor request fixtures exist in `fixtures/advisor_requests`, including first-turn
  Spirit choice and late active turn.
- `tools.service_smoke` starts `harmonies-service`, checks `/health`, checks cheap `/advise`, then
  verifies `/ws` returns a real Side A Spirit-choice plan.
- `tools.evaluate_weights` ranks candidate weights by validated Rust self-play fitness over tracked
  active-turn fixtures.
- Extension overlay now highlights verified BGA central holes (`#hole-1..5`) and marks recommended
  board cells (`#cell_<playerId>_<col>_<row>`) with action step badges.
- Extension advisor run is now manual: cached page state only, search starts on `Analyze`, `Stop`
  does not auto-restart, and same-page updates no longer trigger repeated searches.
- Extension central token groups prefer visible DOM tokens (`#hole-N-token-1..3`) when all five groups
  are readable, falling back to `gamedatas.tokensOnCentralBoard`.
- `tools/bga_harmonies_group_inspector.user.js` compares visible DOM central groups against
  `gamedatas` and labels holes for active/spectated parser QA.
- Extension `Analyze` now also works in spectator mode by freezing the clicked state and analyzing
  from the current active player's perspective through the same JS normalizer/native service path.
- Extension visual annotations are now separate fixed overlay elements. They no longer add classes,
  append markers, or change style on BGA board, cell, central-hole, or token nodes.
- Advisor WebSocket progress now emits after each completed future-search depth. The overlay keeps
  the first usable plan fixed and appends later streamed depth results as collapsible tiers.
- Native service initializes Rayon to `available_parallelism - 1` search threads by default, with
  `HARMONIES_SEARCH_THREADS` override for local tuning.
- Future search now parallelizes frontier-state expansion, not only root central-group evaluation.
- Extension search budget is now about 100s of Rust search time, with max future depth raised to 4.
- Extension plan panel is scrollable; every streamed plan is a named collapsible section, and selecting
  a plan switches the visual indicators to that plan.
- Plan visual indicators are document-anchored absolute overlays, so they stay on board cells/groups
  during page scrolling.
- Plan sections now mark the highest-utility plan seen so far with a `Best so far` badge. At a fixed
  user cutoff, follow that badge unless choosing a simpler plan intentionally.
- Settlement steps now draw a low-opacity card-to-cell arrow from the BGA per-game card DOM id
  (`#card_<cardId>`) to the target cell, plus a subtle card ring, so the user can see which card cube
  to settle without DevTools. Labels still show persistent card `typeArg`.
- Nature Spirit choice parsing is now gated by active-player `gamestate.args.canChooseSpirit` plus
  `actChooseSpirit`/`chooseSpirit`, avoiding stale `chooseSpirit` plans after the first-turn window.
- Group inspector labels now use a separate fixed overlay layer and stricter visible-DOM token reads,
  so inspector QA does not move or restyle central tokens.
- Active participant Analyze now only runs for the current user's own active turn. Spectator mode still
  analyzes the active player. Panel status includes player name plus id when BGA exposes the name.
- Extension advisor requests now override analyzed player's board cells from visible DOM tokens, because
  live QA showed `gamedatas.players[*].tokensOnBoard` can lag behind just like central token groups.

## Known Gaps

- Fixed in code, pending live QA: invalid card-cube settlement recommendations. Normalizers now require
  board/done card locations to match the owning player exactly, no longer infer player aliases from card
  arrays, ignore stale no-cube Spirit offers after the choice window, and card arrows find DOM cards by
  BGA per-game `cardId` while labels still show persistent `typeArg`. Regression tests and
  `tools.validate_advisor_plan_legality` cover unavailable/completed/undrafted/over-count settlements.
- Capture helper is now `v0.3.4`: payload includes `scriptVersion`, Download shows a persistent `Save`
  fallback link, and Copy/Download status updates before heavy JSON building starts.
- Extension visible-card reliability now requires at least one visible card DOM node. Empty hand
  containers alone no longer cause DOM card override to wipe fallback card state.
- `tools.validate_advisor_plan_legality --capture <captures...> --time-budget-ms 10000 --max-results 10`
  batch-validates capture files, skips `gameEnd` captures, and stresses wider result sets.
- Match 14 active-turn captures are now durable anonymized advisor request fixtures:
  `sidea_2p_nature_match14_full_hand_request.json` and
  `sidea_2p_nature_match14_after_completion_near_end_request.json`. They cover full-hand
  draft exclusion and freed-slot near-end search legality.
- `tools.build_advisor_request_fixture` converts capture/normalized snapshots into anonymized
  `AdvisorRequestV1` fixtures so future card-source bugs can be reproduced offline.
- `tools.check_advisor_request_fixtures` validates tracked request fixtures for anonymized player ids,
  no player metadata, non-zero `bagCounts`, five complete central groups, and active-card hand limit.
- Need broader fixture set listed in [Snapshot QA](./Snapshot%20QA.md) beyond final-score parity.
- Need more settlement-available active-turn fixtures if BGA exposes edge cases, but match 12 already
  covers early Spirit choice plus late active-turn search smoke.
- CMA-ES optimizer still pending; current tuning path is grid/candidate evaluation over validated
  Rust self-play.
- Early stop cancels between search phases/expansions, not inside one expensive current-turn generation.
- Advisor now supports interleaved place/draft/settle ordering with bounded frontier search.
- Future search does not know exact hidden animal-card deck order; river replacements are sampled from unseen standard cards.
- Opponent handling is v1 heuristic only: visible current-board value for central token groups, not full opponent future search.
- WASM runtime gate pending; native service path exists.
- Extension uses native service when running, streaming WebSocket first; mock fallback otherwise.
- Live BGA extension visual QA still pending after DOM-board override: confirm turn-after-turn active
  participant plans never place field/water/foliage on occupied field/water/foliage cells.
- Live BGA spectator QA pending for the new frozen Analyze flow: confirm active-player perspective,
  central-group parsing, non-mutating overlays, and progressive depth tiers on cheap spectated games
  before spending time on real active matches.
- Current CPU utilization is still low on i5-12600K despite `HARMONIES_SEARCH_THREADS=12`.
  Root and future-frontier parallelism exist, but search fan-out is likely still too narrow after
  pruning. Next work should benchmark node counts/frontier sizes and tune branch/refill/depth knobs
  before assuming a threading bug.
- Offline benchmark tooling now supports fixture corpus runs, `RAYON_NUM_THREADS`, time-budget
  override, wall/engine timing, node counts, depth, and top-group stability:
  `python -m tools.benchmark_cli --threads 12 --time-budget-ms 30000`.
- Parameter sweep harness added:
  `python -m tools.sweep_search_params --time-budget-ms 30000 --threads 12`.
- Native search knobs are now env-configurable for sweeps:
  `HARMONIES_ROOT_BEAM`, `HARMONIES_FUTURE_BEAM`, `HARMONIES_FUTURE_BRANCH`,
  `HARMONIES_FUTURE_DEPTH`, `HARMONIES_REFILL_SAMPLES`, `HARMONIES_CARD_REFILL_SAMPLES`,
  `HARMONIES_HARD_STOP_MARGIN_MS`, `HARMONIES_MIN_FUTURE_EXPAND_MS`.
- Search depth labels now count actual future turns. Earlier `depth 1` was zero future turns because
  `future_value` received `depth - 1`.
- DOM/capture conversion now infers `bagCounts` from visible board/central tokens plus BGA
  `remainingTokens`; old DOM fixtures had zero bag counts, making future refill expansion impossible.
- Match 14 full-hand benchmark after bag-count/depth fixes:
  - 30s budget: ~17.7s engine, 48,120 nodes, depth 1 complete, future estimate 104.
  - 100s budget: ~89.9s engine, 1,071,337 nodes, depth 1 complete, depth 2 partial, future estimate 115.
  - 30s aggressive narrow sweep (`futureBeam=10`, `futureBranch=5`, `refillSamples=2`,
    `cardRefillSamples=1`): ~25.9s, 8,800 nodes, depth 3 complete, future estimate 104.
- Match 14 near-end aggressive narrow sweep smoke: 15s budget, ~7.5s engine, 4,169 nodes,
  depth 4 complete.
- 30s two-fixture sweep report: `logs/benchmarks/search-param-sweep-match14-30s.json`.
  `balanced_narrow` currently best small-sweep candidate: full-hand future 115 at depth 2 partial,
  near-end future 129 with depth 4 complete.

## Next Phases

1. Benchmark/instrument search on spectated active-turn snapshots: elapsed per phase, root count,
   future frontier sizes, nodes per depth, Rayon thread count.
2. Parameter sweep: root beam, future turn beam, branch width, token refill samples, card refill
   samples, depth/time budget. Track p50/p95 and best-plan stability.
3. Tune weights using validated fixtures and self-play candidates, then compare against baseline
   on the same active-turn corpus.
4. Expand fixture corpus with cheap spectated active turns that include settlement opportunities,
   late-game near-full boards, and first-turn Spirit choices.
5. Final active-game QA once spectator parsing/UI and benchmarked parameters are stable.
