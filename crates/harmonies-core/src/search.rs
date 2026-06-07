use std::time::{Duration, Instant};

use rayon::prelude::*;

mod cache;
mod deck;
mod denial;
mod refill;
mod settings;
mod types;

use crate::{
    advisor::{turn_step_action, MoveActionV1, MovePlanV1},
    cards::CardCatalog,
    eval::EvalWeights,
    model::{BoardSide, GameSnapshotV1, PlayerState},
    scoring::score_player,
    turn::{generate_current_turn_sequences, TurnSequence, TurnStep},
};

use deck::{initial_unseen_standard_cards, river_after_turn_with_refills};
use denial::apply_opponent_denial;
use refill::candidate_refills;
use settings::SearchSettings;
use types::{FutureState, RootCandidate};
pub use types::{SearchOutcome, SearchProgress};

pub fn search_current_player_turn_with_progress(
    snapshot: &GameSnapshotV1,
    player: &PlayerState,
    catalog: &CardCatalog,
    weights: &EvalWeights,
    max_results: usize,
    seed: u64,
    time_budget_ms: u64,
    started: Instant,
    should_cancel: impl Fn() -> bool + Sync,
    mut on_progress: impl FnMut(SearchOutcome),
) -> SearchOutcome {
    let settings = SearchSettings::from_env();
    let deadline = started
        + Duration::from_millis(time_budget_ms.saturating_sub(settings.hard_stop_margin_ms));
    let mut progress = SearchProgress::default();
    let mut warnings = Vec::new();
    if snapshot.bag_counts.is_empty() {
        warnings.push("bag counts unavailable; future refill search disabled".into());
    }

    let mut roots = root_candidates(snapshot, player, catalog, weights, &settings, &mut progress);
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
    if should_cancel() {
        progress.stopped_early = true;
    } else {
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
    }

    let tt = cache::TranspositionTable::new(settings.transposition_table_size_power_of_two);
    for depth in 1..=settings.future_depth {
        if progress.stopped_early || should_cancel() || Instant::now() >= deadline {
            progress.stopped_early = true;
            break;
        }
        estimate_depth(
            &mut roots,
            snapshot,
            catalog,
            weights,
            &settings,
            seed,
            depth,
            deadline,
            &should_cancel,
            &mut progress,
            &tt,
        );
        if !progress.stopped_early {
            progress.depth_completed = depth;
            on_progress(SearchOutcome {
                plans: sorted_plans(&roots, max_results),
                progress: progress.clone(),
                warnings: warnings.clone(),
            });
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
            .then_with(|| left.group_index.cmp(&right.group_index))
    });
    roots
        .into_iter()
        .take(max_results.max(1))
        .map(root_plan)
        .collect()
}

fn get_opponents(snapshot: &GameSnapshotV1, player_id: &str) -> Vec<PlayerState> {
    snapshot
        .players
        .iter()
        .filter(|p| p.player_id != player_id)
        .cloned()
        .collect()
}

fn root_candidates(
    snapshot: &GameSnapshotV1,
    player: &PlayerState,
    catalog: &CardCatalog,
    weights: &EvalWeights,
    settings: &SearchSettings,
    progress: &mut SearchProgress,
) -> Vec<RootCandidate> {
    let started = Instant::now();
    let opponents = get_opponents(snapshot, &player.player_id);
    let t = crate::eval::calculate_phase_index(player, &opponents, &snapshot.bag_counts);
    let group_results: Vec<_> = snapshot
        .central_token_groups
        .par_iter()
        .enumerate()
        .flat_map_iter(|(group_index, tokens)| {
            if tokens.len() != 3 {
                return Vec::new();
            }
            let turns = generate_current_turn_sequences(
                player,
                tokens,
                &snapshot.river_cards,
                catalog,
                snapshot.board_side,
                settings.root_turn_beam_width,
                weights,
                t,
            );
            let sequences_generated = turns.len();
            let turns =
                turns
                    .into_iter()
                    .fold(Vec::<TurnSequence>::new(), |mut best_by_choice, turn| {
                        upsert_best_turn_by_spirit_choice(
                            &mut best_by_choice,
                            turn,
                            snapshot,
                            &opponents,
                            catalog,
                            weights,
                            t,
                        );
                        best_by_choice
                    });
            turns
                .into_iter()
                .map(|turn| {
                    let immediate = score_player(&turn.player, snapshot.board_side, catalog);
                    let future_estimate = crate::eval::eval_player(&turn.player, snapshot.board_side, catalog, weights, t);
                    RootCandidate {
                        group_index,
                        tokens: tokens.clone(),
                        turn,
                        immediate,
                        future_estimate,
                        utility_estimate: weights.utility(future_estimate, 0, t),
                        opponent_denial_estimate: 0,
                    }
                })
                .map(move |root| (root, sequences_generated))
                .collect::<Vec<_>>()
        })
        .collect();
    let sequences_generated = group_results
        .iter()
        .map(|(_, sequences)| *sequences)
        .sum::<usize>();
    let mut roots: Vec<_> = group_results.into_iter().map(|(root, _)| root).collect();
    roots.sort_by(|left, right| {
        left.group_index
            .cmp(&right.group_index)
            .then_with(|| left.utility_estimate.cmp(&right.utility_estimate))
    });
    progress.root_generation_ms = started.elapsed().as_millis().try_into().unwrap_or(u64::MAX);
    progress.root_sequences_generated = sequences_generated;
    progress.nodes_evaluated += roots.len();
    roots
}

