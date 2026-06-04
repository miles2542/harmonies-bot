from __future__ import annotations

import argparse
import json
import re
import sys
from dataclasses import dataclass
from pathlib import Path

CELL_RE = re.compile(r"^cell_(\d+)_(\-?\d+)_(\-?\d+)$")
CARD_RE = re.compile(r"^card_(\d+)$")
COLOR_RE = re.compile(r"\bcolor-(\d+)\b")
LEVEL_RE = re.compile(r"\blevel-(\d+)\b")
PLAYER_TABLE_RE = re.compile(r"^player-table-(\d+)$")

COLOR_BY_CLASS = {
    "1": "water",
    "2": "mountain",
    "3": "trunk",
    "4": "foliage",
    "5": "field",
    "6": "building",
    "7": "building",
}


@dataclass(frozen=True)
class Rect:
    x: int
    y: int
    width: int
    height: int


@dataclass(frozen=True)
class DomNode:
    node_id: str
    class_name: str
    dataset: dict[str, str]
    text: str
    rect: Rect


def as_object(value: object) -> dict[str, object]:
    return value if isinstance(value, dict) else {}


def as_list(value: object) -> list[object]:
    return value if isinstance(value, list) else []


def string_value(value: object) -> str:
    return value if isinstance(value, str) else ""


def int_value(value: object) -> int:
    return value if isinstance(value, int) else 0


def read_rect(value: object) -> Rect:
    raw = as_object(value)
    return Rect(
        x=int_value(raw.get("x")),
        y=int_value(raw.get("y")),
        width=int_value(raw.get("width")),
        height=int_value(raw.get("height")),
    )


def read_node(value: object) -> DomNode:
    raw = as_object(value)
    dataset = {
        key: string_value(item)
        for key, item in as_object(raw.get("dataset")).items()
        if isinstance(item, str)
    }
    return DomNode(
        node_id=string_value(raw.get("id")),
        class_name=string_value(raw.get("className")),
        dataset=dataset,
        text=string_value(raw.get("text")),
        rect=read_rect(raw.get("rect")),
    )


def contains(outer: Rect, inner: Rect) -> bool:
    if outer.width <= 0 or outer.height <= 0 or inner.width <= 0 or inner.height <= 0:
        return False
    return (
        inner.x >= outer.x
        and inner.y >= outer.y
        and inner.x + inner.width <= outer.x + outer.width
        and inner.y + inner.height <= outer.y + outer.height
    )


def center_inside(outer: Rect, inner: Rect) -> bool:
    if outer.width <= 0 or outer.height <= 0 or inner.width <= 0 or inner.height <= 0:
        return False
    center_x = inner.x + inner.width / 2
    center_y = inner.y + inner.height / 2
    return (
        outer.x <= center_x <= outer.x + outer.width
        and outer.y <= center_y <= outer.y + outer.height
    )


def parse_color(class_name: str) -> str | None:
    match = COLOR_RE.search(class_name)
    if not match:
        return None
    return COLOR_BY_CLASS.get(match.group(1))


def parse_level(class_name: str) -> int:
    match = LEVEL_RE.search(class_name)
    return int(match.group(1)) if match else 1


def read_nodes(path: Path) -> list[DomNode]:
    data = json.loads(path.read_text(encoding="utf-8"))
    dom = as_object(as_object(data).get("domSnapshot"))
    nodes = [read_node(item) for item in as_list(dom.get("nodes"))]
    if not nodes:
        raise SystemExit("snapshot has no domSnapshot.nodes")
    return nodes


def player_ids(nodes: list[DomNode]) -> list[str]:
    ids = []
    for node in nodes:
        match = PLAYER_TABLE_RE.match(node.node_id)
        if match:
            ids.append(match.group(1))
    return ids


def board_side(nodes: list[DomNode]) -> str:
    for node in nodes:
        if node.node_id == "overall-content":
            if "sideB" in node.class_name:
                return "sideB"
            if "sideA" in node.class_name:
                return "sideA"
    return "sideA"


def cells_for_player(nodes: list[DomNode], player_id: str) -> list[dict[str, object]]:
    cell_nodes = [
        node
        for node in nodes
        if (match := CELL_RE.match(node.node_id)) is not None and match.group(1) == player_id
    ]
    result = []
    for cell in sorted(cell_nodes, key=lambda node: cell_sort_key(node.node_id)):
        match = CELL_RE.match(cell.node_id)
        if not match:
            continue
        tokens = tokens_in_cell(nodes, cell)
        result.append(
            {
                "coord": {"col": int(match.group(2)), "row": int(match.group(3))},
                "stack": {"tokens": tokens},
                "lockedByCube": cube_in_cell(nodes, cell),
            }
        )
    return result


