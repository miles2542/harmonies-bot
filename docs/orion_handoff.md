# Orion Handoff

## Current Objective

Build and verify a passive Firefox Harmonies advisor for BGA. It reads visible/current game state, computes a recommended turn plan, and overlays visual/text instructions. It must not click, call BGA actions, or automate play.

Primary target remains 2-player Side A with Nature Spirit enabled. Side B and 3-4p are bonus only.

## Current Stable State

- Rust core/service, Firefox extension, snapshot/score QA tools exist.
- DOM central token groups are trusted over `gamedatas.tokensOnCentralBoard`; live QA confirmed `gamedatas` can lag or show empty/previous central groups after refills.
- Extension can analyze spectated games from current active player's perspective using same path as active play.
- Extension panel is manual, scrollable, plan sections are selectable, and visual indicators switch to selected plan.
- Visual overlays are separate elements, not mutations of BGA tokens/cells/cards.
- Latest UI additions:
  - `Best so far` badge for highest utility plan seen so far.
  - Settlement arrows from visible `#card_<typeArg>` to target cell, plus card rings.
- Search budget is about 100s, max future depth currently 4.
- CPU utilization is still low on i5-12600K despite `HARMONIES_SEARCH_THREADS=12`; do not tune until settlement correctness audit is complete.

## Card Cube Settlement Audit Result

Observed in live spectate QA: advisor appeared to produce invalid settlement plans. Example screenshot showed active player Quilman with plan:

- Take group 3: Building, Foliage, Field.
- Place Field at (2,3).
- Place Building at (2,1).
- Then multiple settlements:
  - `Settle card 6 cube at ...` repeated.
  - `Settle card 16 cube ...`.
  - `Draft card 8 ...`.
  - `Settle card 8 ...`.

Visual arrows showed settlements from cards that appeared to be in river/common area/opponent/completed zones rather than only active hand cards.

Code audit found two concrete risk points:

- Overlay was looking up cards by `typeArg` first. BGA DOM uses per-game `cardId` in `#card_<id>`,
  while `typeArg` is the persistent catalog id. This could draw arrows from the wrong visible card even
  if the engine chose a legal card instance.
- Normalizers inferred player aliases from card `location` fields and accepted prefix matches. Bad/stale
  card arrays could make opponent/river/done cards look owned by the player.

Fixes added:

- Card ownership now requires exact `board<playerBgaId>` / `done<playerBgaId>` / `river` locations.
- Player id aliases are inferred from board/cube cell ids plus player order, not card arrays.
- Stale no-cube Spirit offers after the choice window are ignored; selected Spirits in hand still parse
  from `boardAnimalCards`.
- Overlay finds DOM card nodes by per-game `cardId` first, labels arrows by persistent `typeArg`.
- Regression tests cover stale Spirit offers, exact ownership, remaining cube cap, locked cells, and full-hand river-card exclusion.
- `tools.validate_advisor_plan_legality` validates advisor outputs against snapshot active hand/draft/completed/cube state.

Rules/constraints that must be enforced:

- Settlement can only use current player's active cards in the 4 hand slots.
- A card drafted during this same plan can be settled later in the same plan only if the player had an empty hand slot and the plan explicitly drafts that card.
- Cards in the river/common selection area must not be settled unless that plan drafted them first.
- Opponent active cards must never be considered for current player settlements.
- Completed/done cards have no cubes and must never be settlement candidates.
- A card can place at most its remaining cube count. If card started with 3 cubes and only 2 remain, at most 2 settlements for that card are legal even if more matching patterns exist.
- Cells already locked by cubes must not accept another cube.
- Board state/cube locations must be parsed correctly before settlement generation.

## Required Next Action

Live QA this fix before tuning/benchmarking:

1. Reload temporary extension.
2. Start `harmonies-service`.
3. Analyze cheap spectated games with visible settlement opportunities.
4. For every settlement step, verify arrow starts at active player's hand card, not river/opponent/done area.
5. Run `python -m tools.validate_advisor_plan_legality` after adding any new active-turn fixtures.

## How To Choose Plans During QA

At a fixed time cutoff, follow the `Best so far` plan unless deliberately choosing a simpler plan. `utility` is ranking value, not guaranteed immediate score. `Immediate score if followed now` is the definite current-turn resulting score estimate.

## Latest Commits Before This Handoff

- `cdfdeeb` Add spectator analysis and overlay QA fixes
- `ff379e4` Refine advisor plan UI and deeper search
- `5487ea4` Clarify advisor score labels
- `df07d31` Add settlement arrows and best-plan badge