fn upsert_best_turn_by_spirit_choice(
    best_by_choice: &mut Vec<TurnSequence>,
    turn: TurnSequence,
    snapshot: &GameSnapshotV1,
    _opponents: &[PlayerState],
    catalog: &CardCatalog,
    weights: &EvalWeights,
    t: f64,
) {
    let choice = spirit_choice_key(&turn);
    let score = crate::eval::eval_player(&turn.player, snapshot.board_side, catalog, weights, t);
    if let Some(existing) = best_by_choice
        .iter_mut()
        .find(|existing| spirit_choice_key(existing) == choice)
    {
        let existing_score = crate::eval::eval_player(&existing.player, snapshot.board_side, catalog, weights, t);
        if score > existing_score {
            *existing = turn;
        }
    } else {
        best_by_choice.push(turn);
    }
}

fn spirit_choice_key(turn: &TurnSequence) -> Option<(u32, u8)> {
    turn.steps.iter().find_map(|step| match step {
        TurnStep::ChooseSpirit { card_id, type_arg } => Some((*card_id, *type_arg)),
        _ => None,
    })
}

fn estimate_depth(
    roots: &mut [RootCandidate],
    snapshot: &GameSnapshotV1,
    catalog: &CardCatalog,
    weights: &EvalWeights,
    settings: &SearchSettings,
    seed: u64,
    depth: usize,
    deadline: Instant,
    should_cancel: &(impl Fn() -> bool + Sync),
    progress: &mut SearchProgress,
    tt: &cache::TranspositionTable,
) {
    let player = snapshot
        .players
        .iter()
        .find(|p| p.player_id == snapshot.perspective_player_id)
        .expect("perspective player missing");
    let opponents = get_opponents(snapshot, &player.player_id);
    let t = crate::eval::calculate_phase_index(player, &opponents, &snapshot.bag_counts);

    let outcomes: Vec<SearchProgress> = roots
        .par_iter_mut()
        .enumerate()
        .map(|(index, root)| {
            let mut local_progress = SearchProgress::default();
            if should_cancel() || Instant::now() >= deadline {
                local_progress.stopped_early = true;
                return local_progress;
            }
            let state = state_after_root(
                root,
                snapshot,
                catalog,
                weights,
                &opponents,
                settings,
                seed.wrapping_add(index as u64),
            );
            let future = future_value(
                state,
                snapshot.board_side,
                catalog,
                weights,
                &opponents,
                settings,
                seed.wrapping_add(index as u64),
                depth,
                deadline,
                should_cancel,
                &mut local_progress,
                tt,
            );
            root.future_estimate = future.max(crate::eval::eval_player(&root.turn.player, snapshot.board_side, catalog, weights, t));
            root.utility_estimate =
                weights.utility(root.future_estimate, root.opponent_denial_estimate, t);
            local_progress
        })
        .collect();

    for outcome in outcomes {
        progress.nodes_evaluated += outcome.nodes_evaluated;
        progress.stopped_early |= outcome.stopped_early;
    }
}

fn future_value(
    initial: FutureState,
    board_side: BoardSide,
    catalog: &CardCatalog,
    weights: &EvalWeights,
    opponents: &[PlayerState],
    settings: &SearchSettings,
    seed: u64,
    depth_remaining: usize,
    deadline: Instant,
    should_cancel: &(impl Fn() -> bool + Sync),
    progress: &mut SearchProgress,
    tt: &cache::TranspositionTable,
) -> i32 {
    let initial_score = initial.score;
    let initial_hash = cache::hash_future_state(&initial);
    if let Some(cached) = tt.lookup(initial_hash, depth_remaining) {
        return cached;
    }

    let mut frontier = vec![initial];
    let mut best = frontier[0].score;
    for depth in 0..depth_remaining {
        if should_stop(deadline, should_cancel, settings) {
            progress.stopped_early = true;
            return best;
        }
        let expanded: Vec<(Vec<FutureState>, SearchProgress)> = frontier
            .into_par_iter()
            .enumerate()
            .map(|(state_index, state)| {
                let mut output = Vec::new();
                let mut local_progress = SearchProgress::default();
                if should_stop(deadline, should_cancel, settings) {
                    local_progress.stopped_early = true;
                    return (output, local_progress);
                }
                
                let current_depth_remaining = depth_remaining - depth;
                let hash = cache::hash_future_state(&state);
                if let Some(cached_score) = tt.lookup(hash, current_depth_remaining) {
                    let mut cached_state = state.clone();
                    cached_state.score = cached_score;
                    output.push(cached_state);
                    return (output, local_progress);
                }

                let state_seed = seed
                    .wrapping_add(depth as u64)
                    .wrapping_add((state_index as u64).wrapping_mul(1_000_003));
                expand_future_state(
                    state,
                    board_side,
                    catalog,
                    weights,
                    opponents,
                    settings,
                    state_seed,
                    deadline,
                    should_cancel,
                    &mut output,
                    &mut local_progress,
                );
                (output, local_progress)
            })
            .collect();

        let mut next = Vec::new();
        for (mut states, local_progress) in expanded {
            progress.nodes_evaluated += local_progress.nodes_evaluated;
            progress.stopped_early |= local_progress.stopped_early;
            next.append(&mut states);
        }
        if progress.stopped_early {
            return best;
        }
        if next.is_empty() {
            break;
        }
        next.sort_by(|left, right| right.score.cmp(&left.score));
        next.truncate(settings.future_branch_width);
        // Discount the score improvement (gain) over the initial root state to favor immediate scores
        let gain = next[0].score - initial_score;
        let discount = 0.85f64;
        let discounted_score = initial_score + (gain as f64 * discount.powi(depth as i32 + 1)).round() as i32;
        best = best.max(discounted_score);
        frontier = next;
    }
    
    tt.store(initial_hash, depth_remaining, best);
    best
}

