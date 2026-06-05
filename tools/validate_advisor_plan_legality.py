from __future__ import annotations

import argparse
import json
import subprocess
import tempfile
from collections import Counter
from copy import deepcopy
from pathlib import Path
from typing import Any, TypeAlias

try:
    from tools.dom_capture_to_snapshot import convert as convert_dom_capture
except ModuleNotFoundError:
    from dom_capture_to_snapshot import convert as convert_dom_capture

DEFAULT_REQUESTS = [
    Path("fixtures/advisor_requests/sidea_2p_nature_match12_early_spirit_choice_request.json"),
    Path("fixtures/advisor_requests/sidea_2p_nature_match12_late_active_turn_request.json"),
    Path("fixtures/advisor_requests/sidea_2p_nature_match14_full_hand_request.json"),
    Path("fixtures/advisor_requests/sidea_2p_nature_match14_after_completion_near_end_request.json"),
]
CARD_CATALOG_PATH = Path("docs/cards_database.json")
DIRECTIONS = 6
ColorName: TypeAlias = str
Coord: TypeAlias = tuple[int, int]
CellState: TypeAlias = dict[str, Any]

COLOR_FROM_TYPE_ARG = {
    1: "water",
    2: "mountain",
    3: "trunk",
    4: "foliage",
    5: "field",
    6: "building",
    7: "building",
}


def run_advisor(request_path: Path) -> dict[str, Any]:
    result = subprocess.run(
        [
            "cargo",
            "run",
            "-q",
            "-p",
            "harmonies-cli",
            "--",
            str(request_path),
            "docs/cards_database.json",
            "docs/weights.baseline.json",
        ],
        check=True,
        capture_output=True,
        text=True,
    )
    return json.loads(result.stdout)


def load_request(path: Path) -> dict[str, Any]:
    return json.loads(path.read_text(encoding="utf-8"))


def load_card_catalog(path: Path = CARD_CATALOG_PATH) -> dict[int, dict[str, Any]]:
    raw: dict[str, dict[str, Any]] = json.loads(path.read_text(encoding="utf-8"))
    return {int(card["type_arg"]): card for card in raw.values()}


def request_from_capture(path: Path) -> dict[str, Any]:
    return {
        "snapshot": convert_dom_capture(path),
        "timeBudgetMs": 1500,
        "maxResults": 3,
        "seed": 1,
        "runtimeMode": "capture-legality",
    }


def write_temp_request(tmp_dir: Path, capture_path: Path, request: dict[str, Any]) -> Path:
    request_path = tmp_dir / f"{capture_path.stem}_request.json"
    request_path.write_text(json.dumps(request, indent=2) + "\n", encoding="utf-8")
    return request_path


def coord_from_action(action: dict[str, Any]) -> Coord:
    return (int(action["col"]), int(action["row"]))


def coord_from_cell(cell: dict[str, Any]) -> Coord:
    coord = cell["coord"]
    return (int(coord["col"]), int(coord["row"]))


def build_cells(player: dict[str, Any]) -> dict[Coord, CellState]:
    cells: dict[Coord, CellState] = {}
    for cell in player.get("cells", []):
        cells[coord_from_cell(cell)] = {
            "stack": list(cell.get("stack", {}).get("tokens", [])),
            "locked": bool(cell.get("lockedByCube")),
        }
    return cells


def placement_error(cell: CellState, color: ColorName) -> str | None:
    stack: list[ColorName] = cell["stack"]
    if cell["locked"]:
        return "cell locked by cube"
    if len(stack) >= 3:
        return "stack height already 3"
    if color == "mountain" and all(token == "mountain" for token in stack):
        return None
    if color == "trunk" and len(stack) < 2 and all(token == "trunk" for token in stack):
        return None
    if color == "foliage" and (not stack or all(token == "trunk" for token in stack)):
        return None
    if color == "building" and (not stack or stack[-1] in {"trunk", "mountain", "building"}):
        return None
    if color in {"field", "water"} and not stack:
        return None
    return "token cannot be placed on this stack"


def neighbor(coord: Coord, direction: int) -> Coord:
    col, row = coord
    even = col % 2 == 0
    offsets = {
        (True, 0): (1, 0),
        (True, 1): (1, -1),
        (True, 2): (0, -1),
        (True, 3): (-1, -1),
        (True, 4): (-1, 0),
        (True, 5): (0, 1),
        (False, 0): (1, 1),
        (False, 1): (1, 0),
        (False, 2): (0, -1),
        (False, 3): (-1, 0),
        (False, 4): (-1, 1),
        (False, 5): (0, 1),
    }
    dc, dr = offsets[(even, direction)]
    return (col + dc, row + dr)


