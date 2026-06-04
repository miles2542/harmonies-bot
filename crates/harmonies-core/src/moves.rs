use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::{
    cards::{find_pattern_matches, CardCatalog},
    model::{ActiveCard, Cell, Color, Coord, PlayerState},
    rules::place_token,
};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlacementStep {
    pub token: Color,
    pub coord: Coord,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlacementSequence {
    pub steps: Vec<PlacementStep>,
    pub player: PlayerState,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SettlementMove {
    pub card_id: u32,
    pub type_arg: u8,
    pub is_spirit: bool,
    pub cube_coord: Coord,
    pub pattern_coords: Vec<Coord>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SettlementSequence {
    pub settlements: Vec<SettlementMove>,
    pub player: PlayerState,
}

pub fn generate_placement_sequences(
    player: &PlayerState,
    tokens: &[Color],
) -> Vec<PlacementSequence> {
    let mut results = Vec::new();
    let mut remaining = tokens.to_vec();
    remaining.sort_by_key(color_sort_key);
    let mut steps = Vec::new();
    generate_placement_sequences_inner(player.clone(), &remaining, &mut steps, &mut results);
    dedupe_sequences(results)
}

pub fn legal_settlements(player: &PlayerState, catalog: &CardCatalog) -> Vec<SettlementMove> {
    let cells_by_coord: HashMap<Coord, &Cell> =
        player.cells.iter().map(|cell| (cell.coord, cell)).collect();
    let mut settlements = Vec::new();

    for card in player
        .active_cards
        .iter()
        .filter(|card| card.remaining_cubes > 0)
    {
        let Some(definition) = catalog.get(card.type_arg) else {
            continue;
        };
        for coords in find_pattern_matches(&cells_by_coord, definition) {
            let Some(cube_coord) = definition
                .pattern
                .iter()
                .zip(&coords)
                .find(|(step, _)| step.allow_cube)
                .map(|(_, coord)| *coord)
            else {
                continue;
            };
            let Some(cube_cell) = cells_by_coord.get(&cube_coord) else {
                continue;
            };
            if cube_cell.locked_by_cube {
                continue;
            }
            settlements.push(SettlementMove {
                card_id: card.card_id,
                type_arg: card.type_arg,
                is_spirit: card.is_spirit,
                cube_coord,
                pattern_coords: coords,
            });
        }
    }

    settlements
}

pub fn apply_settlement(player: &mut PlayerState, settlement: &SettlementMove) -> bool {
    let Some(card_index) = player
        .active_cards
        .iter()
        .position(|card| card.card_id == settlement.card_id && card.remaining_cubes > 0)
    else {
        return false;
    };
    let Some(cell) = player
        .cells
        .iter_mut()
        .find(|cell| cell.coord == settlement.cube_coord && !cell.locked_by_cube)
    else {
        return false;
    };

    cell.locked_by_cube = true;
    player.active_cards[card_index].remaining_cubes -= 1;
    if player.active_cards[card_index].remaining_cubes == 0 {
        let completed = player.active_cards.remove(card_index);
        player.completed_cards.push(completed);
    }
    true
}

pub fn apply_all_forced_settlements(
    player: &mut PlayerState,
    catalog: &CardCatalog,
) -> Vec<SettlementMove> {
    let mut applied = Vec::new();
    loop {
        let Some(next) = legal_settlements(player, catalog).into_iter().next() else {
            break;
        };
        if !apply_settlement(player, &next) {
            break;
        }
        applied.push(next);
    }
    applied
}

pub fn generate_settlement_sequences(
    player: &PlayerState,
    catalog: &CardCatalog,
) -> Vec<SettlementSequence> {
    let mut results = vec![SettlementSequence {
        settlements: Vec::new(),
        player: player.clone(),
    }];
    let mut path = Vec::new();
    generate_settlement_sequences_inner(player.clone(), catalog, &mut path, &mut results);
    dedupe_settlement_sequences(results)
}

fn generate_settlement_sequences_inner(
    player: PlayerState,
    catalog: &CardCatalog,
    path: &mut Vec<SettlementMove>,
    results: &mut Vec<SettlementSequence>,
) {
    for settlement in legal_settlements(&player, catalog) {
        let mut next_player = player.clone();
        if !apply_settlement(&mut next_player, &settlement) {
            continue;
        }
        path.push(settlement);
        results.push(SettlementSequence {
            settlements: path.clone(),
            player: next_player.clone(),
        });
        generate_settlement_sequences_inner(next_player, catalog, path, results);
        path.pop();
    }
}

fn generate_placement_sequences_inner(
    player: PlayerState,
    remaining: &[Color],
    steps: &mut Vec<PlacementStep>,
    results: &mut Vec<PlacementSequence>,
) {
    if remaining.is_empty() {
        results.push(PlacementSequence {
            steps: steps.clone(),
            player,
        });
        return;
    }

    let mut used_colors = HashSet::new();
    for (token_index, token) in remaining.iter().copied().enumerate() {
        if !used_colors.insert(token) {
            continue;
        }

        for cell_index in 0..player.cells.len() {
            let mut next_player = player.clone();
            if place_token(&mut next_player.cells[cell_index], token).is_err() {
                continue;
            }
            steps.push(PlacementStep {
                token,
                coord: next_player.cells[cell_index].coord,
            });
            let mut next_remaining = remaining.to_vec();
            next_remaining.remove(token_index);
            generate_placement_sequences_inner(next_player, &next_remaining, steps, results);
            steps.pop();
        }
    }
}

fn dedupe_sequences(sequences: Vec<PlacementSequence>) -> Vec<PlacementSequence> {
    let mut seen = HashSet::new();
    let mut unique = Vec::new();
    for sequence in sequences {
        let key: Vec<(Color, Coord)> = sequence
            .steps
            .iter()
            .map(|step| (step.token, step.coord))
            .collect();
        if seen.insert(key) {
            unique.push(sequence);
        }
    }
    unique
}

fn dedupe_settlement_sequences(sequences: Vec<SettlementSequence>) -> Vec<SettlementSequence> {
    let mut seen = HashSet::new();
    let mut unique = Vec::new();
    for sequence in sequences {
        let key: Vec<(u32, Coord)> = sequence
            .settlements
            .iter()
            .map(|settlement| (settlement.card_id, settlement.cube_coord))
            .collect();
        if seen.insert(key) {
            unique.push(sequence);
        }
    }
    unique
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

#[allow(dead_code)]
fn _assert_active_card_send_sync(_: &ActiveCard) {}

#[cfg(test)]
mod tests {
    use crate::{
        cards::{CardDefinition, CardPatternStep},
        model::Stack,
    };

    use super::*;

    fn cell(col: i8, row: i8, tokens: Vec<Color>, locked: bool) -> Cell {
        Cell {
            coord: Coord { col, row },
            stack: Stack { tokens },
            locked_by_cube: locked,
        }
    }

    fn player(cells: Vec<Cell>) -> PlayerState {
        PlayerState {
            player_id: "p1".into(),
            cells,
            active_cards: Vec::new(),
            spirit_card_choices: Vec::new(),
            completed_cards: Vec::new(),
            empty_hexes: 0,
        }
    }

    #[test]
    fn placement_sequences_respect_stack_rules_and_locks() {
        let player = player(vec![
            cell(0, 0, Vec::new(), false),
            cell(1, 0, vec![Color::Field], false),
            cell(2, 0, Vec::new(), true),
        ]);
        let sequences = generate_placement_sequences(&player, &[Color::Water]);
        assert_eq!(sequences.len(), 1);
        assert_eq!(sequences[0].steps[0].coord, Coord { col: 0, row: 0 });
    }

    #[test]
    fn settlement_detects_unlocked_allow_cube_cell() {
        let mut catalog = CardCatalog::default();
        catalog.cards.insert(
            1,
            CardDefinition {
                type_arg: 1,
                point_locations: vec![5],
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
        let mut player = player(vec![
            cell(0, 0, vec![Color::Field], false),
            cell(1, 0, vec![Color::Water], false),
        ]);
        player.active_cards.push(ActiveCard {
            card_id: 10,
            type_arg: 1,
            remaining_cubes: 1,
            is_spirit: false,
        });

        let settlements = legal_settlements(&player, &catalog);
        assert_eq!(settlements.len(), 1);
        assert_eq!(settlements[0].cube_coord, Coord { col: 1, row: 0 });
    }

    #[test]
    fn apply_settlement_locks_cell_and_completes_card() {
        let mut player = player(vec![cell(1, 0, vec![Color::Water], false)]);
        player.active_cards.push(ActiveCard {
            card_id: 10,
            type_arg: 1,
            remaining_cubes: 1,
            is_spirit: false,
        });
        let settlement = SettlementMove {
            card_id: 10,
            type_arg: 1,
            is_spirit: false,
            cube_coord: Coord { col: 1, row: 0 },
            pattern_coords: vec![Coord { col: 1, row: 0 }],
        };
        assert!(apply_settlement(&mut player, &settlement));
        assert!(player.cells[0].locked_by_cube);
        assert!(player.active_cards.is_empty());
        assert_eq!(player.completed_cards.len(), 1);
    }
}
