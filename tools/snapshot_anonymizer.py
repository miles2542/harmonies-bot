from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any


PLAYER_ID_KEYS = {"player_id", "active_player", "current_player_id"}
NAME_KEYS = {"player_name", "name", "avatar", "flag"}


def anonymize_value(value: Any, player_map: dict[str, str]) -> Any:
    if isinstance(value, dict):
        return anonymize_dict(value, player_map)
    if isinstance(value, list):
        return [anonymize_value(item, player_map) for item in value]
    if isinstance(value, str) and value in player_map:
        return player_map[value]
    return value


def anonymize_dict(data: dict[str, Any], player_map: dict[str, str]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in data.items():
        mapped_key = player_map.get(key, key)
        if key in NAME_KEYS:
            result[mapped_key] = "<redacted>"
        elif key in PLAYER_ID_KEYS and isinstance(value, str):
            result[mapped_key] = player_map.setdefault(value, f"player_{len(player_map) + 1}")
        else:
            result[mapped_key] = anonymize_value(value, player_map)
    return result


def collect_player_ids(data: Any, player_map: dict[str, str]) -> None:
    if isinstance(data, dict):
        players = data.get("players")
        if isinstance(players, dict):
            for player_id in players:
                player_map.setdefault(str(player_id), f"player_{len(player_map) + 1}")
        for value in data.values():
            collect_player_ids(value, player_map)
    elif isinstance(data, list):
        for item in data:
            collect_player_ids(item, player_map)


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
