from __future__ import annotations

import argparse
import json
import re
from pathlib import Path
from typing import Any


PLAYER_ID_KEYS = {"player_id", "active_player", "current_player_id"}
NAME_KEYS = {"player_name", "name", "avatar", "flag"}


def anonymize_value(value: Any, player_map: dict[str, str]) -> Any:
    if isinstance(value, dict):
        return anonymize_dict(value, player_map)
    if isinstance(value, list):
        return [anonymize_value(item, player_map) for item in value]
    if isinstance(value, str):
        return anonymize_string(value, player_map)
    if isinstance(value, int) and str(value) in player_map:
        return player_map[str(value)]
    return value


def anonymize_dict(data: dict[str, Any], player_map: dict[str, str]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in data.items():
        mapped_key = player_map.get(key, anonymize_string(key, player_map))
        if key in NAME_KEYS:
            result[mapped_key] = "<redacted>"
        elif key in PLAYER_ID_KEYS and isinstance(value, str | int):
            raw_value = str(value)
            result[mapped_key] = player_map.setdefault(raw_value, f"player_{len(player_map) + 1}")
        else:
            result[mapped_key] = anonymize_value(value, player_map)
    return result


def anonymize_string(value: str, player_map: dict[str, str]) -> str:
    if value in player_map:
        return player_map[value]
    result = value
    for raw_id, mapped_id in sorted(player_map.items(), key=lambda item: len(item[0]), reverse=True):
        result = replace_player_id_fragment(result, raw_id, mapped_id)
    return result


def replace_player_id_fragment(value: str, raw_id: str, mapped_id: str) -> str:
    replacements = [
        (rf"cell_{re.escape(raw_id)}_", f"cell_{mapped_id}_"),
        (rf"board{re.escape(raw_id)}\b", f"board{mapped_id}"),
        (rf"done{re.escape(raw_id)}\b", f"done{mapped_id}"),
        (rf"cardontable_{re.escape(raw_id)}_", f"cardontable_{mapped_id}_"),
    ]
    result = value
    for pattern, replacement in replacements:
        result = re.sub(pattern, replacement, result)
    return result


def collect_player_ids(data: Any, player_map: dict[str, str]) -> None:
    if isinstance(data, dict):
        players = data.get("players")
        if isinstance(players, dict):
            playerorder = data.get("playerorder")
            for player_id, player in players.items():
                mapped_id = player_map.setdefault(str(player_id), next_player_label(player_id, player_map))
                collect_player_aliases(str(player_id), player, playerorder, mapped_id, player_map)
        for value in data.values():
            collect_player_ids(value, player_map)
    elif isinstance(data, list):
        for item in data:
            collect_player_ids(item, player_map)


def next_player_label(player_id: object, player_map: dict[str, str]) -> str:
    raw_id = str(player_id)
    if re.fullmatch(r"player_\d+", raw_id):
        return raw_id
    used = {value for value in player_map.values() if re.fullmatch(r"player_\d+", value)}
    return f"player_{len(used) + 1}"


def collect_player_aliases(
    player_id: str,
    player: Any,
    playerorder: Any,
    mapped_id: str,
    player_map: dict[str, str],
) -> None:
    if not isinstance(player, dict):
        return
    raw_id = player.get("id")
    if raw_id is not None:
        player_map.setdefault(str(raw_id), mapped_id)
    player_no = player.get("playerNo")
    if isinstance(player_no, int) and isinstance(playerorder, list) and 0 < player_no <= len(playerorder):
        player_map.setdefault(str(playerorder[player_no - 1]), mapped_id)
    for alias in infer_location_player_ids(player):
        player_map.setdefault(alias, mapped_id)
    player_map.setdefault(player_id, mapped_id)


def infer_location_player_ids(player: dict[str, Any]) -> set[str]:
    aliases: set[str] = set()
    tokens_on_board = player.get("tokensOnBoard")
    if isinstance(tokens_on_board, dict):
        aliases.update(filter(None, (cell_key_player_id(key) for key in tokens_on_board)))
    cubes_on_board = player.get("animalCubesOnBoard")
    if isinstance(cubes_on_board, dict):
        aliases.update(filter(None, (cell_key_player_id(key) for key in cubes_on_board)))
    if isinstance(cubes_on_board, list):
        aliases.update(filter(None, (cell_key_player_id(str(key)) for key in cubes_on_board)))
    return aliases


def cell_key_player_id(key: str) -> str:
    match = re.match(r"^cell_(.+)_-?\d+_-?\d+$", key)
    return match.group(1) if match else ""


def anonymize_snapshot(data: dict[str, Any]) -> dict[str, Any]:
    player_map: dict[str, str] = {}
    collect_player_ids(data, player_map)
    return anonymize_dict(data, player_map)


def main() -> None:
    parser = argparse.ArgumentParser(description="Anonymize BGA gamedatas snapshot JSON.")
    parser.add_argument("input", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()

    data = json.loads(args.input.read_text(encoding="utf-8"))
    anonymized = anonymize_snapshot(data)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(anonymized, indent=2, sort_keys=True), encoding="utf-8")


if __name__ == "__main__":
    main()