def pattern_cells(origin: Coord, pattern: list[dict[str, Any]], rotation: int) -> list[Coord]:
    coords: list[Coord] = []
    current = origin
    for index, step in enumerate(pattern):
        direction = (int(step["position"]) + rotation) % DIRECTIONS
        if index == 0:
            current = origin
        else:
            current = neighbor(current, direction)
        coords.append(current)
    return coords


def stack_matches_colors(stack: list[ColorName], colors: list[int]) -> bool:
    if colors == [6, 7]:
        return bool(stack) and stack[-1] == "building"
    expected = [COLOR_FROM_TYPE_ARG.get(raw) for raw in reversed(colors)]
    return all(color is not None for color in expected) and expected == stack


def pattern_allows_target(
    cells: dict[Coord, CellState],
    definition: dict[str, Any],
    target: Coord,
) -> bool:
    pattern = list(definition.get("pattern", []))
    for origin in cells:
        for rotation in range(DIRECTIONS):
            coords = pattern_cells(origin, pattern, rotation)
            if not all(
                coord in cells
                and stack_matches_colors(cells[coord]["stack"], list(step.get("colors", [])))
                for coord, step in zip(coords, pattern, strict=True)
            ):
                continue
            cube_steps = zip(coords, pattern, strict=True)
            if any(bool(step.get("allowCube")) and coord == target for coord, step in cube_steps):
                return True
    return False


def validate_response(request: dict[str, Any], response: dict[str, Any]) -> list[str]:
    snapshot = request["snapshot"]
    player_id = snapshot["perspectivePlayerId"]
    player = next(player for player in snapshot["players"] if player["playerId"] == player_id)
    catalog = load_card_catalog()
    issues: list[str] = []
    for move_index, move in enumerate(response.get("bestMoves", []), start=1):
        issues.extend(validate_move(snapshot, player, move, move_index, catalog))
    return issues