def cell_sort_key(node_id: str) -> tuple[int, int]:
    match = CELL_RE.match(node_id)
    if not match:
        return (99, 99)
    return (int(match.group(3)), int(match.group(2)))


def tokens_in_cell(nodes: list[DomNode], cell: DomNode) -> list[str]:
    stacked = []
    for node in nodes:
        if "colored-token" not in node.class_name or not node.node_id.startswith("tokenOnBoard_"):
            continue
        if not center_inside(cell.rect, node.rect):
            continue
        color = parse_color(node.class_name)
        if color:
            stacked.append((parse_level(node.class_name), color))
    return [color for _, color in sorted(stacked)]


def cube_in_cell(nodes: list[DomNode], cell: DomNode) -> bool:
    return any(
        "animal-cube" in node.class_name and center_inside(cell.rect, node.rect)
        for node in nodes
    )


def cards_for_player(
    nodes: list[DomNode],
    player_id: str,
    container_prefix: str,
    completed: bool,
) -> list[dict[str, object]]:
    container = next(
        (node for node in nodes if node.node_id == f"{container_prefix}-{player_id}"),
        None,
    )
    if container is None:
        return []
    cards = []
    for node in nodes:
        match = CARD_RE.match(node.node_id)
        if not match or not contains(container.rect, node.rect):
            continue
        type_arg = node.dataset.get("cardTypeArg")
        if not type_arg:
            continue
        card_id = int(match.group(1))
        cards.append(
            {
                "cardId": card_id,
                "typeArg": int(type_arg),
                "remainingCubes": 0 if completed else remaining_cubes(nodes, card_id),
                "isSpirit": node.dataset.get("isSpirit") == "true",
            }
        )
    return sorted(cards, key=lambda card: int(card["cardId"]))


def remaining_cubes(nodes: list[DomNode], card_id: int) -> int:
    prefix = f"card_{card_id}-score-"
    return sum(
        1
        for node in nodes
        if node.node_id.startswith(prefix)
        and "points-location" in node.class_name
        and "animal-cube" in node.class_name
    )


def central_groups(nodes: list[DomNode]) -> list[list[str]]:
    groups = []
    for hole in range(1, 6):
        tokens = []
        for node in nodes:
            if not node.node_id.startswith(f"hole-{hole}-token-"):
                continue
            color = parse_color(node.class_name)
            if color:
                tokens.append(color)
        groups.append(tokens)
    return groups


def empty_hexes(cells: list[dict[str, object]]) -> int:
    return sum(1 for cell in cells if not as_object(cell.get("stack")).get("tokens"))


def convert(path: Path) -> dict[str, object]:
    nodes = read_nodes(path)
    players = []
    ids = player_ids(nodes)
    for player_id in ids:
        cells = cells_for_player(nodes, player_id)
        players.append(
            {
                "playerId": player_id,
                "cells": cells,
                "activeCards": cards_for_player(nodes, player_id, "hand", completed=False),
                "completedCards": cards_for_player(nodes, player_id, "done", completed=True),
                "emptyHexes": empty_hexes(cells),
            }
        )
    if not players:
        raise SystemExit("no player-table DOM nodes found")
    return {
        "schemaVersion": 1,
        "perspectivePlayerId": ids[0],
        "activePlayerId": ids[0],
        "boardSide": board_side(nodes),
        "players": players,
        "centralTokenGroups": central_groups(nodes),
        "riverCards": [],
        "bagCounts": {
            "water": 0,
            "mountain": 0,
            "trunk": 0,
            "foliage": 0,
            "field": 0,
            "building": 0,
            "unknown": 0,
        },
        "cardsCatalogVersion": "dom-capture",
    }


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Convert BGA result DOM capture to GameSnapshotV1.",
    )
    parser.add_argument("snapshot", type=Path)
    parser.add_argument("--out", type=Path)
    args = parser.parse_args()

    output = json.dumps(convert(args.snapshot), indent=2)
    if args.out:
        args.out.write_text(output + "\n", encoding="utf-8")
    else:
        sys.stdout.write(output + "\n")


if __name__ == "__main__":
    main()
