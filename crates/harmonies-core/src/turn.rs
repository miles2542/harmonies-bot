use std::{
    cmp::Reverse,
    collections::{hash_map::DefaultHasher, HashSet},
    hash::{Hash, Hasher},
};

use serde::{Deserialize, Serialize};

use crate::{
    cards::CardCatalog,
    model::{ActiveCard, Color, Coord, PlayerState},
    moves::{apply_settlement, legal_settlements},
    rules::place_token,
    scoring::score_player,
    BoardSide,
};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum TurnStep {
    PlaceToken {
        token: Color,
        coord: Coord,
    },
    DraftCard {
        card_id: u32,
        type_arg: u8,
    },
    ChooseSpirit {
        card_id: u32,
        type_arg: u8,
    },
    SettleCard {
        card_id: u32,
        type_arg: u8,
        coord: Coord,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TurnSequence {
    pub steps: Vec<TurnStep>,
    pub player: PlayerState,
}

#[derive(Clone)]
struct TurnState {
    player: PlayerState,
    remaining_tokens: Vec<Color>,
    draft_done: bool,
    steps: Vec<TurnStep>,
}

pub fn generate_current_turn_sequences(
    player: &PlayerState,
    tokens: &[Color],
    river_cards: &[ActiveCard],
    catalog: &CardCatalog,
    board_side: BoardSide,
    beam_width: usize,
) -> Vec<TurnSequence> {
    let mut initial_tokens = tokens.to_vec();
    initial_tokens.sort_by_key(color_sort_key);
    let mut frontier = vec![TurnState {
        player: player.clone(),
        remaining_tokens: initial_tokens,
        draft_done: false,
        steps: Vec::new(),
    }];
    let mut finals = Vec::new();
    let mut seen = HashSet::new();

    for _ in 0..32 {
        let mut next_frontier = Vec::new();
        for state in frontier {
            if !state.player.spirit_card_choices.is_empty() {
                expand_spirit_choices(&state, &mut next_frontier);
                continue;
            }
            if state.remaining_tokens.is_empty() {
                finals.push(TurnSequence {
                    steps: state.steps.clone(),
                    player: state.player.clone(),
                });
            }
            expand_settlements(&state, catalog, &mut next_frontier);
            expand_drafts(&state, river_cards, &mut next_frontier);
            expand_placements(&state, &mut next_frontier);
        }
        if next_frontier.is_empty() {
            break;
        }
        next_frontier.retain(|state| seen.insert(calculate_state_hash(state)));
        next_frontier.sort_by_cached_key(|state| {
            Reverse(score_player(&state.player, board_side, catalog).total())
        });
        next_frontier.truncate(beam_width);
        frontier = next_frontier;
    }

    dedupe_final_sequences(finals)
}

fn expand_spirit_choices(state: &TurnState, output: &mut Vec<TurnState>) {
    for choice in &state.player.spirit_card_choices {
        let mut next = state.clone();
        let chosen = choice.clone();
        next.player.spirit_card_choices.clear();
        next.player.active_cards.push(chosen.clone());
        next.steps.push(TurnStep::ChooseSpirit {
            card_id: chosen.card_id,
            type_arg: chosen.type_arg,
        });
        output.push(next);
    }
}

fn expand_settlements(state: &TurnState, catalog: &CardCatalog, output: &mut Vec<TurnState>) {
    for settlement in legal_settlements(&state.player, catalog) {
        let mut next = state.clone();
        if !apply_settlement(&mut next.player, &settlement) {
            continue;
        }
        next.steps.push(TurnStep::SettleCard {
            card_id: settlement.card_id,
            type_arg: settlement.type_arg,
            coord: settlement.cube_coord,
        });
        output.push(next);
    }
}

fn expand_drafts(state: &TurnState, river_cards: &[ActiveCard], output: &mut Vec<TurnState>) {
    if state.draft_done || state.player.active_cards.len() >= 4 {
        return;
    }
    for card in river_cards {
        let mut next = state.clone();
        next.draft_done = true;
        next.player.active_cards.push(card.clone());
        next.steps.push(TurnStep::DraftCard {
            card_id: card.card_id,
            type_arg: card.type_arg,
        });
        output.push(next);
    }
}

fn expand_placements(state: &TurnState, output: &mut Vec<TurnState>) {
    let mut used_tokens = HashSet::new();
    for (token_index, token) in state.remaining_tokens.iter().copied().enumerate() {
        if !used_tokens.insert(token) {
            continue;
        }
        for cell_index in 0..state.player.cells.len() {
            let mut next = state.clone();
            let was_empty = next.player.cells[cell_index].stack.is_empty();
            if place_token(&mut next.player.cells[cell_index], token).is_err() {
                continue;
            }
            if was_empty {
                next.player.empty_hexes = next.player.empty_hexes.saturating_sub(1);
            }
            let coord = next.player.cells[cell_index].coord;
            next.remaining_tokens.remove(token_index);
            next.steps.push(TurnStep::PlaceToken { token, coord });
            output.push(next);
        }
    }
}

fn dedupe_final_sequences(sequences: Vec<TurnSequence>) -> Vec<TurnSequence> {
    let mut seen = HashSet::new();
    let mut unique = Vec::new();
    for sequence in sequences {
        if seen.insert(format!("{:?}", sequence.steps)) {
            unique.push(sequence);
        }
    }
    unique
}

fn calculate_state_hash(state: &TurnState) -> u64 {
    let mut hasher = DefaultHasher::new();
    state.player.cells.hash(&mut hasher);
    state.player.active_cards.hash(&mut hasher);
    state.remaining_tokens.hash(&mut hasher);
    state.draft_done.hash(&mut hasher);
    hasher.finish()
}

fn color_sort_key(color: &Color) -> u8 {
    match color {
        Color::Water => 1,
        Color::Mountain => 2,
        Color::Trunk => 3,
        Color::Foliage => 4,
        Color::Field => 5,
        Color::Building => 6,
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        cards::{CardDefinition, CardPatternStep},
        model::{BoardSide, Cell, Stack},
    };

    use super::*;

    fn cell(col: i8, row: i8, tokens: Vec<Color>) -> Cell {
        let mut stack = Stack::default();
        for t in tokens {
            stack.push(t);
        }
        Cell {
            coord: Coord { col, row },
            stack,
            locked_by_cube: false,
        }
    }

    #[test]
    fn can_draft_place_then_settle_same_turn() {
        let mut catalog = CardCatalog::default();
        catalog.cards.insert(
            7,
            CardDefinition {
                type_arg: 7,
                point_locations: vec![4],
                pattern: vec![
                    CardPatternStep {
                        colors: vec![5],
                        position: 0,
                        allow_cube: false,
                    },
                    CardPatternStep {
                        colors: vec![1],
                        position: 3,
                        allow_cube: true,
                    },
                ],
                is_spirit: false,
                spirit_scoring_logic: None,
            },
        );
        let player = PlayerState {
            player_id: "p1".into(),
            cells: vec![cell(0, 0, vec![Color::Field]), cell(1, 0, Vec::new())],
            active_cards: Vec::new(),
            spirit_card_choices: Vec::new(),
            completed_cards: Vec::new(),
            empty_hexes: 1,
        };
        let river = vec![ActiveCard {
            card_id: 10,
            type_arg: 7,
            remaining_cubes: 1,
            is_spirit: false,
        }];
        let turns = generate_current_turn_sequences(
            &player,
            &[Color::Water],
            &river,
            &catalog,
            BoardSide::SideA,
            32,
        );
        assert!(turns.iter().any(|turn| {
            turn.steps
                .iter()
                .any(|step| matches!(step, TurnStep::DraftCard { .. }))
                && turn
                    .steps
                    .iter()
                    .any(|step| matches!(step, TurnStep::SettleCard { .. }))
        }));
    }

    #[test]
    fn spirit_choice_happens_before_normal_turn_actions() {
        let player = PlayerState {
            player_id: "p1".into(),
            cells: vec![cell(0, 0, Vec::new())],
            active_cards: Vec::new(),
            spirit_card_choices: vec![
                ActiveCard {
                    card_id: 19,
                    type_arg: 38,
                    remaining_cubes: 1,
                    is_spirit: true,
                },
                ActiveCard {
                    card_id: 24,
                    type_arg: 41,
                    remaining_cubes: 1,
                    is_spirit: true,
                },
            ],
            completed_cards: Vec::new(),
            empty_hexes: 1,
        };
        let turns = generate_current_turn_sequences(
            &player,
            &[Color::Water],
            &[],
            &CardCatalog::default(),
            BoardSide::SideA,
            32,
        );
        assert!(turns.iter().any(|turn| {
            matches!(turn.steps.first(), Some(TurnStep::ChooseSpirit { .. }))
                && turn
                    .steps
                    .iter()
                    .any(|step| matches!(step, TurnStep::PlaceToken { .. }))
        }));
    }

    #[test]
    fn full_hand_cannot_settle_undrafted_river_card() {
        let mut catalog = CardCatalog::default();
        catalog.cards.insert(
            7,
            CardDefinition {
                type_arg: 7,
                point_locations: vec![4],
                pattern: vec![CardPatternStep {
                    colors: vec![1],
                    position: 0,
                    allow_cube: true,
                }],
                is_spirit: false,
                spirit_scoring_logic: None,
            },
        );
        let player = PlayerState {
            player_id: "p1".into(),
            cells: vec![cell(0, 0, vec![Color::Water]), cell(1, 0, Vec::new())],
            active_cards: vec![
                ActiveCard {
                    card_id: 1,
                    type_arg: 99,
                    remaining_cubes: 1,
                    is_spirit: false,
                },
                ActiveCard {
                    card_id: 2,
                    type_arg: 98,
                    remaining_cubes: 1,
                    is_spirit: false,
                },
                ActiveCard {
                    card_id: 3,
                    type_arg: 97,
                    remaining_cubes: 1,
                    is_spirit: false,
                },
                ActiveCard {
                    card_id: 4,
                    type_arg: 96,
                    remaining_cubes: 1,
                    is_spirit: false,
                },
            ],
            spirit_card_choices: Vec::new(),
            completed_cards: Vec::new(),
            empty_hexes: 1,
        };
        let river = vec![ActiveCard {
            card_id: 10,
            type_arg: 7,
            remaining_cubes: 1,
            is_spirit: false,
        }];
        let turns = generate_current_turn_sequences(
            &player,
            &[Color::Field],
            &river,
            &catalog,
            BoardSide::SideA,
            64,
        );
        assert!(turns.iter().all(|turn| {
            turn.steps.iter().all(|step| {
                !matches!(
                    step,
                    TurnStep::DraftCard { card_id: 10, .. }
                        | TurnStep::SettleCard { card_id: 10, .. }
                )
            })
        }));
    }

    #[test]
    fn turn_sequence_updates_empty_hexes() {
        let player = PlayerState {
            player_id: "p1".into(),
            cells: vec![cell(0, 0, Vec::new()), cell(1, 0, vec![Color::Trunk])],
            active_cards: Vec::new(),
            spirit_card_choices: Vec::new(),
            completed_cards: Vec::new(),
            empty_hexes: 1,
        };
        let turns = generate_current_turn_sequences(
            &player,
            &[Color::Water],
            &[],
            &CardCatalog::default(),
            BoardSide::SideA,
            32,
        );
        assert_eq!(turns.len(), 1);
        assert_eq!(turns[0].player.empty_hexes, 0);

        let turns_trunk = generate_current_turn_sequences(
            &player,
            &[Color::Trunk],
            &[],
            &CardCatalog::default(),
            BoardSide::SideA,
            32,
        );
        assert_eq!(turns_trunk.len(), 2);
        let turn_empty = turns_trunk
            .iter()
            .find(|t| {
                matches!(
                    t.steps[0],
                    TurnStep::PlaceToken { coord, .. } if coord == Coord { col: 0, row: 0 }
                )
            })
            .unwrap();
        assert_eq!(turn_empty.player.empty_hexes, 0);

        let turn_non_empty = turns_trunk
            .iter()
            .find(|t| {
                matches!(
                    t.steps[0],
                    TurnStep::PlaceToken { coord, .. } if coord == Coord { col: 1, row: 0 }
                )
            })
            .unwrap();
        assert_eq!(turn_non_empty.player.empty_hexes, 1);
    }
}
