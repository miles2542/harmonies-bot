from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path
from statistics import mean
from typing import Any

DEFAULT_FIXTURES = [
    Path("fixtures/advisor_requests/sidea_2p_nature_match12_early_spirit_choice_request.json"),
    Path("fixtures/advisor_requests/sidea_2p_nature_match12_late_active_turn_request.json"),
]


@dataclass(frozen=True)
class Candidate:
    index: int
    version: str
    weights: dict[str, Any]


def load_candidates(path: Path, limit: int | None) -> list[Candidate]:
    candidates = []
    for line_number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), start=1):
        if not line.strip():
            continue
        weights = json.loads(line)
        version = str(weights.get("version") or f"candidate-{line_number}")
        candidates.append(Candidate(index=len(candidates), version=version, weights=weights))
        if limit is not None and len(candidates) >= limit:
            break
    if not candidates:
        raise SystemExit(f"no candidates in {path}")
    return candidates


def parse_seeds(value: str) -> list[int]:
    seeds = [int(part.strip()) for part in value.split(",") if part.strip()]
    if not seeds:
        raise SystemExit("--seeds must contain at least one integer")
    return seeds


def run_score_gate() -> None:
    subprocess.run(
        [sys.executable, "-m", "tools.score_fixture_corpus"],
        check=True,
        stdout=subprocess.DEVNULL,
    )


def normalized_snapshot_from_file(path: Path) -> dict[str, Any]:
    data = json.loads(path.read_text(encoding="utf-8"))
    snapshot = data.get("snapshot") if isinstance(data, dict) else None
    if isinstance(snapshot, dict):
        return snapshot
    if isinstance(data, dict) and data.get("schemaVersion") == 1:
        return data
    raise SystemExit(f"{path} is neither AdvisorRequestV1 nor GameSnapshotV1")


def write_temp_json(path: Path, data: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(data, indent=2) + "\n", encoding="utf-8")


def safe_name(value: str) -> str:
    return re.sub(r"[^A-Za-z0-9_.-]+", "_", value).strip("_") or "candidate"


def run_self_play(
    snapshot_path: Path,
    weights_path: Path,
    seed: int,
    turn_budget_ms: int,
    max_turns: int,
) -> dict[str, Any]:
    command = [
        "cargo",
        "run",
        "-q",
        "-p",
        "harmonies-cli",
        "--",
        "self-play",
        str(snapshot_path),
        "--weights",
        str(weights_path),
        "--turn-budget-ms",
        str(turn_budget_ms),
        "--max-turns",
        str(max_turns),
        "--seed",
        str(seed),
        "--validated-scorer",
    ]
    completed = subprocess.run(command, check=True, capture_output=True, text=True)
    return json.loads(completed.stdout)


def fitness_from_report(report: dict[str, Any], target_player_id: str) -> dict[str, Any]:
    scores = {
        str(player["playerId"]): int(player["total"])
        for player in report.get("finalScores", [])
        if "playerId" in player and "total" in player
    }
    target_score = scores.get(target_player_id)
    opponent_scores = [
        score for player_id, score in scores.items() if player_id != target_player_id
    ]
    if target_score is None or not opponent_scores:
        raise RuntimeError(f"missing target/opponent scores in report: {scores}")
    opponent_mean = mean(opponent_scores)
    return {
        "targetScore": target_score,
        "opponentMean": opponent_mean,
        "fitness": target_score - opponent_mean,
    }


def summarize(rows: list[dict[str, Any]]) -> list[dict[str, Any]]:
    by_candidate: dict[str, list[dict[str, Any]]] = {}
    for row in rows:
        by_candidate.setdefault(str(row["candidateVersion"]), []).append(row)
    summary = []
    for version, candidate_rows in by_candidate.items():
        summary.append(
            {
                "candidateVersion": version,
                "runs": len(candidate_rows),
                "avgFitness": mean(row["fitness"] for row in candidate_rows),
                "avgTurns": mean(row["turns"] for row in candidate_rows),
                "completedRuns": sum(1 for row in candidate_rows if row["completed"]),
            }
        )
    return sorted(summary, key=lambda row: row["avgFitness"], reverse=True)


def evaluate(args: argparse.Namespace) -> dict[str, Any]:
    candidates = load_candidates(args.candidates, args.max_candidates)
    seeds = parse_seeds(args.seeds)
    fixture_paths = args.fixture or DEFAULT_FIXTURES
    temp_dir = args.temp_dir
    rows = []
    for fixture_path in fixture_paths:
        snapshot = normalized_snapshot_from_file(fixture_path)
        target_player_id = str(snapshot["perspectivePlayerId"])
        snapshot_path = temp_dir / "snapshots" / f"{fixture_path.stem}.json"
        write_temp_json(snapshot_path, snapshot)
        for candidate in candidates:
            weights_path = temp_dir / "weights" / f"{safe_name(candidate.version)}.json"
            write_temp_json(weights_path, candidate.weights)
            for seed in seeds:
                report = run_self_play(
                    snapshot_path,
                    weights_path,
                    seed,
                    args.turn_budget_ms,
                    args.max_turns,
                )
                metrics = fitness_from_report(report, target_player_id)
                rows.append(
                    {
                        "candidateIndex": candidate.index,
                        "candidateVersion": candidate.version,
                        "fixture": str(fixture_path),
                        "seed": seed,
                        "completed": bool(report.get("completed")),
                        "turns": len(report.get("turns", [])),
                        "warnings": len(report.get("warnings", [])),
                        **metrics,
                    }
                )
    return {
        "schemaVersion": 1,
        "turnBudgetMs": args.turn_budget_ms,
        "maxTurns": args.max_turns,
        "seeds": seeds,
        "summary": summarize(rows),
        "runs": rows,
    }


def main() -> None:
    parser = argparse.ArgumentParser(description="Evaluate weight candidates via Rust self-play.")
    parser.add_argument(
        "--candidates",
        type=Path,
        default=Path("temp/training/weight_candidates.jsonl"),
    )
    parser.add_argument("--out", type=Path, default=Path("temp/training/evaluation.json"))
    parser.add_argument("--fixture", action="append", type=Path)
    parser.add_argument("--seeds", default="1")
    parser.add_argument("--turn-budget-ms", type=int, default=250)
    parser.add_argument("--max-turns", type=int, default=6)
    parser.add_argument("--max-candidates", type=int)
    parser.add_argument("--temp-dir", type=Path, default=Path("temp/training/evaluate_weights"))
    parser.add_argument("--skip-score-gate", action="store_true")
    args = parser.parse_args()

    if not args.skip_score_gate:
        run_score_gate()
    output = evaluate(args)
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(json.dumps(output, indent=2) + "\n", encoding="utf-8")
    print(json.dumps({"written": str(args.out), "summary": output["summary"]}, indent=2))


if __name__ == "__main__":
    main()
