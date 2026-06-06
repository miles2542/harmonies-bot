use rayon::prelude::*;

use crate::{
    cards::CardCatalog,
    eval::EvalWeights,
    model::{GameSnapshotV1, PlayerState},
    turn::generate_current_turn_sequences,
};

use super::{RootCandidate, SearchProgress};

const OPPONENT_TURN_BEAM_WIDTH: usize = 32;

pub(super) fn apply_opponent_denial(
    roots: &mut [RootCandidate],
    snapshot: &GameSnapshotV1,
    player: &PlayerState,
    catalog: &CardCatalog,
    weights: &EvalWeights,
    progress: &mut SearchProgress,
) {
    let opponents = snapshot
        .players
        .iter()
        .filter(|candidate| candidate.player_id != player.player_id)
        .cloned()
        .collect::<Vec<_>>();
    let t = crate::eval::calculate_phase_index(player, &opponents, &snapshot.bag_counts);

    // 1. Compute opponent opportunity gains O(G_j) for each of the 5 central groups j in {0..4}
    let results: Vec<(i32, usize)> = (0..5)
        .into_par_iter()
        .map(|j| {
            let tokens = match snapshot.central_token_groups.get(j) {
                Some(toks) if toks.len() == 3 => toks,
                _ => return (0, 0),
            };
            let mut nodes = 0;
            let val = opponents
                .iter()
                .filter_map(|opponent| {
                    let opp_opponents = snapshot
                        .players
                        .iter()
                        .filter(|candidate| candidate.player_id != opponent.player_id)
                        .cloned()
                        .collect::<Vec<_>>();
                    let t_base = crate::eval::calculate_phase_index(opponent, &opp_opponents, &snapshot.bag_counts);
                    let baseline = crate::eval::eval_player(opponent, snapshot.board_side, catalog, weights, t_base);
                    let best = generate_current_turn_sequences(
                        opponent,
                        tokens,
                        &snapshot.river_cards,
                        catalog,
                        snapshot.board_side,
                        OPPONENT_TURN_BEAM_WIDTH,
                        weights,
                        t_base,
                    )
                    .into_iter()
                    .map(|turn| {
                        let t_turn = crate::eval::calculate_phase_index(&turn.player, &opp_opponents, &snapshot.bag_counts);
                        crate::eval::eval_player(&turn.player, snapshot.board_side, catalog, weights, t_turn)
                    })
                    .max()?;
                    nodes += 1;
                    Some((best - baseline).max(0))
                })
                .max()
                .unwrap_or(0);
            (val, nodes)
        })
        .collect();

    let o_g: Vec<i32> = results.iter().map(|(val, _)| *val).collect();
    let total_nodes: usize = results.iter().map(|(_, nodes)| *nodes).sum();
    progress.nodes_evaluated += total_nodes;

    // 2. For each group i, calculate true denial value D(i) = M_1 - M_2(i)
    let m1 = o_g.iter().copied().max().unwrap_or(0);
    let mut d = vec![0; 5];
    for i in 0..5 {
        let m2 = o_g
            .iter()
            .enumerate()
            .filter(|&(j, _)| j != i)
            .map(|(_, &val)| val)
            .max()
            .unwrap_or(0);
        d[i] = m1 - m2;
    }

    // 3. Set opponent_denial_estimate
    for root in roots {
        if root.group_index < d.len() {
            root.opponent_denial_estimate = d[root.group_index];
        } else {
            root.opponent_denial_estimate = 0;
        }
        root.utility_estimate =
            weights.utility(root.future_estimate, root.opponent_denial_estimate, t);
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_minimax_difference_denial_math() {
        // We will simulate the O(G_j) values of [10, 20, 5, 8, 12]
        let o_g = vec![10, 20, 5, 8, 12];
        let m1 = o_g.iter().copied().max().unwrap_or(0);
        assert_eq!(m1, 20);

        let mut d = vec![0; 5];
        for i in 0..5 {
            let m2 = o_g
                .iter()
                .enumerate()
                .filter(|&(j, _)| j != i)
                .map(|(_, &val)| val)
                .max()
                .unwrap_or(0);
            d[i] = m1 - m2;
        }

        assert_eq!(d[0], 20 - 20); // 0
        assert_eq!(d[1], 20 - 12); // 8
        assert_eq!(d[2], 20 - 20); // 0
        assert_eq!(d[3], 20 - 20); // 0
        assert_eq!(d[4], 20 - 20); // 0
    }
}
