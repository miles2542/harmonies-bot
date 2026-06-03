from __future__ import annotations

import json
import re
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Literal


Severity = Literal["error", "warn", "info"]
SnapshotKind = Literal["raw-bga", "normalized-v1", "unknown"]
CountValue = int | str | None

CELL_KEY_RE = re.compile(r"^cell_(.+)_(-?\d+)_(-?\d+)$")
COMPARE_KEYS = [
    "players",
    "hexes",
    "nonemptyCells",
    "lockedCells",
    "tokens",
    "activeCards",
    "completedCards",
    "riverCards",
    "centralTokenGroups",
]


@dataclass(frozen=True)
class Issue:
    severity: Severity
    message: str

    def to_dict(self) -> dict[str, str]:
        return {"severity": self.severity, "message": self.message}


@dataclass(frozen=True)
class PlayerSummary:
    player_id: str
    cells: int
    nonempty_cells: int
    locked_cells: int
    tokens: int
    active_cards: int
    completed_cards: int
    empty_hexes: int | None

    def to_dict(self) -> dict[str, CountValue]:
        return {
            "playerId": self.player_id,
            "cells": self.cells,
            "nonemptyCells": self.nonempty_cells,
            "lockedCells": self.locked_cells,
            "tokens": self.tokens,
            "activeCards": self.active_cards,
            "completedCards": self.completed_cards,
            "emptyHexes": self.empty_hexes,
        }


@dataclass(frozen=True)
class SnapshotSummary:
    path: str
    kind: SnapshotKind
    counts: dict[str, int]
    metadata: dict[str, str]
    players: list[PlayerSummary]
    issues: list[Issue]

    def to_dict(self) -> dict[str, Any]:
        return {
            "path": self.path,
            "kind": self.kind,
            "counts": self.counts,
            "metadata": self.metadata,
            "players": [player.to_dict() for player in self.players],
            "issues": [issue.to_dict() for issue in self.issues],
        }


@dataclass(frozen=True)
class Comparison:
    raw_path: str
    normalized_path: str
    issues: list[Issue]

    def to_dict(self) -> dict[str, Any]:
        return {
            "rawPath": self.raw_path,
            "normalizedPath": self.normalized_path,
            "issues": [issue.to_dict() for issue in self.issues],
        }


def as_object(value: Any) -> dict[str, Any] | None:
    return value if isinstance(value, dict) else None


def as_list(value: Any) -> list[Any] | None:
    return value if isinstance(value, list) else None


def value_to_string(value: Any) -> str | None:
    if isinstance(value, str):
        return value
    if isinstance(value, int):
        return str(value)
    return None


def list_count(value: Any) -> int:
    return len(as_list(value) or [])


def object_count(value: Any) -> int:
    return len(as_object(value) or {})


def load_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def get_gamedatas(data: Any) -> dict[str, Any] | None:
    root = as_object(data)
    if root is None:
        return None
    return as_object(root.get("gamedatas")) or root


def is_normalized_snapshot(data: Any) -> bool:
    root = as_object(data)
    return root is not None and "schemaVersion" in root and isinstance(root.get("players"), list)


def token_color_is_known(token: Any) -> bool:
    token_object = as_object(token)
    return token_object is not None and token_object.get("type_arg") in {1, 2, 3, 4, 5, 6, 7}


def cell_location(value: Any) -> str | None:
    location = value_to_string(value)
    return location if location and CELL_KEY_RE.match(location) else None


def count_token_cells(tokens_on_board: Any) -> tuple[int, int, list[Issue]]:
    nonempty_cells: set[str] = set()
    token_count = 0
    issues: list[Issue] = []
    tokens_object = as_object(tokens_on_board)
    if tokens_object is not None:
        for cell_key, stack in tokens_object.items():
            stack_items = as_list(stack)
            if stack_items is None:
                issues.append(Issue("warn", f"{cell_key}: tokensOnBoard stack is not array"))
                continue
            if stack_items:
                nonempty_cells.add(str(cell_key))
            token_count += len(stack_items)
            issues.extend(
                Issue("warn", f"{cell_key}: token missing known type_arg")
                for token in stack_items
                if not token_color_is_known(token)
            )
        return len(nonempty_cells), token_count, issues

    tokens_array = as_list(tokens_on_board)
    if tokens_array is None:
        return 0, 0, issues
    for token in tokens_array:
        token_object = as_object(token)
        if token_object is None:
            issues.append(Issue("warn", "tokensOnBoard item is not object"))
            continue
        location = cell_location(token_object.get("location"))
        if location:
            nonempty_cells.add(location)
        token_count += 1
        if not token_color_is_known(token_object):
            issues.append(Issue("warn", "tokensOnBoard item missing known type_arg"))
    return len(nonempty_cells), token_count, issues


