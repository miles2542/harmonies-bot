from __future__ import annotations

import argparse
import json
import re
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Any

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
OFFICIAL_TOKEN_COUNTS = {
    "water": 23,
    "mountain": 23,
    "trunk": 21,
    "foliage": 19,
    "field": 19,
    "building": 15,
}
CARD_CATALOG_PATH = Path("docs/cards_database.json")


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


def bool_value(value: object) -> bool:
    return value if isinstance(value, bool) else False


def number_value(value: object) -> int:
    if isinstance(value, bool):
        return 0
    if isinstance(value, int):
        return value
    if isinstance(value, float) and value.is_integer():
        return int(value)
    if isinstance(value, str) and value.strip().lstrip("-").isdigit():
        return int(value)
    return 0


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
    matches = LEVEL_RE.findall(class_name)
    return int(matches[-1]) if matches else 1


def read_nodes(path: Path) -> list[DomNode]:
    data = json.loads(path.read_text(encoding="utf-8"))
    return read_nodes_from_capture(data)


def read_nodes_from_capture(data: dict[str, Any]) -> list[DomNode]:
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


def cells_from_gamedatas(data: dict[str, Any], player_id: str) -> list[dict[str, object]]:
    COLOR_BY_TYPE_ARG = {
        1: "water",
        2: "mountain",
        3: "trunk",
        4: "foliage",
        5: "field",
        6: "building",
        7: "building",
    }
    gamedatas = as_object(data.get("gamedatas", data))
    players = as_object(gamedatas.get("players", {}))
    player = as_object(players.get(player_id, {}))
    
    tokens_map = as_object(player.get("tokensOnBoard", {}))
    cubes_map = as_object(player.get("animalCubesOnBoard", {}))
    
    result = []
    for row in range(5):
        for col in range(5):
            cell_key = f"cell_{player_id}_{col}_{row}"
            # Extract tokens
            raw_tokens = as_list(tokens_map.get(cell_key))
            # Sort raw_tokens by location_arg (level)
            sorted_tokens = sorted(raw_tokens, key=lambda t: number_value(as_object(t).get("location_arg")))
            tokens = []
            for t in sorted_tokens:
                color_id = number_value(as_object(t).get("type_arg"))
                color_name = COLOR_BY_TYPE_ARG.get(color_id)
                if color_name:
                    tokens.append(color_name)
            
            # Extract lockedByCube
            raw_cubes = as_list(cubes_map.get(cell_key))
            locked = len(raw_cubes) > 0
            
            result.append({
                "coord": {"col": col, "row": row},
                "stack": {"tokens": tokens},
                "lockedByCube": locked
            })
    return sorted(
        result,
        key=lambda cell: (
            int(as_object(cell["coord"]).get("row") or 0),
            int(as_object(cell["coord"]).get("col") or 0),
        ),
    )


def river_cards_from_gamedatas(data: dict[str, Any]) -> list[dict[str, object]]:
    gamedatas = as_object(data.get("gamedatas", data))
    raw_river = as_list(gamedatas.get("river", []))
    catalog_cube_counts = catalog_cube_count_by_type_arg()
    result = []
    for raw in raw_river:
        card = as_object(raw)
        card_id = int(number_value(card.get("id")))
        type_arg = int(number_value(card.get("type_arg")))
        is_spirit = bool_value(card.get("isSpirit"))
        max_cubes = catalog_cube_counts.get(type_arg, 0)
        result.append({
            "cardInstanceId": card_id,
            "cardId": card_id,
            "typeArg": type_arg,
            "isSpirit": is_spirit,
            "remainingCubes": max_cubes,
        })
    return sorted(result, key=lambda c: c["cardId"])


