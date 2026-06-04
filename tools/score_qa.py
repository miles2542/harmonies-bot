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


@dataclass(frozen=True)
class BgaResult:
    player_id: str
    total: int
    score_aux: int | None


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


def load_snapshot(snapshot: Path) -> dict[str, Any]:
    with snapshot.open("r", encoding="utf-8") as file:
        data = json.load(file)
    if not isinstance(data, dict):
        raise SystemExit("snapshot root must be a JSON object")
    return data


def selected_gamedatas(data: dict[str, Any]) -> dict[str, Any]:
    gamedatas = data.get("gamedatas", data)
    return gamedatas if isinstance(gamedatas, dict) else {}


def load_bga_results(snapshot: Path) -> list[BgaResult]:
    data = load_snapshot(snapshot)
    gamedatas = selected_gamedatas(data)
    result = gamedatas.get("gamestate", {}).get("args", {}).get("result", [])
    expected = []
    for item in result:
        if not isinstance(item, dict):
            continue
        player_id = str(item.get("player") or item.get("id") or "")
        raw_total = item.get("score")
        try:
            total = int(raw_total)
        except (TypeError, ValueError):
            continue
        score_aux = maybe_int(item.get("score_aux"))
        if player_id:
            expected.append(BgaResult(player_id=player_id, total=total, score_aux=score_aux))
    return expected


def load_bga_result_expected(snapshot: Path) -> list[ExpectedScore]:
    return [
        ExpectedScore(player_id=result.player_id, total=result.total)
        for result in load_bga_results(snapshot)
    ]


def maybe_int(value: object) -> int | None:
    try:
        return int(value)
    except (TypeError, ValueError):
        return None


def raw_board_cube_counts(snapshot: Path) -> dict[str, int]:
    data = load_snapshot(snapshot)
    gamedatas = selected_gamedatas(data)
    players = gamedatas.get("players", {})
    if not isinstance(players, dict):
        return {}
    counts: dict[str, int] = {}
    for player_id, player in players.items():
        if not isinstance(player, dict):
            continue
        cubes = player.get("animalCubesOnBoard")
        if isinstance(cubes, list):
            counts[str(player_id)] = len(cubes)
        elif isinstance(cubes, dict):
            counts[str(player_id)] = sum(
                len(value) if isinstance(value, list) else 1
                for value in cubes.values()
            )
    return counts


def capture_warnings(snapshot: Path, use_bga_result: bool) -> list[dict[str, Any]]:
    if not use_bga_result:
        return []
    cube_counts = raw_board_cube_counts(snapshot)
    warnings = []
    for result in load_bga_results(snapshot):
        if result.score_aux is None:
            continue
        raw_cubes = cube_counts.get(result.player_id)
        if raw_cubes is not None and raw_cubes != result.score_aux:
            warnings.append(
                {
                    "kind": "captureStateMismatch",
                    "playerId": result.player_id,
                    "message": (
                        "BGA result score_aux differs from captured "
                        "animalCubesOnBoard count"
                    ),
                    "scoreAux": result.score_aux,
                    "capturedBoardCubes": raw_cubes,
                }
            )
    return warnings


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
    parser.add_argument("--use-bga-result", action="store_true")
    parser.add_argument("--perspective")
    parser.add_argument("--catalog", type=Path, default=Path("docs/cards_database.json"))
    parser.add_argument("--report", type=Path)
    args = parser.parse_args()

    expected = list(args.expected)
    if args.use_capture_scores:
        expected.extend(load_capture_expected(args.snapshot))
    if args.use_bga_result:
        expected.extend(load_bga_result_expected(args.snapshot))
    if not expected:
        raise SystemExit("no expected scores provided or found in selected snapshot fields")
    report = run_score(args.snapshot, args.perspective, args.catalog)
    comparison = compare_scores(report, expected)
    comparison["warnings"] = capture_warnings(args.snapshot, args.use_bga_result)
    output = json.dumps(comparison, indent=2)
    print(output)
    if args.report:
        args.report.write_text(output + "\n", encoding="utf-8")
    if not comparison["ok"]:
        sys.exit(1)


if __name__ == "__main__":
    main()
