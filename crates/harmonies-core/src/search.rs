use std::time::{Duration, Instant};

mod deck;
mod denial;
mod refill;

use serde::{Deserialize, Serialize};

use crate::{
    advisor::{turn_step_action, MoveActionV1, MovePlanV1},
    cards::CardCatalog,
    eval::EvalWeights,
    model::{ActiveCard, BagCounts, BoardSide, Color, GameSnapshotV1, PlayerState},
    scoring::{score_player, ScoreBreakdown},
    turn::{generate_current_turn_sequences, TurnSequence},
};

use deck::{initial_unseen_standard_cards, river_after_turn_with_refills};
use denial::apply_opponent_denial;
use refill::candidate_refills;

const ROOT_TURN_BEAM_WIDTH: usize = 512;
const FUTURE_TURN_BEAM_WIDTH: usize = 50;
const FUTURE_BRANCH_WIDTH: usize = 50;
const FUTURE_DEPTH: usize = 3;
const REFILL_SAMPLES: usize = 10;
const CARD_REFILL_SAMPLES: usize = 4;
const HARD_STOP_MARGIN_MS: u64 = 6_000;
const MIN_FUTURE_EXPAND_MS: u64 = 7_000;

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchProgress {
    pub depth_completed: usize,
    pub nodes_evaluated: usize,
    pub stopped_early: bool,
}

#[derive(Clone, Debug)]
pub struct SearchOutcome {
    pub plans: Vec<MovePlanV1>,
    pub progress: SearchProgress,
    pub warnings: Vec<String>,
}

#[derive(Clone)]
struct RootCandidate {
    group_index: usize,
    tokens: Vec<Color>,
    turn: TurnSequence,
    immediate: ScoreBreakdown,
    future_estimate: i32,
    utility_estimate: i32,
    opponent_denial_estimate: i32,
}

#[derive(Clone)]
struct FutureState {
    player: PlayerState,
    central_groups: Vec<Vec<Color>>,
    river_cards: Vec<ActiveCard>,
    unseen_cards: Vec<u8>,
    bag_counts: BagCounts,
    score: i32,
}

pub fn search_current_player_turn_with_progress(
    snapshot: &GameSnapshotV1,
    player: &PlayerState,
    catalog: &CardCatalog,
    weights: &EvalWeights,
    max_results: usize,
    seed: u64,
    time_budget_ms: u64,
    started: Instant,
    mut on_progress: impl FnMut(SearchOutcome),
) -> SearchOutcome {
    let deadline =
        started + Duration::from_millis(time_budget_ms.saturating_sub(HARD_STOP_MARGIN_MS));
    let mut progress = SearchProgress::default();
    let mut warnings = Vec::new();
    if snapshot.bag_counts.is_empty() {
        warnings.push("bag counts unavailable; future refill search disabled".into());
    }

    let mut roots = root_candidates(snapshot, player, catalog, weights, &mut progress);
    if roots.is_empty() {
        return SearchOutcome {
            plans: Vec::new(),
            progress,
            warnings,
        };
    }

    if snapshot.bag_counts.unknown > 0 {
        warnings.push("bag color counts inferred with unknown remainder".into());
    }
    if !snapshot.river_cards.is_empty() {
        warnings.push("future card river refill sampled from unseen standard cards".into());
    }
    on_progress(SearchOutcome {
        plans: sorted_plans(&roots, max_results),
        progress: progress.clone(),
        warnings: warnings.clone(),
    });
    apply_opponent_denial(
        &mut roots,
        snapshot,
        player,
        catalog,
        weights,
        &mut progress,
    );
    on_progress(SearchOutcome {
        plans: sorted_plans(&roots, max_results),
        progress: progress.clone(),
        warnings: warnings.clone(),
    });

    for depth in 1..=FUTURE_DEPTH {
        if Instant::now() >= deadline {
            progress.stopped_early = true;
            break;
        }
        estimate_depth(
            &mut roots,
            snapshot,
            catalog,
            weights,
            seed,
            depth,
            deadline,
            &mut progress,
        );
        if !progress.stopped_early {
            progress.depth_completed = depth;
        }
    }

    SearchOutcome {
        plans: sorted_plans(&roots, max_results),
        progress,
        warnings,
    }
}

fn sorted_plans(roots: &[RootCandidate], max_results: usize) -> Vec<MovePlanV1> {
    let mut roots = roots.to_vec();
    roots.sort_by(|left, right| {
        right
            .utility_estimate
            .cmp(&left.utility_estimate)
            .then_with(|| right.future_estimate.cmp(&left.future_estimate))
    });
    roots
        .into_iter()
        .take(max_results.max(1))
        .map(root_plan)
        .collect()
}

fn root_candidates(
    snapshot: &GameSnapshotV1,
    player: &PlayerState,
    catalog: &CardCatalog,
    weights: &EvalWeights,
    progress: &mut SearchProgress,
) -> Vec<RootCandidate> {
    let mut roots = Vec::new();
    for (group_index, tokens) in snapshot.central_token_groups.iter().enumerate() {
        let Some(best_turn) = generate_current_turn_sequences(
            player,
            tokens,
            &snapshot.river_cards,
            catalog,
            snapshot.board_side,
            ROOT_TURN_BEAM_WIDTH,
        )
        .into_iter()
        .max_by_key(|turn| score_player(&turn.player, snapshot.board_side, catalog).total()) else {
            continue;
        };
        let immediate = score_player(&best_turn.player, snapshot.board_side, catalog);
        let future_estimate = immediate.total();
        progress.nodes_evaluated += 1;
        roots.push(RootCandidate {
            group_index,
            tokens: tokens.clone(),
            turn: best_turn,
            immediate,
            future_estimate,
            utility_estimate: weights.utility(future_estimate, 0),
            opponent_denial_estimate: 0,
        });
    }
    roots
}

