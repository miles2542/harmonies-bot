use crate::{
    cards::{CardCatalog, CardDefinition},
    model::{ActiveCard, BoardSide, Cell, Color, Coord, PlayerState, Stack},
};

use super::score_player;

fn cell(col: i8, row: i8, tokens: Vec<Color>) -> Cell {
    Cell {
        coord: Coord { col, row },
        stack: Stack { tokens },
        locked_by_cube: false,
    }
}

fn empty(col: i8, row: i8) -> Cell {
    cell(col, row, Vec::new())
}

fn player(cells: Vec<Cell>) -> PlayerState {
    PlayerState {
        player_id: "p1".into(),
        cells,
        active_cards: Vec::new(),
        completed_cards: Vec::new(),
        empty_hexes: 0,
    }
}

#[test]
fn scores_core_landscapes() {
    let player = player(vec![
        cell(0, 0, vec![Color::Trunk, Color::Trunk, Color::Foliage]),
        cell(1, 0, vec![Color::Mountain]),
        cell(2, 0, vec![Color::Mountain, Color::Mountain]),
        cell(0, 1, vec![Color::Field]),
        cell(1, 1, vec![Color::Field]),
    ]);
    let score = score_player(&player, BoardSide::SideA, &CardCatalog::default());
    assert_eq!(score.trees, 7);
    assert_eq!(score.mountains, 4);
    assert_eq!(score.fields, 5);
}

#[test]
fn isolated_mountain_scores_zero_but_adjacent_group_scores_each_stack() {
    let player = player(vec![
        cell(0, 0, vec![Color::Mountain]),
        cell(4, 0, vec![Color::Mountain]),
        cell(
            5,
            0,
            vec![Color::Mountain, Color::Mountain, Color::Mountain],
        ),
    ]);
    assert_eq!(
        score_player(&player, BoardSide::SideA, &CardCatalog::default()).mountains,
        8
    );
}

#[test]
fn fields_score_each_connected_group_of_two_or_more_once() {
    let player = player(vec![
        cell(0, 0, vec![Color::Field]),
        cell(1, 0, vec![Color::Field]),
        cell(3, 0, vec![Color::Field]),
        cell(4, 0, vec![Color::Field]),
        cell(6, 0, vec![Color::Field]),
    ]);
    assert_eq!(
        score_player(&player, BoardSide::SideA, &CardCatalog::default()).fields,
        10
    );
}

#[test]
fn fields_use_bga_column_offset_adjacency() {
    let player = player(vec![
        cell(3, 2, vec![Color::Field]),
        cell(4, 3, vec![Color::Field]),
    ]);
    assert_eq!(
        score_player(&player, BoardSide::SideA, &CardCatalog::default()).fields,
        5
    );
}

#[test]
fn building_needs_three_distinct_adjacent_top_colors() {
    let player = player(vec![
        cell(0, 0, vec![Color::Mountain, Color::Building]),
        cell(-1, 0, vec![Color::Water]),
        cell(1, 0, vec![Color::Field]),
        cell(0, -1, vec![Color::Trunk, Color::Foliage]),
        cell(2, 0, vec![Color::Mountain, Color::Building]),
        cell(3, 0, vec![Color::Field]),
        cell(2, -1, vec![Color::Field]),
    ]);
    assert_eq!(
        score_player(&player, BoardSide::SideA, &CardCatalog::default()).buildings,
        5
    );
}

#[test]
fn single_red_token_is_not_a_scoring_building() {
    let player = player(vec![
        cell(0, 0, vec![Color::Building]),
        cell(1, 0, vec![Color::Field]),
        cell(1, -1, vec![Color::Mountain]),
        cell(0, -1, vec![Color::Trunk, Color::Foliage]),
        cell(3, 0, vec![Color::Building, Color::Building]),
        cell(4, 0, vec![Color::Field]),
        cell(4, 1, vec![Color::Mountain]),
        cell(3, -1, vec![Color::Trunk, Color::Foliage]),
    ]);
    assert_eq!(
        score_player(&player, BoardSide::SideA, &CardCatalog::default()).buildings,
        5
    );
}

#[test]
fn side_a_river_uses_longest_path_not_all_branching_water() {
    let player = player(vec![
        cell(0, 0, vec![Color::Water]),
        cell(1, 0, vec![Color::Water]),
        cell(2, 0, vec![Color::Water]),
        cell(-1, -1, vec![Color::Water]),
        cell(-1, -2, vec![Color::Water]),
        cell(-1, 1, vec![Color::Water]),
        cell(-1, 2, vec![Color::Water]),
    ]);
    assert_eq!(
        score_player(&player, BoardSide::SideA, &CardCatalog::default()).water,
        11
    );
}

#[test]
fn side_a_river_uses_bga_column_offset_adjacency() {
    let player = player(vec![
        cell(1, 0, vec![Color::Water]),
        cell(0, 1, vec![Color::Water]),
        cell(0, 2, vec![Color::Water]),
        cell(0, 3, vec![Color::Water]),
        cell(1, 3, vec![Color::Water]),
        cell(3, 3, vec![Color::Water]),
        cell(2, 4, vec![Color::Water]),
    ]);
    assert_eq!(
        score_player(&player, BoardSide::SideA, &CardCatalog::default()).water,
        19
    );
}

#[test]
fn side_b_water_splits_non_water_islands() {
    let player = player(vec![
        cell(0, 0, vec![Color::Field]),
        cell(1, 0, vec![Color::Water]),
        cell(2, 0, vec![Color::Field]),
        empty(0, 1),
        cell(1, 1, vec![Color::Water]),
        empty(2, 1),
    ]);
    assert_eq!(
        score_player(&player, BoardSide::SideB, &CardCatalog::default()).water,
        10
    );
}

#[test]
fn completed_spirit_scores() {
    let mut catalog = CardCatalog::default();
    catalog.cards.insert(
        42,
        CardDefinition {
            type_arg: 42,
            point_locations: vec![0],
            pattern: Vec::new(),
            is_spirit: true,
            spirit_scoring_logic: None,
        },
    );
    let mut player = player(vec![
        cell(0, 0, vec![Color::Water]),
        cell(1, 0, vec![Color::Water]),
    ]);
    player.completed_cards.push(ActiveCard {
        card_id: 1,
        type_arg: 42,
        remaining_cubes: 0,
        is_spirit: true,
    });
    assert_eq!(score_player(&player, BoardSide::SideA, &catalog).spirits, 4);
}

#[test]
fn spirit_36_scores_short_trees_at_three_and_tall_trees_at_one() {
    let mut catalog = CardCatalog::default();
    catalog.cards.insert(
        36,
        CardDefinition {
            type_arg: 36,
            point_locations: vec![0],
            pattern: Vec::new(),
            is_spirit: true,
            spirit_scoring_logic: None,
        },
    );
    let mut player = player(vec![
        cell(0, 0, vec![Color::Foliage]),
        cell(1, 0, vec![Color::Foliage]),
        cell(2, 0, vec![Color::Trunk, Color::Foliage]),
        cell(3, 0, vec![Color::Trunk, Color::Foliage]),
        cell(4, 0, vec![Color::Trunk, Color::Foliage]),
        cell(0, 1, vec![Color::Trunk, Color::Trunk, Color::Foliage]),
    ]);
    player.completed_cards.push(ActiveCard {
        card_id: 1,
        type_arg: 36,
        remaining_cubes: 0,
        is_spirit: true,
    });
    assert_eq!(
        score_player(&player, BoardSide::SideA, &catalog).spirits,
        16
    );
}