def validate_move(
    snapshot: dict[str, Any],
    player: dict[str, Any],
    move: dict[str, Any],
    move_index: int,
    catalog: dict[int, dict[str, Any]],
) -> list[str]:
    issues: list[str] = []
    cells = build_cells(player)
    active_cards = {int(card["cardId"]): deepcopy(card) for card in player.get("activeCards", [])}
    completed_cards = {int(card["cardId"]) for card in player.get("completedCards", [])}
    spirit_choices = {
        int(card["cardId"]): deepcopy(card) for card in player.get("spiritCardChoices", [])
    }
    river_cards = {int(card["cardId"]): deepcopy(card) for card in snapshot.get("riverCards", [])}
    drafted = False
    selected_tokens: Counter[ColorName] | None = None
    central_groups = list(snapshot.get("centralTokenGroups", []))
    move_group_index = int(move.get("centralGroupIndex", -1))
    if move_group_index < 0 or move_group_index >= len(central_groups):
        issues.append(
            f"move {move_index}: centralGroupIndex missing from snapshot: {move_group_index}"
        )
        expected_tokens: Counter[ColorName] | None = None
    else:
        expected_tokens = Counter(central_groups[move_group_index])

    for action_index, action in enumerate(move.get("orderedActions", []), start=1):
        kind = action.get("kind")
        prefix = f"move {move_index} action {action_index} {kind}"
        if kind == "chooseSpirit":
            card_id = int(action["cardId"])
            if card_id not in spirit_choices:
                issues.append(f"{prefix}: spirit card not offered to player: {card_id}")
                continue
            card = spirit_choices[card_id]
            if int(action.get("typeArg", card["typeArg"])) != int(card["typeArg"]):
                issues.append(f"{prefix}: typeArg mismatch for spirit card: {card_id}")
            spirit_choices.clear()
            active_cards[card_id] = card
            continue
        if kind == "takeGroup":
            action_group_index = int(action.get("groupIndex", -1))
            action_tokens = Counter(action.get("tokens", []))
            if selected_tokens is not None:
                issues.append(f"{prefix}: second takeGroup in same turn")
                continue
            if action_group_index != move_group_index:
                issues.append(
                    f"{prefix}: groupIndex {action_group_index} "
                    f"!= centralGroupIndex {move_group_index}"
                )
            if expected_tokens is None:
                issues.append(f"{prefix}: central group missing: {action_group_index}")
                continue
            if action_tokens != expected_tokens:
                issues.append(
                    f"{prefix}: tokens {dict(action_tokens)} != snapshot {dict(expected_tokens)}"
                )
                continue
            selected_tokens = action_tokens
            continue
        if kind == "placeToken":
            token = str(action["token"])
            coord = coord_from_action(action)
            if selected_tokens is None:
                issues.append(f"{prefix}: place before takeGroup: {token}")
                continue
            if selected_tokens[token] <= 0:
                issues.append(f"{prefix}: token not available from selected group: {token}")
                continue
            cell = cells.get(coord)
            if cell is None:
                issues.append(f"{prefix}: target cell missing from player board: {coord}")
                continue
            error = placement_error(cell, token)
            if error:
                issues.append(f"{prefix}: {error}: {coord} {token} on {cell['stack']}")
                continue
            cell["stack"].append(token)
            selected_tokens[token] -= 1
            continue
        if kind == "draftCard":
            card_id = int(action["cardId"])
            if drafted:
                issues.append(f"{prefix}: second draft in same turn: {card_id}")
            if len(active_cards) >= 4:
                issues.append(f"{prefix}: hand full before draft: {card_id}")
            if card_id not in river_cards:
                issues.append(f"{prefix}: drafted card not in river: {card_id}")
            else:
                card = river_cards.pop(card_id)
                if int(action.get("typeArg", card["typeArg"])) != int(card["typeArg"]):
                    issues.append(f"{prefix}: typeArg mismatch for river card: {card_id}")
                active_cards[card_id] = card
            drafted = True
            continue
        if kind != "settleCard":
            issues.append(f"{prefix}: unknown action kind")
            continue

        card_id = int(action["cardId"])
        action_type_arg = int(action["typeArg"])
        coord = coord_from_action(action)
        if card_id in completed_cards:
            issues.append(f"{prefix}: completed card settled: {card_id}")
        if card_id not in active_cards:
            river_note = " river" if card_id in river_cards else ""
            issues.append(f"{prefix}: card not active{river_note}: {card_id}")
            continue
        card = active_cards[card_id]
        if action_type_arg != int(card["typeArg"]):
            issues.append(f"{prefix}: typeArg mismatch for active card: {card_id}")
            continue
        remaining = int(active_cards[card_id].get("remainingCubes", 0))
        if remaining <= 0:
            issues.append(f"{prefix}: no cubes remaining: {card_id}")
            continue
        cell = cells.get(coord)
        if cell is None:
            issues.append(f"{prefix}: target cell missing from player board: {coord}")
            continue
        if not cell["stack"]:
            issues.append(f"{prefix}: target cell has no stack: {coord}")
            continue
        if cell["locked"]:
            issues.append(f"{prefix}: target already locked: {coord}")
            continue
        definition = catalog.get(action_type_arg)
        if definition is None:
            issues.append(f"{prefix}: card typeArg missing from catalog: {action_type_arg}")
            continue
        if not pattern_allows_target(cells, definition, coord):
            issues.append(f"{prefix}: no matching pattern for typeArg {action_type_arg}: {coord}")
            continue
        active_cards[card_id]["remainingCubes"] = remaining - 1
        cell["locked"] = True
        if remaining - 1 == 0:
            completed_cards.add(card_id)
            active_cards.pop(card_id, None)

    if selected_tokens is None:
        issues.append(f"move {move_index}: missing takeGroup action")
    elif not issues:
        unplaced = +selected_tokens
        if unplaced:
            issues.append(
                f"move {move_index}: selected group tokens not all placed: {dict(unplaced)}"
            )

    return issues


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Validate advisor output uses only legally available cards."
    )
    parser.add_argument("requests", nargs="*", type=Path)
    parser.add_argument("--capture", action="append", type=Path, default=[])
    args = parser.parse_args()

    report: list[dict[str, Any]] = []
    ok = True
    request_paths = args.requests or ([] if args.capture else DEFAULT_REQUESTS)
    with tempfile.TemporaryDirectory(prefix="harmonies_capture_requests_") as tmp:
        tmp_dir = Path(tmp)
        cases: list[tuple[str, Path, dict[str, Any]]] = []
        for request_path in request_paths:
            cases.append((str(request_path), request_path, load_request(request_path)))
        for capture_path in args.capture:
            request = request_from_capture(capture_path)
            cases.append(
                (
                    str(capture_path),
                    write_temp_request(tmp_dir, capture_path, request),
                    request,
                )
            )
        for label, request_path, request in cases:
            response = run_advisor(request_path)
            issues = validate_response(request, response)
            ok = ok and not issues
            report.append(
                {
                    "request": label,
                    "status": response.get("status"),
                    "bestMoves": len(response.get("bestMoves", [])),
                    "issues": issues,
                }
            )
    print(json.dumps({"ok": ok, "cases": report}, indent=2))
    return 0 if ok else 1


if __name__ == "__main__":
    raise SystemExit(main())