fn estimate_depth(
    roots: &mut [RootCandidate],
    snapshot: &GameSnapshotV1,
    catalog: &CardCatalog,
    weights: &EvalWeights,
    seed: u64,
    depth: usize,
    deadline: Instant,
    progress: &mut SearchProgress,
) {
    for (index, root) in roots.iter_mut().enumerate() {
        if Instant::now() >= deadline {
            progress.stopped_early = true;
            return;
        }
        let state = state_after_root(root, snapshot, catalog, seed.wrapping_add(index as u64));
        let future = future_value(
            state,
            snapshot.board_side,
            catalog,
            seed.wrapping_add(index as u64),
            depth.saturating_sub(1),
            deadline,
            progress,
        );
        root.future_estimate = future.max(root.immediate.total());
        root.utility_estimate =
            weights.utility(root.future_estimate, root.opponent_denial_estimate);
    }
}

fn future_value(
    initial: FutureState,
    board_side: BoardSide,
    catalog: &CardCatalog,
    seed: u64,
    depth_remaining: usize,
    deadline: Instant,
    progress: &mut SearchProgress,
) -> i32 {
    let mut frontier = vec![initial];
    let mut best = frontier[0].score;
    for depth in 0..depth_remaining {
        let mut next = Vec::new();
        for state in frontier {
            if should_stop(deadline) {
                progress.stopped_early = true;
                return best;
            }
            expand_future_state(
                state,
                board_side,
                catalog,
                seed.wrapping_add(depth as u64),
                deadline,
                &mut next,
                progress,
            );
        }
        if next.is_empty() {
            break;
        }
        next.sort_by(|left, right| right.score.cmp(&left.score));
        next.truncate(FUTURE_BRANCH_WIDTH);
        best = best.max(next[0].score);
        frontier = next;
    }
    best
}

fn expand_future_state(
    state: FutureState,
    board_side: BoardSide,
    catalog: &CardCatalog,
    seed: u64,
    deadline: Instant,
    output: &mut Vec<FutureState>,
    progress: &mut SearchProgress,
) {
    if should_stop(deadline) {
        progress.stopped_early = true;
        return;
    }
    for (group_index, tokens) in state.central_groups.iter().enumerate() {
        if should_stop(deadline) {
            progress.stopped_early = true;
            return;
        }
        let turns = generate_current_turn_sequences(
            &state.player,
            tokens,
            &state.river_cards,
            catalog,
            board_side,
            FUTURE_TURN_BEAM_WIDTH,
        );
        for turn in turns.into_iter().take(FUTURE_BRANCH_WIDTH) {
            for refill in candidate_refills(&state.bag_counts, REFILL_SAMPLES, seed) {
                let mut central_groups = state.central_groups.clone();
                central_groups[group_index] = refill.clone();
                let mut bag_counts = state.bag_counts.clone();
                refill
                    .iter()
                    .copied()
                    .for_each(|color| bag_counts.saturating_sub_color(color));
                for (river_cards, unseen_cards) in river_after_turn_with_refills(
                    &state.river_cards,
                    &turn,
                    &state.unseen_cards,
                    catalog,
                    seed,
                    CARD_REFILL_SAMPLES,
                ) {
                    let score = score_player(&turn.player, board_side, catalog).total();
                    progress.nodes_evaluated += 1;
                    output.push(FutureState {
                        player: turn.player.clone(),
                        central_groups: central_groups.clone(),
                        river_cards,
                        unseen_cards,
                        bag_counts: bag_counts.clone(),
                        score,
                    });
                }
            }
        }
    }
}

fn should_stop(deadline: Instant) -> bool {
    Instant::now() + Duration::from_millis(MIN_FUTURE_EXPAND_MS) >= deadline
}

fn state_after_root(
    root: &RootCandidate,
    snapshot: &GameSnapshotV1,
    catalog: &CardCatalog,
    seed: u64,
) -> FutureState {
    let mut central_groups = snapshot.central_token_groups.clone();
    let refill = candidate_refills(
        &snapshot.bag_counts,
        REFILL_SAMPLES,
        root.group_index as u64 + 1,
    )
    .into_iter()
    .next()
    .unwrap_or_default();
    central_groups[root.group_index] = refill.clone();
    let mut bag_counts = snapshot.bag_counts.clone();
    refill
        .iter()
        .copied()
        .for_each(|color| bag_counts.saturating_sub_color(color));
    let unseen_cards = initial_unseen_standard_cards(snapshot, catalog);
    let river_branch = river_after_turn_with_refills(
        &snapshot.river_cards,
        &root.turn,
        &unseen_cards,
        catalog,
        seed,
        CARD_REFILL_SAMPLES,
    )
    .into_iter()
    .next()
    .unwrap_or_else(|| (snapshot.river_cards.clone(), unseen_cards));
    FutureState {
        player: root.turn.player.clone(),
        central_groups,
        river_cards: river_branch.0,
        unseen_cards: river_branch.1,
        bag_counts,
        score: root.immediate.total(),
    }
}

fn root_plan(root: RootCandidate) -> MovePlanV1 {
    let mut actions = vec![MoveActionV1::TakeGroup {
        group_index: root.group_index,
        tokens: root.tokens,
    }];
    actions.extend(root.turn.steps.into_iter().map(turn_step_action));
    MovePlanV1 {
        central_group_index: root.group_index,
        ordered_actions: actions,
        score_estimate: root.future_estimate,
        utility_estimate: root.utility_estimate,
        opponent_denial_estimate: root.opponent_denial_estimate,
        score_breakdown: root.immediate,
    }
}
