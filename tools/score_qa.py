from __future__ import annotations

import argparse
import json
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Any


@dataclass(frozen=True)
class ExpectedScore:
    player_id: str
    total: int


def parse_expected(value: str) -> ExpectedScore:
    if "=" not in value:
        raise argparse.ArgumentTypeError("expected score must look like player_id=42")
    player_id, raw_total = value.split("=", 1)
    if not player_id:
        raise argparse.ArgumentTypeError("player id must not be empty")
    try:
        total = int(raw_total)
    except ValueError as error:
        raise argparse.ArgumentTypeError("score must be an integer") from error
    return ExpectedScore(player_id=player_id, total=total)


def run_score(snapshot: Path, perspective: str | None, catalog: Path) -> dict[str, Any]:
    command = ["cargo", "run", "-q", "-p", "harmonies-cli", "--", "score", str(snapshot)]
    if perspective:
        command.extend(["--perspective", perspective])
    command.extend(["--catalog", str(catalog)])
    completed = subprocess.run(command, check=True, capture_output=True, text=True)
    return json.loads(completed.stdout)


def load_capture_expected(snapshot: Path) -> list[ExpectedScore]:
    with snapshot.open("r", encoding="utf-8") as file:
        data = json.load(file)
    expected = []
    for hint in data.get("scoreHints", []):
        player_id = str(hint.get("playerId", ""))
        total = hint.get("total")
        if player_id and isinstance(total, int):
            expected.append(ExpectedScore(player_id=player_id, total=total))
    return expected


def player_totals(report: dict[str, Any]) -> dict[str, int]:
    totals: dict[str, int] = {}
    for player in report.get("players", []):
        player_id = str(player.get("playerId", ""))
        total = player.get("total")
        if player_id and isinstance(total, int):
            totals[player_id] = total
    return totals


def compare_scores(report: dict[str, Any], expected: list[ExpectedScore]) -> dict[str, Any]:
    totals = player_totals(report)
    checks = []
    ok = True
    for item in expected:
        actual = totals.get(item.player_id)
        matched = actual == item.total
        ok = ok and matched
        checks.append(
            {
                "playerId": item.player_id,
                "expected": item.total,
                "actual": actual,
                "ok": matched,
            }
        )
    return {
        "ok": ok,
        "boardSide": report.get("boardSide"),
        "perspectivePlayerId": report.get("perspectivePlayerId"),
        "activePlayerId": report.get("activePlayerId"),
        "checks": checks,
        "players": report.get("players", []),
    }


def main() -> None:
    parser = argparse.ArgumentParser(description="Compare harmonies scorer totals to BGA scores.")
    parser.add_argument("snapshot", type=Path)
    parser.add_argument("--expected", action="append", type=parse_expected, default=[])
    parser.add_argument("--use-capture-scores", action="store_true")
    parser.add_argument("--perspective")
    parser.add_argument("--catalog", type=Path, default=Path("docs/cards_database.json"))
    parser.add_argument("--report", type=Path)
    args = parser.parse_args()

    expected = list(args.expected)
    if args.use_capture_scores:
        expected.extend(load_capture_expected(args.snapshot))
    if not expected:
        raise SystemExit("no expected scores provided or found in capture scoreHints")
    report = run_score(args.snapshot, args.perspective, args.catalog)
    comparison = compare_scores(report, expected)
    output = json.dumps(comparison, indent=2)
    print(output)
    if args.report:
        args.report.write_text(output + "\n", encoding="utf-8")
    if not comparison["ok"]:
        sys.exit(1)


if __name__ == "__main__":
    main()