fn expand_future_state(
    state: FutureState,
    board_side: BoardSide,
    catalog: &CardCatalog,
    weights: &EvalWeights,
    opponents: &[PlayerState],
    settings: &SearchSettings,
    seed: u64,
    deadline: Instant,
    should_cancel: &(impl Fn() -> bool + Sync),
    output: &mut Vec<FutureState>,
    progress: &mut SearchProgress,
) {
    if should_stop(deadline, should_cancel, settings) {
        progress.stopped_early = true;
        return;
    }
    for (group_index, tokens) in state.central_groups.iter().enumerate() {
        if should_stop(deadline, should_cancel, settings) {
            progress.stopped_early = true;
            return;
        }
        let t_state = crate::eval::calculate_phase_index(&state.player, opponents, &state.bag_counts);
        let turns = generate_current_turn_sequences(
            &state.player,
            tokens,
            &state.river_cards,
            catalog,
            board_side,
            settings.future_turn_beam_width,
            weights,
            t_state,
        );
        for turn in turns.into_iter().take(settings.future_branch_width) {
            for refill in candidate_refills(&state.bag_counts, 1, seed) {
                let mut central_groups = state.central_groups.clone();
                central_groups[group_index] = refill.clone();
                let mut bag_counts = state.bag_counts.clone();
                refill
                    .iter()
                    .copied()
                    .for_each(|color| bag_counts.saturating_sub_color(color));
                let t = crate::eval::calculate_phase_index(&turn.player, opponents, &bag_counts);
                let score = crate::eval::eval_player(&turn.player, board_side, catalog, weights, t);
                for (river_cards, unseen_cards) in river_after_turn_with_refills(
                    &state.river_cards,
                    &turn,
                    &state.unseen_cards,
                    catalog,
                    seed,
                    1,
                ) {
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

fn should_stop(
    deadline: Instant,
    should_cancel: &(impl Fn() -> bool + Sync),
    settings: &SearchSettings,
) -> bool {
    should_cancel()
        || Instant::now() + Duration::from_millis(settings.min_future_expand_ms) >= deadline
}

fn state_after_root(
    root: &RootCandidate,
    snapshot: &GameSnapshotV1,
    catalog: &CardCatalog,
    weights: &EvalWeights,
    opponents: &[PlayerState],
    settings: &SearchSettings,
    seed: u64,
) -> FutureState {
    let mut central_groups = snapshot.central_token_groups.clone();
    let refill = candidate_refills(
        &snapshot.bag_counts,
        settings.refill_samples,
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
        settings.card_refill_samples,
    )
    .into_iter()
    .next()
    .unwrap_or_else(|| (snapshot.river_cards.clone(), unseen_cards));
    let t = crate::eval::calculate_phase_index(&root.turn.player, opponents, &bag_counts);
    let score = crate::eval::eval_player(&root.turn.player, snapshot.board_side, catalog, weights, t);
    FutureState {
        player: root.turn.player.clone(),
        central_groups,
        river_cards: river_branch.0,
        unseen_cards: river_branch.1,
        bag_counts,
        score,
    }
}

fn root_plan(root: RootCandidate) -> MovePlanV1 {
    let take_group = MoveActionV1::TakeGroup {
        group_index: root.group_index,
        tokens: root.tokens,
    };
    let mut actions = Vec::new();
    let mut take_group_inserted = false;
    for step in root.turn.steps {
        if !take_group_inserted && !matches!(step, TurnStep::ChooseSpirit { .. }) {
            actions.push(take_group.clone());
            take_group_inserted = true;
        }
        actions.push(turn_step_action(step));
    }
    if !take_group_inserted {
        actions.push(take_group);
    }
    MovePlanV1 {
        central_group_index: root.group_index,
        ordered_actions: actions,
        score_estimate: root.future_estimate,
        utility_estimate: root.utility_estimate,
        opponent_denial_estimate: root.opponent_denial_estimate,
        score_breakdown: root.immediate,
    }
}