def collect_player_locked_cells(player: dict[str, Any]) -> set[str]:
    value = player.get("animalCubesOnBoard")
    locked = {
        location
        for item in as_list(value) or []
        if (location := cell_location(item)) is not None
    }
    locked.update(key for key in (as_object(value) or {}) if cell_location(key) is not None)
    return locked


def collect_global_locked_cells(gamedatas: dict[str, Any]) -> set[str]:
    locked: set[str] = set()
    for cube in as_list(gamedatas.get("cubesOnAnimalCards")) or []:
        cube_object = as_object(cube)
        if cube_object is None:
            continue
        location = cell_location(cube_object.get("location"))
        if location:
            locked.add(location)
    return locked


def count_spirit_cards_for_player(gamedatas: dict[str, Any], player_id: str) -> int:
    return sum(
        1
        for card in as_list(gamedatas.get("spiritsCards")) or []
        if (card_object := as_object(card)) is not None
        and value_to_string(card_object.get("location_arg")) == player_id
    )


def raw_player_summary(
    player_id: str,
    player: dict[str, Any],
    gamedatas: dict[str, Any],
    hex_count: int,
) -> tuple[PlayerSummary, list[Issue]]:
    nonempty_cells, tokens, issues = count_token_cells(player.get("tokensOnBoard"))
    active_cards = list_count(player.get("boardAnimalCards")) + count_spirit_cards_for_player(
        gamedatas,
        player_id,
    )
    empty_hexes = player.get("emptyHexes")
    summary = PlayerSummary(
        player_id=player_id,
        cells=hex_count,
        nonempty_cells=nonempty_cells,
        locked_cells=len(collect_player_locked_cells(player)),
        tokens=tokens,
        active_cards=active_cards,
        completed_cards=list_count(player.get("doneAnimalCards")),
        empty_hexes=empty_hexes if isinstance(empty_hexes, int) else None,
    )
    return summary, issues


def summarize_raw_snapshot(path: Path, data: Any) -> SnapshotSummary:
    gamedatas = get_gamedatas(data)
    if gamedatas is None:
        issue = Issue("error", "JSON root is not object")
        return SnapshotSummary(str(path), "unknown", {}, {}, [], [issue])

    issues: list[Issue] = []
    players_object = as_object(gamedatas.get("players"))
    if players_object is None:
        issues.append(Issue("error", "players missing or not object"))
        players_object = {}
    hexes = as_list(gamedatas.get("hexes"))
    if hexes is None:
        issues.append(Issue("error", "hexes missing or not array"))
        hex_count = 0
    else:
        hex_count = sum(1 for item in hexes if as_object(item) is not None)

    gamestate = as_object(gamedatas.get("gamestate")) or {}
    metadata = {
        key: value
        for key, value in {
            "activePlayerId": value_to_string(gamestate.get("active_player")),
            "boardSide": value_to_string(gamedatas.get("boardSide")),
        }.items()
        if value is not None
    }
    players: list[PlayerSummary] = []
    player_locked_union: set[str] = set()
    for player_id, player_value in sorted(players_object.items()):
        player = as_object(player_value)
        if player is None:
            issues.append(Issue("warn", f"{player_id}: player value is not object"))
            continue
        summary, player_issues = raw_player_summary(str(player_id), player, gamedatas, hex_count)
        issues += [Issue(i.severity, f"{player_id}: {i.message}") for i in player_issues]
        player_locked_union.update(collect_player_locked_cells(player))
        players.append(summary)

    counts = {
        "players": len(players_object),
        "hexes": hex_count,
        "nonemptyCells": sum(player.nonempty_cells for player in players),
        "lockedCells": len(collect_global_locked_cells(gamedatas) | player_locked_union),
        "tokens": sum(player.tokens for player in players),
        "activeCards": sum(player.active_cards for player in players),
        "completedCards": sum(player.completed_cards for player in players),
        "riverCards": list_count(gamedatas.get("river")),
        "centralTokenGroups": object_count(gamedatas.get("tokensOnCentralBoard")),
    }
    return SnapshotSummary(str(path), "raw-bga", counts, metadata, players, issues)


