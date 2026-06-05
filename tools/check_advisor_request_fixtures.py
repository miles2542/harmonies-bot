from __future__ import annotations

import argparse
import json
import re
from pathlib import Path
from typing import Any

DEFAULT_FIXTURES = sorted(Path("fixtures/advisor_requests").glob("*.json"))
PLAYER_ID_RE = re.compile(r"^player_\d+$")
FORBIDDEN_KEYS = {"name", "player_name", "avatar", "flag"}
BAG_COLORS = {"water", "mountain", "trunk", "foliage", "field", "building", "unknown"}


def as_object(value: object) -> dict[str, Any]:
    return value if isinstance(value, dict) else {}


def as_list(value: object) -> list[Any]:
    return value if isinstance(value, list) else []


def load_fixture(path: Path) -> dict[str, Any]:
    data = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(data, dict):
        raise ValueError("root must be object")
    snapshot = data.get("snapshot")
    if not isinstance(snapshot, dict):
        raise ValueError("missing snapshot object")
    return data


def validate_fixture(path: Path) -> list[str]:
    issues: list[str] = []
    try:
        data = load_fixture(path)
    except (json.JSONDecodeError, ValueError) as error:
        return [str(error)]
    snapshot = as_object(data.get("snapshot"))
    player_ids = [str(player.get("playerId")) for player in as_list(snapshot.get("players"))]
    validate_player_ids(snapshot, player_ids, issues)
    validate_bag_counts(snapshot, issues)
    validate_shape(snapshot, player_ids, issues)
    validate_no_forbidden_keys(data, issues)
    return issues


def validate_player_ids(snapshot: dict[str, Any], player_ids: list[str], issues: list[str]) -> None:
    if not player_ids:
        issues.append("snapshot.players empty")
    for label in ("perspectivePlayerId", "activePlayerId"):
        value = str(snapshot.get(label) or "")
        if not PLAYER_ID_RE.fullmatch(value):
            issues.append(f"{label} not anonymized: {value}")
        if player_ids and value not in player_ids:
            issues.append(f"{label} not in players: {value}")
    for player_id in player_ids:
        if not PLAYER_ID_RE.fullmatch(player_id):
            issues.append(f"playerId not anonymized: {player_id}")


def validate_bag_counts(snapshot: dict[str, Any], issues: list[str]) -> None:
    counts = as_object(snapshot.get("bagCounts"))
    if set(counts) != BAG_COLORS:
        issues.append(f"bagCounts keys invalid: {sorted(counts)}")
        return
    total = 0
    for key, value in counts.items():
        if not isinstance(value, int) or value < 0:
            issues.append(f"bagCounts.{key} invalid: {value}")
        else:
            total += value
    if total <= 0:
        issues.append("bagCounts total is zero; future search fixture cannot refill")


def validate_shape(snapshot: dict[str, Any], player_ids: list[str], issues: list[str]) -> None:
    if snapshot.get("schemaVersion") != 1:
        issues.append(f"schemaVersion invalid: {snapshot.get('schemaVersion')}")
    if snapshot.get("boardSide") not in {"sideA", "sideB"}:
        issues.append(f"boardSide invalid: {snapshot.get('boardSide')}")
    groups = as_list(snapshot.get("centralTokenGroups"))
    if len(groups) != 5 or any(len(as_list(group)) != 3 for group in groups):
        issues.append("centralTokenGroups must be five groups of three tokens")
    for player in as_list(snapshot.get("players")):
        raw_player = as_object(player)
        player_id = str(raw_player.get("playerId"))
        cells = as_list(raw_player.get("cells"))
        if player_id in player_ids and not cells:
            issues.append(f"{player_id} has no cells")
        if len(as_list(raw_player.get("activeCards"))) > 4:
            issues.append(f"{player_id} has more than 4 active cards")
    if len(as_list(snapshot.get("riverCards"))) > 5:
        issues.append("riverCards has more than 5 cards")


def validate_no_forbidden_keys(value: Any, issues: list[str], path: str = "$") -> None:
    if isinstance(value, dict):
        for key, child in value.items():
            child_path = f"{path}.{key}"
            if key in FORBIDDEN_KEYS:
                issues.append(f"forbidden key present: {child_path}")
            validate_no_forbidden_keys(child, issues, child_path)
    elif isinstance(value, list):
        for index, child in enumerate(value):
            validate_no_forbidden_keys(child, issues, f"{path}[{index}]")


def main() -> int:
    parser = argparse.ArgumentParser(description="Validate tracked AdvisorRequestV1 fixtures.")
    parser.add_argument("fixtures", nargs="*", type=Path)
    args = parser.parse_args()

    paths = args.fixtures or DEFAULT_FIXTURES
    report = []
    ok = True
    for path in paths:
        issues = validate_fixture(path)
        ok = ok and not issues
        report.append({"fixture": str(path), "issues": issues})
    print(json.dumps({"ok": ok, "fixtures": report}, indent=2))
    return 0 if ok else 1


if __name__ == "__main__":
    raise SystemExit(main())