def spirit_choices_from_gamedatas(data: dict[str, Any]) -> list[dict[str, object]]:
    gamedatas = as_object(data.get("gamedatas", data))
    raw_spirits = as_list(gamedatas.get("spiritsCards", []))
    catalog_cube_counts = catalog_cube_count_by_type_arg()
    result = []
    for raw in raw_spirits:
        card = as_object(raw)
        card_id = int(number_value(card.get("id")))
        type_arg = int(number_value(card.get("type_arg")))
        is_spirit = bool_value(card.get("isSpirit"))
        max_cubes = catalog_cube_counts.get(type_arg, 0)
        result.append({
            "cardInstanceId": card_id,
            "cardId": card_id,
            "typeArg": type_arg,
            "isSpirit": is_spirit,
            "remainingCubes": max_cubes,
        })
    return sorted(result, key=lambda c: c["cardId"])


def cards_from_gamedatas(data: dict[str, Any], player_id: str, key: str) -> list[dict[str, object]]:
    gamedatas = as_object(data.get("gamedatas", data))
    players = as_object(gamedatas.get("players", {}))
    player = as_object(players.get(player_id, {}))
    raw_cards = as_list(player.get(key, []))
    cubes_list = as_list(gamedatas.get("cubesOnAnimalCards", []))
    result = []
    for raw in raw_cards:
        card = as_object(raw)
        card_id = int(number_value(card.get("id")))
        type_arg = int(number_value(card.get("type_arg")))
        is_spirit = bool_value(card.get("isSpirit"))
        prefix = f"card_{card_id}"
        remaining = sum(
            1 for c in cubes_list
            if string_value(as_object(c).get("location")) == prefix
        )
        if key == "doneAnimalCards":
            remaining = 0
        result.append({
            "cardInstanceId": card_id,
            "cardId": card_id,
            "typeArg": type_arg,
            "isSpirit": is_spirit,
            "remainingCubes": remaining,
        })
    return sorted(result, key=lambda c: c["cardId"])


def cell_sort_key(node_id: str) -> tuple[int, int]:
    match = CELL_RE.match(node_id)
    if not match:
        return (99, 99)
    return (int(match.group(3)), int(match.group(2)))


def tokens_in_cell(nodes: list[DomNode], cell: DomNode) -> list[str]:
    stacked = []
    for node in nodes:
        if "colored-token" not in node.class_name:
            continue
        if not center_inside(cell.rect, node.rect):
            continue
        color = parse_color(node.class_name)
        if color:
            stacked.append((parse_level(node.class_name), color))
    return [color for _, color in sorted(stacked)]


def cube_in_cell(nodes: list[DomNode], cell: DomNode) -> bool:
    outer = cell.rect
    if outer.width <= 0 or outer.height <= 0:
        return False
    for node in nodes:
        if "animal-cube" not in node.class_name:
            continue
        inner = node.rect
        if inner.width <= 0 or inner.height <= 0:
            continue
        center_x = inner.x + inner.width / 2
        center_y = inner.y + inner.height / 2
        if (
            outer.x <= center_x <= outer.x + outer.width
            and outer.y - 65 <= center_y <= outer.y + outer.height * 0.6
        ):
            return True
    return False


def catalog_cube_count_by_type_arg(path: Path = CARD_CATALOG_PATH) -> dict[int, int]:
    if not path.exists():
        return {}
    raw = json.loads(path.read_text(encoding="utf-8"))
    return {
        int(card["type_arg"]): len(as_list(card.get("pointLocations")))
        for card in as_object(raw).values()
        if number_value(as_object(card).get("type_arg")) > 0
    }


