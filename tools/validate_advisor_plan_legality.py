from __future__ import annotations

import argparse
import json
import subprocess
from copy import deepcopy
from pathlib import Path
from typing import Any

DEFAULT_REQUESTS = [
    Path("fixtures/advisor_requests/sidea_2p_nature_match12_early_spirit_choice_request.json"),
    Path("fixtures/advisor_requests/sidea_2p_nature_match12_late_active_turn_request.json"),
]


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


def validate_response(request: dict[str, Any], response: dict[str, Any]) -> list[str]:
    snapshot = request["snapshot"]
    player_id = snapshot["perspectivePlayerId"]
    player = next(player for player in snapshot["players"] if player["playerId"] == player_id)
    issues: list[str] = []
    for move_index, move in enumerate(response.get("bestMoves", []), start=1):
        issues.extend(validate_move(snapshot, player, move, move_index))
    return issues


def validate_move(
    snapshot: dict[str, Any],
    player: dict[str, Any],
    move: dict[str, Any],
    move_index: int,
) -> list[str]:
    issues: list[str] = []
    active_cards = {int(card["cardId"]): deepcopy(card) for card in player.get("activeCards", [])}
    completed_cards = {int(card["cardId"]) for card in player.get("completedCards", [])}
    spirit_choices = {
        int(card["cardId"]): deepcopy(card) for card in player.get("spiritCardChoices", [])
    }
    river_cards = {int(card["cardId"]): deepcopy(card) for card in snapshot.get("riverCards", [])}
    locked = {
        (int(cell["coord"]["col"]), int(cell["coord"]["row"]))
        for cell in player.get("cells", [])
        if cell.get("lockedByCube")
    }
    drafted = False

    for action_index, action in enumerate(move.get("orderedActions", []), start=1):
        kind = action.get("kind")
        prefix = f"move {move_index} action {action_index} {kind}"
        if kind == "chooseSpirit":
            card_id = int(action["cardId"])
            if card_id not in spirit_choices:
                issues.append(f"{prefix}: spirit card not offered to player: {card_id}")
                continue
            active_cards[card_id] = spirit_choices.pop(card_id)
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
                active_cards[card_id] = river_cards[card_id]
            drafted = True
            continue
        if kind != "settleCard":
            continue

        card_id = int(action["cardId"])
        coord = (int(action["col"]), int(action["row"]))
        if card_id in completed_cards:
            issues.append(f"{prefix}: completed card settled: {card_id}")
        if card_id not in active_cards:
            river_note = " river" if card_id in river_cards else ""
            issues.append(f"{prefix}: card not active{river_note}: {card_id}")
            continue
        remaining = int(active_cards[card_id].get("remainingCubes", 0))
        if remaining <= 0:
            issues.append(f"{prefix}: no cubes remaining: {card_id}")
            continue
        if coord in locked:
            issues.append(f"{prefix}: target already locked: {coord}")
            continue
        active_cards[card_id]["remainingCubes"] = remaining - 1
        locked.add(coord)
        if remaining - 1 == 0:
            completed_cards.add(card_id)
            active_cards.pop(card_id, None)

    return issues


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Validate advisor output uses only legally available cards."
    )
    parser.add_argument("requests", nargs="*", type=Path, default=DEFAULT_REQUESTS)
    args = parser.parse_args()

    report: list[dict[str, Any]] = []
    ok = True
    for request_path in args.requests:
        request = load_request(request_path)
        response = run_advisor(request_path)
        issues = validate_response(request, response)
        ok = ok and not issues
        report.append(
            {
                "request": str(request_path),
                "status": response.get("status"),
                "bestMoves": len(response.get("bestMoves", [])),
                "issues": issues,
            }
        )
    print(json.dumps({"ok": ok, "cases": report}, indent=2))
    return 0 if ok else 1


if __name__ == "__main__":
    raise SystemExit(main())
