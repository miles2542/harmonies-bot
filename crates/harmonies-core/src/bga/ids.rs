use std::collections::{HashMap, HashSet};

use serde_json::Value;

use crate::model::Coord;

pub(super) fn map_player_ids(gamedatas: &Value) -> HashMap<String, String> {
    let mut ids = HashMap::new();
    let Some(players) = gamedatas.get("players").and_then(Value::as_object) else {
        return ids;
    };
    for (key, player) in players {
        ids.insert(key.clone(), key.clone());
        if let Some(id) = string_field(player, "id") {
            ids.insert(id, key.clone());
        }
        let inferred_ids = infer_player_ids_from_locations(player);
        for inferred in &inferred_ids {
            ids.insert(inferred.clone(), key.clone());
        }
        if inferred_ids.is_empty() {
            if let Some(order_id) = player_order_id_for_player(gamedatas, player) {
                ids.insert(order_id, key.clone());
            }
        }
    }
    ids
}

pub(super) fn bga_ids_for_player(
    player_id: &str,
    player: &Value,
    gamedatas: &Value,
    mapped_ids: &HashMap<String, String>,
) -> Vec<String> {
    let mut ids = vec![player_id.to_owned()];
    if let Some(id) = string_field(player, "id") {
        ids.push(id);
    }
    let inferred = infer_player_ids_from_locations(player);
    if inferred.is_empty() {
        if let Some(order_id) = player_order_id_for_player(gamedatas, player) {
            ids.push(order_id);
        }
    } else {
        ids.extend(inferred);
    }
    ids.extend(
        mapped_ids
            .iter()
            .filter(|(_, mapped)| mapped.as_str() == player_id)
            .map(|(raw, _)| raw.clone()),
    );
    ids.sort();
    ids.dedup();
    ids
}

pub(super) fn cell_key(player_id: &str, coord: Coord) -> String {
    format!("cell_{player_id}_{}_{}", coord.col, coord.row)
}

pub(super) fn collect_all_cube_locations(gamedatas: &Value) -> HashSet<String> {
    let mut locations = collect_cube_locations(gamedatas.get("cubesOnAnimalCards"));
    collect_player_cube_locations(gamedatas, &mut locations);
    locations
}

pub(super) fn collect_single_player_cube_locations(
    player: &Value,
    locations: &mut HashSet<String>,
) {
    match player.get("animalCubesOnBoard") {
        Some(Value::Array(items)) => {
            for location in items.iter().filter_map(Value::as_str) {
                locations.insert(location.to_owned());
            }
        }
        Some(Value::Object(items)) => {
            locations.extend(items.keys().cloned());
        }
        _ => {}
    }
}

pub(super) fn collect_single_player_cube_coords(player: &Value) -> HashSet<Coord> {
    let mut locations = HashSet::new();
    collect_single_player_cube_locations(player, &mut locations);
    locations
        .into_iter()
        .filter_map(|location| cell_key_coord(&location))
        .collect()
}

fn player_order_id_for_player(gamedatas: &Value, player: &Value) -> Option<String> {
    let player_no = player.get("playerNo")?.as_u64()? as usize;
    if player_no == 0 {
        return None;
    }
    gamedatas
        .get("playerorder")
        .and_then(Value::as_array)
        .and_then(|order| order.get(player_no - 1))
        .and_then(|value| {
            value
                .as_str()
                .map(str::to_owned)
                .or_else(|| value.as_u64().map(|id| id.to_string()))
        })
}

fn infer_player_ids_from_locations(player: &Value) -> Vec<String> {
    let mut ids = Vec::new();
    if let Some(tokens) = player.get("tokensOnBoard").and_then(Value::as_object) {
        ids.extend(tokens.keys().filter_map(|key| cell_key_player_id(key)));
    }
    for field in ["boardAnimalCards", "doneAnimalCards"] {
        for card in player
            .get(field)
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            if let Some(location) = card.get("location").and_then(Value::as_str) {
                ids.extend(location.strip_prefix("board").map(str::to_owned));
                ids.extend(location.strip_prefix("done").map(str::to_owned));
            }
        }
    }
    ids.sort();
    ids.dedup();
    ids
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

fn collect_player_cube_locations(gamedatas: &Value, locations: &mut HashSet<String>) {
    for player in gamedatas
        .get("players")
        .and_then(Value::as_object)
        .into_iter()
        .flat_map(|players| players.values())
    {
        collect_single_player_cube_locations(player, locations);
    }
}

fn cell_key_player_id(key: &str) -> Option<String> {
    let rest = key.strip_prefix("cell_")?;
    let mut parts = rest.rsplitn(3, '_');
    parts.next()?.parse::<i8>().ok()?;
    parts.next()?.parse::<i8>().ok()?;
    Some(parts.next()?.to_owned())
}

fn cell_key_coord(key: &str) -> Option<Coord> {
    let rest = key.strip_prefix("cell_")?;
    let mut parts = rest.rsplitn(3, '_');
    let row = parts.next()?.parse::<i8>().ok()?;
    let col = parts.next()?.parse::<i8>().ok()?;
    Some(Coord { col, row })
}

fn string_field(value: &Value, key: &str) -> Option<String> {
    value.get(key).and_then(Value::as_str).map(str::to_owned)
}
