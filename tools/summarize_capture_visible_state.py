from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any

from tools.dom_capture_to_snapshot import convert


def as_object(value: object) -> dict[str, Any]:
    return value if isinstance(value, dict) else {}


def card_label(card: dict[str, Any]) -> str:
    return (
        f"{card.get('cardId')}/type{card.get('typeArg')}"
        f"/cubes{card.get('remainingCubes')}"
        f"{'/spirit' if card.get('isSpirit') else ''}"
    )


def summarize(path: Path) -> dict[str, Any]:
    capture = json.loads(path.read_text(encoding="utf-8"))
    visible = as_object(capture.get("visibleStateV1"))
    reliability = as_object(visible.get("reliability"))
    snapshot = convert(path)
    active_id = str(snapshot.get("activePlayerId") or "")
    active = next(
        (player for player in snapshot.get("players", []) if player.get("playerId") == active_id),
        None,
    )
    return {
        "file": str(path),
        "state": as_object(capture.get("context")).get("gameStateName"),
        "activePlayerId": active_id,
        "activePlayerName": active_name(capture, active_id),
        "boardSide": snapshot.get("boardSide"),
        "visibleReliability": reliability,
        "centralTokenGroups": snapshot.get("centralTokenGroups"),
        "activeCards": [card_label(card) for card in as_object(active).get("activeCards", [])],
        "completedCards": [card_label(card) for card in as_object(active).get("completedCards", [])],
        "riverCards": [card_label(card) for card in snapshot.get("riverCards", [])],
        "activeEmptyHexes": as_object(active).get("emptyHexes"),
    }


def active_name(capture: dict[str, Any], player_id: str) -> str | None:
    players = as_object(as_object(capture.get("gamedatas")).get("players"))
    player = as_object(players.get(player_id))
    name = player.get("name") or player.get("player_name")
    return str(name) if name else None


def main() -> int:
    parser = argparse.ArgumentParser(description="Summarize Harmonies capture visible state.")
    parser.add_argument("captures", nargs="+", type=Path)
    args = parser.parse_args()
    print(json.dumps([summarize(path) for path in args.captures], indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