def summarize_norm_player(index: int, player: dict[str, Any], issues: list[Issue]) -> PlayerSummary:
    cells = as_list(player.get("cells")) or []
    nonempty_cells = 0
    locked_cells = 0
    token_count = 0
    player_id = value_to_string(player.get("playerId")) or f"players[{index}]"
    for cell_index, cell_value in enumerate(cells):
        cell = as_object(cell_value)
        if cell is None:
            issues.append(Issue("warn", f"{player_id}: cells[{cell_index}] is not object"))
            continue
        tokens = as_list((as_object(cell.get("stack")) or {}).get("tokens")) or []
        nonempty_cells += int(bool(tokens))
        token_count += len(tokens)
        locked_cells += int(cell.get("lockedByCube") is True)
    empty_hexes = player.get("emptyHexes")
    return PlayerSummary(
        player_id=player_id,
        cells=len(cells),
        nonempty_cells=nonempty_cells,
        locked_cells=locked_cells,
        tokens=token_count,
        active_cards=list_count(player.get("activeCards")),
        completed_cards=list_count(player.get("completedCards")),
        empty_hexes=empty_hexes if isinstance(empty_hexes, int) else None,
    )


def summarize_normalized_snapshot(path: Path, data: Any) -> SnapshotSummary:
    root = as_object(data)
    if root is None:
        issue = Issue("error", "JSON root is not object")
        return SnapshotSummary(str(path), "unknown", {}, {}, [], [issue])
    issues: list[Issue] = []
    players_array = as_list(root.get("players"))
    if players_array is None:
        issues.append(Issue("error", "players missing or not array"))
        players_array = []
    metadata = {
        key: value
        for key, value in {
            "schemaVersion": value_to_string(root.get("schemaVersion")),
            "activePlayerId": value_to_string(root.get("activePlayerId")),
            "perspectivePlayerId": value_to_string(root.get("perspectivePlayerId")),
            "boardSide": value_to_string(root.get("boardSide")),
            "cardsCatalogVersion": value_to_string(root.get("cardsCatalogVersion")),
        }.items()
        if value is not None
    }
    players = [
        summarize_norm_player(index, player, issues)
        for index, player_value in enumerate(players_array)
        if (player := as_object(player_value)) is not None
    ]
    if len(players) != len(players_array):
        issues.append(Issue("warn", "one or more normalized players are not objects"))
    cell_counts = [player.cells for player in players]
    if len(set(cell_counts)) > 1:
        issues.append(Issue("warn", f"player cell counts differ: {sorted(set(cell_counts))}"))
    counts = {
        "players": len(players),
        "hexes": max(cell_counts, default=0),
        "nonemptyCells": sum(player.nonempty_cells for player in players),
        "lockedCells": sum(player.locked_cells for player in players),
        "tokens": sum(player.tokens for player in players),
        "activeCards": sum(player.active_cards for player in players),
        "completedCards": sum(player.completed_cards for player in players),
        "riverCards": list_count(root.get("riverCards")),
        "centralTokenGroups": list_count(root.get("centralTokenGroups")),
    }
    return SnapshotSummary(str(path), "normalized-v1", counts, metadata, players, issues)


def summarize_file(path: Path) -> SnapshotSummary:
    data = load_json(path)
    if is_normalized_snapshot(data):
        return summarize_normalized_snapshot(path, data)
    return summarize_raw_snapshot(path, data)


def compare_summaries(raw: SnapshotSummary, norm: SnapshotSummary) -> Comparison:
    issues: list[Issue] = []
    if raw.kind != "raw-bga":
        issues.append(Issue("error", f"{raw.path}: expected raw-bga, got {raw.kind}"))
    if norm.kind != "normalized-v1":
        issues.append(Issue("error", f"{norm.path}: expected normalized-v1, got {norm.kind}"))
    for key in COMPARE_KEYS:
        if raw.counts.get(key) != norm.counts.get(key):
            msg = f"{key} mismatch: raw={raw.counts.get(key)} norm={norm.counts.get(key)}"
            issues.append(Issue("error", msg))
    raw_players = {player.player_id for player in raw.players}
    normalized_players = {player.player_id for player in norm.players}
    if missing := sorted(raw_players - normalized_players):
        issues.append(Issue("warn", f"players missing after normalize: {', '.join(missing)}"))
    if extra := sorted(normalized_players - raw_players):
        issues.append(Issue("warn", f"extra normalized players: {', '.join(extra)}"))
    return Comparison(raw.path, norm.path, issues)


def auto_comparisons(summaries: list[SnapshotSummary]) -> list[Comparison]:
    raw = [summary for summary in summaries if summary.kind == "raw-bga"]
    norm = [summary for summary in summaries if summary.kind == "normalized-v1"]
    return [compare_summaries(raw[0], norm[0])] if len(raw) == len(norm) == 1 else []
