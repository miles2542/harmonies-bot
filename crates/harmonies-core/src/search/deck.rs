use std::collections::HashSet;

use crate::{
    cards::CardCatalog,
    model::{ActiveCard, GameSnapshotV1},
    turn::{TurnSequence, TurnStep},
};

const RIVER_SIZE: usize = 5;

pub(super) fn initial_unseen_standard_cards(
    snapshot: &GameSnapshotV1,
    catalog: &CardCatalog,
) -> Vec<u8> {
    let visible = visible_normal_type_args(snapshot);
    let mut type_args: Vec<u8> = catalog
        .cards
        .values()
        .filter(|card| !card.is_spirit)
        .map(|card| card.type_arg)
        .filter(|type_arg| !visible.contains(type_arg))
        .collect();
    type_args.sort_unstable();
    type_args
}

pub(super) fn river_after_turn_with_refills(
    river: &[ActiveCard],
    turn: &TurnSequence,
    unseen: &[u8],
    catalog: &CardCatalog,
    seed: u64,
    samples: usize,
) -> Vec<(Vec<ActiveCard>, Vec<u8>)> {
    let remaining_river = remove_drafted_card(river, turn);
    if remaining_river.len() >= RIVER_SIZE || unseen.is_empty() {
        return vec![(remaining_river, unseen.to_vec())];
    }

    let needed = RIVER_SIZE - remaining_river.len();
    let draws = candidate_card_draws(unseen, catalog, needed, seed, samples);
    if draws.is_empty() {
        return vec![(remaining_river, unseen.to_vec())];
    }

    draws
        .into_iter()
        .map(|draw| {
            let mut river = remaining_river.clone();
            let mut next_unseen = unseen.to_vec();
            for type_arg in draw {
                if let Some(index) = next_unseen.iter().position(|value| *value == type_arg) {
                    next_unseen.remove(index);
                }
                river.push(synthetic_card(type_arg, catalog));
            }
            (river, next_unseen)
        })
        .collect()
}

fn visible_normal_type_args(snapshot: &GameSnapshotV1) -> HashSet<u8> {
    snapshot
        .river_cards
        .iter()
        .chain(
            snapshot
                .players
                .iter()
                .flat_map(|player| player.active_cards.iter().chain(&player.completed_cards)),
        )
        .filter(|card| !card.is_spirit && card.type_arg <= 32)
        .map(|card| card.type_arg)
        .collect()
}

fn remove_drafted_card(river: &[ActiveCard], turn: &TurnSequence) -> Vec<ActiveCard> {
    let drafted: HashSet<u32> = turn
        .steps
        .iter()
        .filter_map(|step| match step {
            TurnStep::DraftCard { card_id, .. } => Some(*card_id),
            _ => None,
        })
        .collect();
    river
        .iter()
        .filter(|card| !drafted.contains(&card.card_id))
        .cloned()
        .collect()
}

fn candidate_card_draws(
    unseen: &[u8],
    catalog: &CardCatalog,
    needed: usize,
    seed: u64,
    samples: usize,
) -> Vec<Vec<u8>> {
    let mut ordered = unseen.to_vec();
    ordered.sort_by_key(|type_arg| std::cmp::Reverse(card_max_score(catalog, *type_arg)));
    let mut draws = Vec::new();
    draws.push(ordered.iter().copied().take(needed).collect());

    let mut rng = seed.max(1);
    for _ in 0..samples {
        let mut pool = unseen.to_vec();
        let mut draw = Vec::new();
        for _ in 0..needed {
            if pool.is_empty() {
                break;
            }
            rng = rng.wrapping_mul(6364136223846793005).wrapping_add(1);
            draw.push(pool.remove((rng as usize) % pool.len()));
        }
        if draw.len() == needed {
            draw.sort_unstable();
            draws.push(draw);
        }
    }

    let mut seen = HashSet::new();
    draws.retain(|draw| draw.len() == needed && seen.insert(draw.clone()));
    draws
}

fn card_max_score(catalog: &CardCatalog, type_arg: u8) -> i32 {
    catalog
        .get(type_arg)
        .and_then(|card| card.point_locations.last().copied())
        .unwrap_or(0)
}

fn synthetic_card(type_arg: u8, catalog: &CardCatalog) -> ActiveCard {
    ActiveCard {
        card_id: 10_000 + type_arg as u32,
        type_arg,
        remaining_cubes: catalog
            .get(type_arg)
            .map(|card| card.point_locations.len() as u8)
            .unwrap_or(0),
        is_spirit: false,
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        cards::{CardCatalog, CardDefinition},
        model::{BoardSide, GameSnapshotV1, PlayerState},
        turn::{TurnSequence, TurnStep},
    };

    use super::*;

    #[test]
    fn unseen_standard_cards_excludes_visible_normal_cards() {
        let mut catalog = CardCatalog::default();
        for type_arg in 1..=3 {
            catalog.cards.insert(
                type_arg,
                CardDefinition {
                    type_arg,
                    point_locations: vec![1],
                    pattern: Vec::new(),
                    is_spirit: false,
                    spirit_scoring_logic: None,
                },
            );
        }
        let snapshot = GameSnapshotV1 {
            schema_version: 1,
            perspective_player_id: "p1".into(),
            active_player_id: "p1".into(),
            board_side: BoardSide::SideA,
            players: vec![PlayerState {
                player_id: "p1".into(),
                cells: Vec::new(),
                active_cards: vec![synthetic_card(1, &catalog)],
                completed_cards: Vec::new(),
                empty_hexes: 0,
            }],
            central_token_groups: Vec::new(),
            river_cards: vec![synthetic_card(2, &catalog)],
            bag_counts: Default::default(),
            cards_catalog_version: "test".into(),
        };
        assert_eq!(initial_unseen_standard_cards(&snapshot, &catalog), vec![3]);
    }

    #[test]
    fn drafted_river_card_gets_sampled_replacement() {
        let mut catalog = CardCatalog::default();
        for type_arg in 1..=6 {
            catalog.cards.insert(
                type_arg,
                CardDefinition {
                    type_arg,
                    point_locations: vec![2, 5],
                    pattern: Vec::new(),
                    is_spirit: false,
                    spirit_scoring_logic: None,
                },
            );
        }
        let river = (1..=5)
            .map(|type_arg| synthetic_card(type_arg, &catalog))
            .collect::<Vec<_>>();
        let turn = TurnSequence {
            steps: vec![TurnStep::DraftCard {
                card_id: 10_001,
                type_arg: 1,
            }],
            player: PlayerState {
                player_id: "p1".into(),
                cells: Vec::new(),
                active_cards: Vec::new(),
                completed_cards: Vec::new(),
                empty_hexes: 0,
            },
        };
        let branches = river_after_turn_with_refills(&river, &turn, &[6], &catalog, 1, 1);
        assert_eq!(branches.len(), 1);
        assert_eq!(branches[0].0.len(), 5);
        assert!(branches[0].0.iter().any(|card| card.type_arg == 6));
        assert_eq!(branches[0].0.last().unwrap().remaining_cubes, 2);
        assert!(branches[0].1.is_empty());
    }
}
