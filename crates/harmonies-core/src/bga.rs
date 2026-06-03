use std::collections::{HashMap, HashSet};

use serde_json::Value;
use thiserror::Error;

use crate::model::{ActiveCard, BoardSide, Cell, Color, Coord, GameSnapshotV1, PlayerState, Stack};

#[derive(Debug, Error)]
pub enum BgaNormalizeError {
    #[error("gamedatas must be a JSON object")]
    NotObject,
    #[error("players missing or invalid")]
    MissingPlayers,
    #[error("hexes missing or invalid")]
    MissingHexes,
}

pub fn normalize_gamedatas(
    gamedatas: &Value,
    perspective_player_id: Option<&str>,
) -> Result<GameSnapshotV1, BgaNormalizeError> {
    let object = gamedatas.as_object().ok_or(BgaNormalizeError::NotObject)?;
    let players_value = object
        .get("players")
        .and_then(Value::as_object)
        .ok_or(BgaNormalizeError::MissingPlayers)?;
    let hexes = parse_hexes(object.get("hexes")).ok_or(BgaNormalizeError::MissingHexes)?;
    let cubes = collect_cube_locations(object.get("cubesOnAnimalCards"));

    let perspective = perspective_player_id
        .map(str::to_owned)
        .or_else(|| string_field(gamedatas, "player_id"))
        .or_else(|| string_field(gamedatas, "current_player_id"))
        .or_else(|| active_player_id(gamedatas))
        .filter(|id| players_value.contains_key(id))
        .unwrap_or_else(|| players_value.keys().next().cloned().unwrap_or_default());

    let active_player_id = active_player_id(gamedatas).unwrap_or_else(|| perspective.clone());
    let players = players_value
        .iter()
        .map(|(player_id, value)| normalize_player(player_id, value, &hexes, &cubes, gamedatas))
        .collect();

    Ok(GameSnapshotV1 {
        schema_version: 1,
        perspective_player_id: perspective,
        active_player_id,
        board_side: parse_board_side(object.get("boardSide")),
        players,
        central_token_groups: parse_central_groups(object.get("tokensOnCentralBoard")),
        river_cards: parse_cards(object.get("river"), &HashMap::new(), false),
        cards_catalog_version: object
            .get("version")
            .and_then(Value::as_str)
            .unwrap_or("bga")
            .to_owned(),
    })
}

fn normalize_player(
    player_id: &str,
    value: &Value,
    hexes: &[Coord],
    cubes: &HashSet<String>,
    gamedatas: &Value,
) -> PlayerState {
    let card_cube_counts = count_card_cubes(gamedatas.get("cubesOnAnimalCards"));
    let token_stacks = parse_tokens_on_board(value.get("tokensOnBoard"));
    let cells = hexes
        .iter()
        .copied()
        .map(|coord| {
            let key = cell_key(player_id, coord);
            Cell {
                coord,
                stack: Stack {
                    tokens: token_stacks.get(&key).cloned().unwrap_or_default(),
                },
                locked_by_cube: cubes.contains(&key),
            }
        })
        .collect();

    let mut active_cards = parse_cards(value.get("boardAnimalCards"), &card_cube_counts, false);
    active_cards.extend(parse_player_spirits(
        gamedatas.get("spiritsCards"),
        player_id,
        &card_cube_counts,
    ));

    PlayerState {
        player_id: player_id.to_owned(),
        cells,
        active_cards,
        completed_cards: parse_cards(value.get("doneAnimalCards"), &card_cube_counts, true),
        empty_hexes: value
            .get("emptyHexes")
            .and_then(Value::as_u64)
            .unwrap_or(0)
            .min(u8::MAX as u64) as u8,
    }
}

fn parse_hexes(value: Option<&Value>) -> Option<Vec<Coord>> {
    let hexes = value?.as_array()?;
    Some(
        hexes
            .iter()
            .filter_map(|hex| {
                Some(Coord {
                    col: hex.get("col")?.as_i64()? as i8,
                    row: hex.get("row")?.as_i64()? as i8,
                })
            })
            .collect(),
    )
}

fn parse_board_side(value: Option<&Value>) -> BoardSide {
    match value.and_then(Value::as_str) {
        Some("sideB") | Some("SideB") => BoardSide::SideB,
        _ => BoardSide::SideA,
    }
}

fn parse_central_groups(value: Option<&Value>) -> Vec<Vec<Color>> {
    let Some(object) = value.and_then(Value::as_object) else {
        return Vec::new();
    };
    let mut groups: Vec<_> = object.iter().collect();
    groups.sort_by_key(|(key, _)| key.parse::<u8>().unwrap_or(u8::MAX));
    groups
        .into_iter()
        .map(|(_, tokens)| parse_token_list(tokens))
        .collect()
}

fn parse_tokens_on_board(value: Option<&Value>) -> HashMap<String, Vec<Color>> {
    let mut stacks: HashMap<String, Vec<(u64, Color)>> = HashMap::new();
    match value {
        Some(Value::Object(object)) => {
            for (cell, tokens) in object {
                for token in tokens.as_array().into_iter().flatten() {
                    if let Some(color) = token_color(token) {
                        let level = token
                            .get("location_arg")
                            .and_then(Value::as_u64)
                            .unwrap_or(1);
                        stacks.entry(cell.clone()).or_default().push((level, color));
                    }
                }
            }
        }
        Some(Value::Array(tokens)) => {
            for token in tokens {
                let Some(cell) = token.get("location").and_then(Value::as_str) else {
                    continue;
                };
                if let Some(color) = token_color(token) {
                    let level = token
                        .get("location_arg")
                        .and_then(Value::as_u64)
                        .unwrap_or(1);
                    stacks
                        .entry(cell.to_owned())
                        .or_default()
                        .push((level, color));
                }
            }
        }
        _ => {}
    }

    stacks
        .into_iter()
        .map(|(cell, mut tokens)| {
            tokens.sort_by_key(|(level, _)| *level);
            (cell, tokens.into_iter().map(|(_, color)| color).collect())
        })
        .collect()
}

