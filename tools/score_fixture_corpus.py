from __future__ import annotations

import argparse
import json
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Any


@dataclass(frozen=True)
class FixtureCase:
    name: str
    snapshot: Path
    expected: dict[str, int]


def load_manifest(path: Path) -> list[FixtureCase]:
    data = json.loads(path.read_text(encoding="utf-8"))
    cases = []
    for raw_case in data.get("cases", []):
        if not isinstance(raw_case, dict):
            continue
        name = str(raw_case.get("name", ""))
        snapshot = raw_case.get("snapshot")
        expected = raw_case.get("expected")
        if not name or not isinstance(snapshot, str) or not isinstance(expected, dict):
            raise SystemExit(f"invalid fixture case: {raw_case!r}")
        cases.append(
            FixtureCase(
                name=name,
                snapshot=(path.parent / snapshot).resolve(),
                expected={str(player_id): int(total) for player_id, total in expected.items()},
            )
        )
    if not cases:
        raise SystemExit("manifest has no fixture cases")
    return cases


def run_score(snapshot: Path, catalog: Path) -> dict[str, Any]:
    command = [
        "cargo",
        "run",
        "-q",
        "-p",
        "harmonies-cli",
        "--",
        "score",
        str(snapshot),
        "--catalog",
        str(catalog),
    ]
    completed = subprocess.run(command, check=True, capture_output=True, text=True)
    return json.loads(completed.stdout)


def totals_by_player(report: dict[str, Any]) -> dict[str, int]:
    totals = {}
    for player in report.get("players", []):
        player_id = str(player.get("playerId", ""))
        total = player.get("total")
        if player_id and isinstance(total, int):
            totals[player_id] = total
    return totals


def check_case(case: FixtureCase, catalog: Path) -> dict[str, Any]:
    report = run_score(case.snapshot, catalog)
    actual = totals_by_player(report)
    checks = []
    ok = True
    for player_id, expected_total in case.expected.items():
        actual_total = actual.get(player_id)
        matched = actual_total == expected_total
        ok = ok and matched
        checks.append(
            {
                "playerId": player_id,
                "expected": expected_total,
                "actual": actual_total,
                "ok": matched,
            }
        )
    return {
        "name": case.name,
        "snapshot": str(case.snapshot),
        "ok": ok,
        "checks": checks,
    }


def main() -> None:
    parser = argparse.ArgumentParser(description="Run scorer parity over tracked fixture corpus.")
    parser.add_argument(
        "--manifest",
        type=Path,
        default=Path("fixtures/score_parity/manifest.json"),
    )
    parser.add_argument("--catalog", type=Path, default=Path("docs/cards_database.json"))
    args = parser.parse_args()

    cases = load_manifest(args.manifest)
    results = [check_case(case, args.catalog) for case in cases]
    output = {"ok": all(result["ok"] for result in results), "cases": results}
    print(json.dumps(output, indent=2))
    if not output["ok"]:
        sys.exit(1)


if __name__ == "__main__":
    main()
