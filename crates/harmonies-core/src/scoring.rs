use std::collections::{HashMap, HashSet, VecDeque};

use serde::{Deserialize, Serialize};

use crate::{
    cards::{card_score, CardCatalog},
    geometry::{connected_components, neighbors},
    model::{BoardSide, Cell, Color, Coord, PlayerState},
};

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScoreBreakdown {
    pub trees: i32,
    pub mountains: i32,
    pub fields: i32,
    pub buildings: i32,
    pub water: i32,
    pub animals: i32,
    pub spirits: i32,
}

impl ScoreBreakdown {
    pub fn total(&self) -> i32 {
        self.trees
            + self.mountains
            + self.fields
            + self.buildings
            + self.water
            + self.animals
            + self.spirits
    }
}

pub fn score_player(
    player: &PlayerState,
    board_side: BoardSide,
    catalog: &CardCatalog,
) -> ScoreBreakdown {
    let cells_by_coord: HashMap<Coord, &Cell> =
        player.cells.iter().map(|cell| (cell.coord, cell)).collect();
    let animals = player
        .active_cards
        .iter()
        .chain(&player.completed_cards)
        .filter_map(|card| {
            catalog
                .get(card.type_arg)
                .map(|def| (def, card.remaining_cubes))
        })
        .filter(|(def, _)| !def.is_spirit)
        .map(|(def, remaining)| card_score(def, remaining))
        .sum();

    let spirits = score_spirits(player, catalog);

    ScoreBreakdown {
        trees: score_trees(player),
        mountains: score_mountains(&cells_by_coord),
        fields: score_fields(player),
        buildings: score_buildings(&cells_by_coord),
        water: score_water(player, board_side),
        animals,
        spirits,
    }
}

fn score_trees(player: &PlayerState) -> i32 {
    player
        .cells
        .iter()
        .map(|cell| match cell.stack.tokens.as_slice() {
            [Color::Foliage] => 1,
            [Color::Trunk, Color::Foliage] => 3,
            [Color::Trunk, Color::Trunk, Color::Foliage] => 7,
            _ => 0,
        })
        .sum()
}

fn score_mountains(cells: &HashMap<Coord, &Cell>) -> i32 {
    cells
        .values()
        .filter(|cell| is_mountain_stack(cell))
        .filter(|cell| {
            neighbors(cell.coord).iter().any(|coord| {
                cells
                    .get(coord)
                    .map(|next| is_mountain_stack(next))
                    .unwrap_or(false)
            })
        })
        .map(|cell| match cell.stack.height() {
            1 => 1,
            2 => 3,
            3 => 7,
            _ => 0,
        })
        .sum()
}

fn score_fields(player: &PlayerState) -> i32 {
    let fields: HashSet<Coord> = player
        .cells
        .iter()
        .filter(|cell| cell.stack.top() == Some(Color::Field))
        .map(|cell| cell.coord)
        .collect();
    connected_components(&fields)
        .into_iter()
        .filter(|group| group.len() >= 2)
        .count() as i32
        * 5
}

fn score_buildings(cells: &HashMap<Coord, &Cell>) -> i32 {
    cells
        .values()
        .filter(|cell| is_building_stack(cell))
        .filter(|cell| {
            let adjacent_colors: HashSet<Color> = neighbors(cell.coord)
                .iter()
                .filter_map(|coord| cells.get(coord).and_then(|next| next.stack.top()))
                .collect();
            adjacent_colors.len() >= 3
        })
        .count() as i32
        * 5
}

fn score_water(player: &PlayerState, board_side: BoardSide) -> i32 {
    let water: HashSet<Coord> = player
        .cells
        .iter()
        .filter(|cell| cell.stack.top() == Some(Color::Water))
        .map(|cell| cell.coord)
        .collect();
    match board_side {
        BoardSide::SideA => river_score(longest_shortest_water_path(&water)),
        BoardSide::SideB => {
            let land: HashSet<Coord> = player
                .cells
                .iter()
                .filter(|cell| cell.stack.top() != Some(Color::Water))
                .map(|cell| cell.coord)
                .collect();
            connected_components(&land).len() as i32 * 5
        }
    }
}