def cards_for_player(
    nodes: list[DomNode],
    player_id: str,
    container_prefix: str,
    completed: bool,
    card_point_counts: dict[int, int] | None = None,
    catalog_cube_counts: dict[int, int] | None = None,
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
        type_arg_int = int(type_arg)
        cards.append(
            {
                "cardId": card_id,
                "typeArg": type_arg_int,
                "remainingCubes": 0
                if completed
                else remaining_cubes(nodes, card_id)
                or as_object(card_point_counts).get(card_id, 0)
                or as_object(catalog_cube_counts).get(type_arg_int, 0),
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


def river_cards(
    nodes: list[DomNode],
    card_point_counts: dict[int, int] | None = None,
    catalog_cube_counts: dict[int, int] | None = None,
) -> list[dict[str, object]]:
    player_tops = [
        node.rect.y
        for node in nodes
        if PLAYER_TABLE_RE.match(node.node_id) and node.rect.width > 0 and node.rect.height > 0
    ]
    if not player_tops:
        return []
    first_player_top = min(player_tops)
    player_containers = [
        node.rect
        for node in nodes
        if re.match(r"^(hand|done)-\d+$", node.node_id)
        and node.rect.width > 0
        and node.rect.height > 0
    ]
    cards = []
    for node in nodes:
        match = CARD_RE.match(node.node_id)
        if not match or node.rect.y >= first_player_top - 8:
            continue
        if any(center_inside(container, node.rect) for container in player_containers):
            continue
        type_arg = node.dataset.get("cardTypeArg")
        if not type_arg:
            continue
        card_id = int(match.group(1))
        type_arg_int = int(type_arg)
        cards.append(
            {
                "cardId": card_id,
                "typeArg": type_arg_int,
                "remainingCubes": remaining_cubes(nodes, card_id)
                or as_object(card_point_counts).get(card_id, 0)
                or as_object(catalog_cube_counts).get(type_arg_int, 0),
                "isSpirit": node.dataset.get("isSpirit") == "true",
            }
        )
    return sorted(cards, key=lambda card: (0 if not card["isSpirit"] else 1, int(card["cardId"])))


def empty_hexes(cells: list[dict[str, object]]) -> int:
    return sum(1 for cell in cells if not as_object(cell.get("stack")).get("tokens"))


def infer_bag_counts(
    capture: dict[str, object],
    players: list[dict[str, object]],
    central_token_groups: list[list[str]],
) -> dict[str, int]:
    counts = dict(OFFICIAL_TOKEN_COUNTS)
    for color in visible_token_colors(players, central_token_groups):
        if color in counts:
            counts[color] = max(0, counts[color] - 1)
    known_total = sum(counts.values())
    reported_total = number_value(as_object(capture.get("gamedatas")).get("remainingTokens"))
    counts["unknown"] = max(0, reported_total - known_total) if reported_total else 0
    return counts


def visible_token_colors(
    players: list[dict[str, object]],
    central_token_groups: list[list[str]],
) -> list[str]:
    colors: list[str] = []
    for player in players:
        for cell in as_list(player.get("cells")):
            colors.extend(
                string_value(token)
                for token in as_list(as_object(as_object(cell).get("stack")).get("tokens"))
                if string_value(token)
            )
    for group in central_token_groups:
        colors.extend(group)
    return colors


def visible_card(card: object) -> dict[str, object] | None:
    raw = as_object(card)
    card_id = number_value(raw.get("cardInstanceId") or raw.get("cardId"))
    type_arg = number_value(raw.get("typeArg"))
    if card_id <= 0 or type_arg <= 0:
        return None
    return {
        "cardId": card_id,
        "typeArg": type_arg,
        "remainingCubes": max(0, number_value(raw.get("remainingCubes"))),
        "isSpirit": bool_value(raw.get("isSpirit")),
    }


def visible_cards(cards: object) -> list[dict[str, object]]:
    return sorted(
        (card for raw in as_list(cards) if (card := visible_card(raw)) is not None),
        key=lambda card: int(card["cardId"]),
    )


def visible_cells(cells: object) -> list[dict[str, object]]:
    result: list[dict[str, object]] = []
    for raw_cell in as_list(cells):
        cell = as_object(raw_cell)
        coord = as_object(cell.get("coord"))
        result.append(
            {
                "coord": {"col": number_value(coord.get("col")), "row": number_value(coord.get("row"))},
                "stack": {
                    "tokens": [
                        string_value(token)
                        for token in as_list(as_object(cell.get("stack")).get("tokens"))
                        if string_value(token)
                    ]
                },
                "lockedByCube": bool_value(cell.get("lockedByCube")),
            }
        )
    return sorted(
        result,
        key=lambda cell: (
            int(as_object(cell["coord"]).get("row") or 0),
            int(as_object(cell["coord"]).get("col") or 0),
        ),
    )


def perspective_player_id(visible: dict[str, object], players: list[dict[str, object]]) -> str:
    player_ids = {string_value(player.get("playerId")) for player in players}
    current = string_value(visible.get("currentPlayerId"))
    active = string_value(visible.get("activePlayerId"))
    if current in player_ids:
        return current
    if active in player_ids:
        return active
    return string_value(players[0].get("playerId")) if players else ""


def visible_central_groups(value: object) -> list[list[str]]:
    return [
        [string_value(token) for token in as_list(group) if string_value(token)]
        for group in as_list(value)
    ]


def board_side_from_capture(capture: dict[str, object]) -> str:
    context = as_object(capture.get("context"))
    gamedatas = as_object(capture.get("gamedatas"))
    raw = string_value(context.get("boardSide")) or string_value(gamedatas.get("boardSide"))
    return "sideB" if raw in {"sideB", "SideB"} else "sideA"


def card_point_counts_from_capture(capture: dict[str, object]) -> dict[int, int]:
    counts: dict[int, int] = {}

    def add(card: object) -> None:
        raw = as_object(card)
        card_id = number_value(raw.get("id"))
        points = len(as_list(raw.get("pointLocations")))
        if card_id > 0 and points > 0:
            counts[card_id] = points

    gamedatas = as_object(capture.get("gamedatas"))
    for player in as_object(gamedatas.get("players")).values():
        add_cards = as_object(player)
        for card in as_list(add_cards.get("boardAnimalCards")):
            add(card)
        for card in as_list(add_cards.get("doneAnimalCards")):
            add(card)
    for card in as_list(gamedatas.get("river")):
        add(card)
    for card in as_list(gamedatas.get("spiritsCards")):
        add(card)
    return counts


def active_player_id_from_capture(capture: dict[str, object], ids: list[str]) -> str:
    context = as_object(capture.get("context"))
    gamedatas = as_object(capture.get("gamedatas"))
    gamestate = as_object(gamedatas.get("gamestate"))
    candidates = [
        string_value(context.get("activePlayer")),
        string_value(gamestate.get("active_player")),
        string_value(gamedatas.get("active_player")),
    ]
    for candidate in candidates:
        if candidate in ids:
            return candidate
    return ids[0] if ids else ""


def convert_visible_state(capture: dict[str, object]) -> dict[str, object] | None:
    visible = as_object(capture.get("visibleStateV1"))
    if not visible:
        return None
    reliability = as_object(visible.get("reliability"))
    use_visible_cards = bool_value(reliability.get("domCards"))
    fallback_snapshot = convert_dom_snapshot(capture) if not use_visible_cards else None
    fallback_players = {
        string_value(player.get("playerId")): player
        for player in as_list(as_object(fallback_snapshot).get("players"))
    }
    players: list[dict[str, object]] = []
    choices_by_player = as_object(visible.get("spiritChoicesByPlayerId"))
    for raw_player in as_list(visible.get("players")):
        player = as_object(raw_player)
        player_id = string_value(player.get("playerId"))
        cells = visible_cells(player.get("cells"))
        fallback_player = as_object(fallback_players.get(player_id))
        players.append({
            "playerId": player_id,
            "cells": cells,
            "activeCards": visible_cards(player.get("activeCards"))
            if use_visible_cards
            else as_list(fallback_player.get("activeCards")),
            "spiritCardChoices": visible_cards(choices_by_player.get(player_id))
            if use_visible_cards
            else [],
            "completedCards": visible_cards(player.get("completedCards"))
            if use_visible_cards
            else as_list(fallback_player.get("completedCards")),
            "emptyHexes": empty_hexes(cells),
        })
    if not players:
        return None
    central_token_groups = visible_central_groups(visible.get("centralTokenGroups"))
    return {
        "schemaVersion": 1,
        "perspectivePlayerId": perspective_player_id(visible, players),
        "activePlayerId": string_value(visible.get("activePlayerId")),
        "boardSide": board_side_from_capture(capture),
        "players": players,
        "centralTokenGroups": central_token_groups,
        "riverCards": visible_cards(visible.get("riverCards"))
        if use_visible_cards
        else as_list(as_object(fallback_snapshot).get("riverCards")),
        "bagCounts": infer_bag_counts(capture, players, central_token_groups),
        "cardsCatalogVersion": "visible-state-v1",
    }


def convert(path: Path) -> dict[str, object]:
    data: dict[str, Any] = json.loads(path.read_text(encoding="utf-8"))
    visible_snapshot = convert_visible_state(data)
    if visible_snapshot:
        return visible_snapshot
    return convert_dom_snapshot(data)


def convert_dom_snapshot(data: dict[str, Any]) -> dict[str, object]:
    nodes = read_nodes_from_capture(data)
    players = []
    ids = player_ids(nodes)
    card_point_counts = card_point_counts_from_capture(data)
    catalog_cube_counts = catalog_cube_count_by_type_arg()
    for player_id in ids:
        cells = cells_for_player(nodes, player_id)
        if all(len(as_list(as_object(c.get("stack")).get("tokens"))) == 0 for c in cells):
            cells = cells_from_gamedatas(data, player_id)
            
        active_cards = cards_for_player(
            nodes,
            player_id,
            "hand",
            completed=False,
            card_point_counts=card_point_counts,
            catalog_cube_counts=catalog_cube_counts,
        )
        if not active_cards:
            active_cards = cards_from_gamedatas(data, player_id, "boardAnimalCards")
            
        completed_cards = cards_for_player(
            nodes,
            player_id,
            "done",
            completed=True,
            card_point_counts=card_point_counts,
            catalog_cube_counts=catalog_cube_counts,
        )
        if not completed_cards:
            completed_cards = cards_from_gamedatas(data, player_id, "doneAnimalCards")
            
        spirit_choices = []
        if not active_cards:
            spirit_choices = spirit_choices_from_gamedatas(data)
            
        players.append(
            {
                "playerId": player_id,
                "cells": cells,
                "activeCards": active_cards,
                "spiritCardChoices": spirit_choices,
                "completedCards": completed_cards,
                "emptyHexes": empty_hexes(cells),
            }
        )
    if not players:
        raise SystemExit("no player-table DOM nodes found")
    active_player_id = active_player_id_from_capture(data, ids)
    central_token_groups = central_groups(nodes)
    
    if not any(central_token_groups):
        gamedatas = as_object(data.get("gamedatas", data))
        raw_board_groups = as_object(gamedatas.get("tokensOnCentralBoard", {}))
        central_token_groups = []
        for hole_id in sorted(raw_board_groups.keys(), key=int):
            group_tokens = as_list(raw_board_groups[hole_id])
            tokens = []
            for t in group_tokens:
                color_id = number_value(as_object(t).get("type_arg"))
                color_name = COLOR_BY_CLASS.get(str(color_id))
                if color_name:
                    tokens.append(color_name)
            central_token_groups.append(tokens)
            
    river = river_cards(nodes, card_point_counts, catalog_cube_counts)
    if not river:
        river = river_cards_from_gamedatas(data)
        
    return {
        "schemaVersion": 1,
        "perspectivePlayerId": active_player_id,
        "activePlayerId": active_player_id,
        "boardSide": board_side(nodes),
        "players": players,
        "centralTokenGroups": central_token_groups,
        "riverCards": river,
        "bagCounts": infer_bag_counts(data, players, central_token_groups),
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