fn parse_token_list(value: &Value) -> Vec<Color> {
    value
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(token_color)
        .collect()
}

fn parse_cards(
    value: Option<&Value>,
    cube_counts: &HashMap<u32, u8>,
    completed: bool,
) -> Vec<ActiveCard> {
    value
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|card| {
            let card_id = card.get("id")?.as_u64()? as u32;
            Some(ActiveCard {
                card_id,
                type_arg: card.get("type_arg")?.as_u64()? as u8,
                remaining_cubes: if completed {
                    0
                } else {
                    cube_counts.get(&card_id).copied().unwrap_or_else(|| {
                        card.get("pointLocations")
                            .and_then(Value::as_array)
                            .map(|points| points.len() as u8)
                            .unwrap_or(0)
                    })
                },
                is_spirit: card
                    .get("isSpirit")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
            })
        })
        .collect()
}

fn parse_player_spirits(
    value: Option<&Value>,
    player_id: &str,
    cube_counts: &HashMap<u32, u8>,
) -> Vec<ActiveCard> {
    value
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|card| {
            card.get("location_arg")
                .map(|location| value_matches_id(location, player_id))
                .unwrap_or(false)
        })
        .filter_map(|card| {
            let card_id = card.get("id")?.as_u64()? as u32;
            Some(ActiveCard {
                card_id,
                type_arg: card.get("type_arg")?.as_u64()? as u8,
                remaining_cubes: cube_counts.get(&card_id).copied().unwrap_or(1),
                is_spirit: true,
            })
        })
        .collect()
}

fn count_card_cubes(value: Option<&Value>) -> HashMap<u32, u8> {
    let mut counts = HashMap::new();
    for cube in value.and_then(Value::as_array).into_iter().flatten() {
        let Some(location) = cube.get("location").and_then(Value::as_str) else {
            continue;
        };
        if let Some(raw_id) = location.strip_prefix("card_") {
            if let Ok(card_id) = raw_id.parse::<u32>() {
                *counts.entry(card_id).or_insert(0) += 1;
            }
        }
    }
    counts
}

fn collect_cube_locations(value: Option<&Value>) -> HashSet<String> {
    value
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|cube| cube.get("location").and_then(Value::as_str))
        .filter(|location| location.starts_with("cell_"))
        .map(str::to_owned)
        .collect()
}

fn token_color(token: &Value) -> Option<Color> {
    token
        .get("type_arg")
        .and_then(Value::as_u64)
        .and_then(|raw| Color::from_bga_type_arg(raw as u8))
}

fn active_player_id(gamedatas: &Value) -> Option<String> {
    gamedatas
        .get("gamestate")
        .and_then(|state| state.get("active_player"))
        .and_then(|value| {
            value
                .as_str()
                .map(str::to_owned)
                .or_else(|| value.as_u64().map(|id| id.to_string()))
        })
}

fn string_field(value: &Value, key: &str) -> Option<String> {
    value.get(key).and_then(Value::as_str).map(str::to_owned)
}

fn value_matches_id(value: &Value, player_id: &str) -> bool {
    value
        .as_str()
        .map(|raw| raw == player_id)
        .or_else(|| value.as_u64().map(|raw| raw.to_string() == player_id))
        .unwrap_or(false)
}

fn cell_key(player_id: &str, coord: Coord) -> String {
    format!("cell_{player_id}_{}_{}", coord.col, coord.row)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn normalizes_observed_tutorial_shape() {
        let raw = json!({
            "version": "230603",
            "boardSide": "sideA",
            "hexes": [{"col": 0, "row": 0}, {"col": 1, "row": 0}],
            "gamestate": {"active_player": "p1"},
            "players": {
                "p1": {
                    "emptyHexes": 1,
                    "tokensOnBoard": {
                        "cell_p1_0_0": [
                            {"location_arg": 1, "type_arg": 3},
                            {"location_arg": 2, "type_arg": 4}
                        ]
                    },
                    "boardAnimalCards": [],
                    "doneAnimalCards": []
                }
            },
            "tokensOnCentralBoard": {
                "1": [{"type_arg": 2}, {"type_arg": 1}, {"type_arg": 4}]
            },
            "river": [{"id": 8, "type_arg": 22, "pointLocations": [3, 6, 10, 15], "isSpirit": false}],
            "spiritsCards": [],
            "cubesOnAnimalCards": []
        });
        let snapshot = normalize_gamedatas(&raw, Some("p1")).unwrap();
        assert_eq!(snapshot.board_side, BoardSide::SideA);
        assert_eq!(
            snapshot.players[0].cells[0].stack.tokens,
            vec![Color::Trunk, Color::Foliage]
        );
        assert_eq!(
            snapshot.central_token_groups[0],
            vec![Color::Mountain, Color::Water, Color::Foliage]
        );
        assert_eq!(snapshot.river_cards[0].type_arg, 22);
    }
}