fn score_spirits(player: &PlayerState, catalog: &CardCatalog) -> i32 {
    player
        .active_cards
        .iter()
        .chain(&player.completed_cards)
        .filter_map(|card| {
            catalog
                .get(card.type_arg)
                .map(|def| (def, card.remaining_cubes))
        })
        .filter(|(def, remaining)| def.is_spirit && *remaining == 0)
        .map(|(def, _)| score_spirit_logic(player, def.type_arg))
        .sum()
}

fn score_spirit_logic(player: &PlayerState, type_arg: u8) -> i32 {
    match type_arg {
        33 => score_color_groups(player, Color::Field, |size| if size >= 3 { 10 } else { 2 }),
        34 => score_color_groups(player, Color::Field, |_| 5),
        35 => count_tree_heights(player, &[2, 3]) * 4,
        36 => count_tree_heights(player, &[1, 2]) * 3 + count_tree_heights(player, &[3]),
        37 => score_color_groups(player, Color::Building, |_| 4),
        38 => score_color_groups(
            player,
            Color::Building,
            |size| if size >= 2 { 6 } else { 0 },
        ),
        39 => count_mountain_heights(player, &[2, 3]) * 4,
        40 => count_mountain_heights(player, &[1, 2]) * 3 + count_mountain_heights(player, &[3]),
        41 => score_color_groups(player, Color::Water, |size| if size >= 2 { 7 } else { 0 }),
        42 => {
            player
                .cells
                .iter()
                .filter(|cell| cell.stack.top() == Some(Color::Water))
                .count() as i32
                * 2
        }
        _ => 0,
    }
}

fn is_mountain_stack(cell: &Cell) -> bool {
    !cell.stack.is_empty()
        && cell
            .stack
            .tokens
            .iter()
            .all(|token| *token == Color::Mountain)
}

fn is_building_stack(cell: &Cell) -> bool {
    matches!(
        cell.stack.tokens.as_slice(),
        [Color::Building, Color::Building]
            | [Color::Trunk, Color::Building]
            | [Color::Mountain, Color::Building]
    )
}

fn longest_shortest_water_path(water: &HashSet<Coord>) -> usize {
    water
        .iter()
        .map(|start| farthest_shortest_path(*start, water))
        .max()
        .unwrap_or(0)
}

fn farthest_shortest_path(start: Coord, water: &HashSet<Coord>) -> usize {
    let mut queue = VecDeque::from([(start, 1usize)]);
    let mut seen = HashSet::from([start]);
    let mut farthest = 1;
    while let Some((coord, distance)) = queue.pop_front() {
        farthest = farthest.max(distance);
        for next in neighbors(coord) {
            if water.contains(&next) && seen.insert(next) {
                queue.push_back((next, distance + 1));
            }
        }
    }
    farthest
}

fn river_score(length: usize) -> i32 {
    match length {
        0 | 1 => 0,
        2 => 2,
        3 => 5,
        4 => 8,
        5 => 11,
        6 => 15,
        value => 15 + (value as i32 - 6) * 4,
    }
}

fn score_color_groups(player: &PlayerState, color: Color, points: fn(usize) -> i32) -> i32 {
    let coords: HashSet<Coord> = player
        .cells
        .iter()
        .filter(|cell| {
            if color == Color::Building {
                is_building_stack(cell)
            } else {
                cell.stack.top() == Some(color)
            }
        })
        .map(|cell| cell.coord)
        .collect();
    connected_components(&coords)
        .into_iter()
        .map(|group| points(group.len()))
        .sum()
}

fn count_tree_heights(player: &PlayerState, heights: &[usize]) -> i32 {
    player
        .cells
        .iter()
        .filter(|cell| {
            cell.stack.top() == Some(Color::Foliage) && heights.contains(&cell.stack.height())
        })
        .count() as i32
}

fn count_mountain_heights(player: &PlayerState, heights: &[usize]) -> i32 {
    player
        .cells
        .iter()
        .filter(|cell| is_mountain_stack(cell) && heights.contains(&cell.stack.height()))
        .count() as i32
}

#[cfg(test)]
mod tests;
