use crate::{
    cards::CardCatalog,
    eval::EvalWeights,
    model::{Color, GameSnapshotV1, PlayerState},
    scoring::score_player,
    turn::generate_current_turn_sequences,
};

use super::{RootCandidate, SearchProgress};

const OPPONENT_TURN_BEAM_WIDTH: usize = 32;

pub(super) fn opponent_denial_estimate(
    snapshot: &GameSnapshotV1,
    player: &PlayerState,
    tokens: &[Color],
    catalog: &CardCatalog,
    progress: &mut SearchProgress,
) -> i32 {
    snapshot
        .players
        .iter()
        .filter(|candidate| candidate.player_id != player.player_id)
        .filter_map(|opponent| {
            let baseline = score_player(opponent, snapshot.board_side, catalog).total();
            let best = generate_current_turn_sequences(
                opponent,
                tokens,
                &snapshot.river_cards,
                catalog,
                snapshot.board_side,
                OPPONENT_TURN_BEAM_WIDTH,
            )
            .into_iter()
            .map(|turn| score_player(&turn.player, snapshot.board_side, catalog).total())
            .max()?;
            progress.nodes_evaluated += 1;
            Some((best - baseline).max(0))
        })
        .max()
        .unwrap_or(0)
}

pub(super) fn apply_opponent_denial(
    roots: &mut [RootCandidate],
    snapshot: &GameSnapshotV1,
    player: &PlayerState,
    catalog: &CardCatalog,
    weights: &EvalWeights,
    progress: &mut SearchProgress,
) {
    for root in roots {
        root.opponent_denial_estimate =
            opponent_denial_estimate(snapshot, player, &root.tokens, catalog, progress);
        root.utility_estimate =
            weights.utility(root.future_estimate, root.opponent_denial_estimate);
    }
}
