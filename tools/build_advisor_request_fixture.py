from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any

try:
    from tools.dom_capture_to_snapshot import convert as convert_capture
except ModuleNotFoundError:
    from dom_capture_to_snapshot import convert as convert_capture


def as_object(value: object) -> dict[str, Any]:
    return value if isinstance(value, dict) else {}


def is_capture(data: dict[str, Any]) -> bool:
    return isinstance(data.get("visibleStateV1"), dict) or isinstance(data.get("domSnapshot"), dict)


def load_snapshot(path: Path) -> dict[str, Any]:
    data = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(data, dict):
        raise SystemExit(f"{path} must contain a JSON object")
    if is_capture(data):
        return convert_capture(path)
    if isinstance(data.get("snapshot"), dict):
        return as_object(data["snapshot"])
    if data.get("schemaVersion") == 1:
        return data
    raise SystemExit(f"{path} is not capture, AdvisorRequestV1, or GameSnapshotV1")


def anonymize_snapshot(snapshot: dict[str, Any]) -> dict[str, Any]:
    player_map = player_id_map(snapshot)
    return rewrite_value(snapshot, player_map)


def player_id_map(snapshot: dict[str, Any]) -> dict[str, str]:
    ordered: list[str] = []
    for key in ("perspectivePlayerId", "activePlayerId"):
        add_unique(ordered, snapshot.get(key))
    for player in snapshot.get("players", []):
        add_unique(ordered, as_object(player).get("playerId"))
    return {player_id: f"player_{index}" for index, player_id in enumerate(ordered, start=1)}


def add_unique(items: list[str], value: object) -> None:
    text = str(value) if value not in (None, "") else ""
    if text and text not in items:
        items.append(text)


def rewrite_value(value: Any, player_map: dict[str, str]) -> Any:
    if isinstance(value, dict):
        return rewrite_dict(value, player_map)
    if isinstance(value, list):
        return [rewrite_value(item, player_map) for item in value]
    if isinstance(value, str):
        return player_map.get(value, value)
    return value


def rewrite_dict(data: dict[str, Any], player_map: dict[str, str]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in data.items():
        if key in {"name", "player_name", "avatar", "flag"}:
            continue
        result[key] = rewrite_value(value, player_map)
    return result


def build_request(
    snapshot_path: Path,
    time_budget_ms: int,
    max_results: int,
    seed: int,
    runtime_mode: str,
) -> dict[str, Any]:
    return {
        "snapshot": anonymize_snapshot(load_snapshot(snapshot_path)),
        "timeBudgetMs": time_budget_ms,
        "maxResults": max_results,
        "seed": seed,
        "runtimeMode": runtime_mode,
    }


def main() -> int:
    parser = argparse.ArgumentParser(description="Build anonymized AdvisorRequestV1 fixture.")
    parser.add_argument("input", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument("--time-budget-ms", type=int, default=1500)
    parser.add_argument("--max-results", type=int, default=3)
    parser.add_argument("--seed", type=int, default=1)
    parser.add_argument("--runtime-mode", default="fixture")
    args = parser.parse_args()

    request = build_request(
        args.input,
        args.time_budget_ms,
        args.max_results,
        args.seed,
        args.runtime_mode,
    )
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(request, indent=2) + "\n", encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
