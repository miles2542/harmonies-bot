use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::{
    geometry::{neighbor, rotate_chain, DIRECTIONS},
    model::{Cell, Color, Coord, Stack},
};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CardPatternStep {
    pub colors: Vec<u8>,
    pub position: usize,
    pub allow_cube: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CardDefinition {
    pub type_arg: u8,
    pub point_locations: Vec<i32>,
    pub pattern: Vec<CardPatternStep>,
    pub is_spirit: bool,
    pub spirit_scoring_logic: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct CardCatalog {
    pub cards: HashMap<u8, CardDefinition>,
}

impl CardCatalog {
    pub fn from_cards_database_json(input: &str) -> Result<Self, serde_json::Error> {
        let keyed: HashMap<String, CardDefinition> = serde_json::from_str(input)?;
        let cards = keyed
            .into_values()
            .map(|card| (card.type_arg, card))
            .collect();
        Ok(Self { cards })
    }

    pub fn get(&self, type_arg: u8) -> Option<&CardDefinition> {
        self.cards.get(&type_arg)
    }
}

pub fn card_score(definition: &CardDefinition, remaining_cubes: u8) -> i32 {
    let total = definition.point_locations.len();
    let settled = total.saturating_sub(remaining_cubes as usize);
    if settled == 0 {
        0
    } else {
        definition.point_locations[settled - 1]
    }
}

pub fn pattern_cells(origin: Coord, pattern: &[CardPatternStep], rotation: usize) -> Vec<Coord> {
    let positions: Vec<usize> = pattern.iter().map(|step| step.position).collect();
    let rotated = rotate_chain(&positions, rotation);
    let mut coords = Vec::with_capacity(pattern.len());
    let mut current = origin;

    for (index, direction) in rotated.into_iter().enumerate() {
        if index == 0 {
            current = origin;
        } else {
            current = neighbor(current, direction);
        }
        coords.push(current);
    }

    coords
}

pub fn stack_matches_colors(stack: &Stack, colors: &[u8]) -> bool {
    if colors == [6, 7] {
        return stack.top() == Some(Color::Building);
    }

    let expected: Option<Vec<Color>> = colors
        .iter()
        .rev()
        .map(|raw| Color::from_bga_type_arg(*raw))
        .collect();

    expected
        .map(|tokens| tokens == stack.tokens)
        .unwrap_or(false)
}

pub fn find_pattern_matches<'a>(
    cells: &'a HashMap<Coord, &'a Cell>,
    definition: &CardDefinition,
) -> Vec<Vec<Coord>> {
    let mut matches = Vec::new();
    for origin in cells.keys().copied() {
        for rotation in 0..DIRECTIONS {
            let coords = pattern_cells(origin, &definition.pattern, rotation);
            let matched = coords.iter().zip(&definition.pattern).all(|(coord, step)| {
                cells
                    .get(coord)
                    .map(|cell| stack_matches_colors(&cell.stack, &step.colors))
                    .unwrap_or(false)
            });
            if matched {
                matches.push(coords);
            }
        }
    }
    matches
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn card_score_uses_highest_revealed_value() {
        let definition = CardDefinition {
            type_arg: 1,
            point_locations: vec![4, 9, 15],
            pattern: Vec::new(),
            is_spirit: false,
            spirit_scoring_logic: None,
        };
        assert_eq!(card_score(&definition, 3), 0);
        assert_eq!(card_score(&definition, 2), 4);
        assert_eq!(card_score(&definition, 1), 9);
        assert_eq!(card_score(&definition, 0), 15);
    }

    #[test]
    fn pattern_stack_colors_are_top_to_bottom() {
        let stack = Stack {
            tokens: vec![Color::Trunk, Color::Trunk, Color::Foliage],
        };
        assert!(stack_matches_colors(&stack, &[4, 3, 3]));
        assert!(!stack_matches_colors(&stack, &[3, 3, 4]));
    }

    #[test]
    fn building_alias_matches_any_building_top() {
        let stack = Stack {
            tokens: vec![Color::Mountain, Color::Building],
        };
        assert!(stack_matches_colors(&stack, &[6, 7]));
    }
}
