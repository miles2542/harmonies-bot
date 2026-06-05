from __future__ import annotations

import argparse
import json
import os
import statistics
import subprocess
import tempfile
import time
from dataclasses import dataclass
from pathlib import Path
from typing import Any

DEFAULT_REQUESTS = [
    Path("fixtures/advisor_requests/sidea_2p_nature_match12_early_spirit_choice_request.json"),
    Path("fixtures/advisor_requests/sidea_2p_nature_match12_late_active_turn_request.json"),
    Path("fixtures/advisor_requests/sidea_2p_nature_match14_full_hand_request.json"),
    Path("fixtures/advisor_requests/sidea_2p_nature_match14_after_completion_near_end_request.json"),
]
DEFAULT_CATALOG = Path("docs/cards_database.json")
DEFAULT_WEIGHTS = Path("docs/weights.baseline.json")


@dataclass(frozen=True)
class RunResult:
    wall_ms: float
    response: dict[str, Any]


def build_cli() -> Path:
    subprocess.run(["cargo", "build", "-q", "-p", "harmonies-cli"], check=True)
    suffix = ".exe" if os.name == "nt" else ""
    return Path("target") / "debug" / f"harmonies-cli{suffix}"


def run_once(
    executable: Path,
    request_path: Path,
    catalog_path: Path,
    weights_path: Path,
    threads: int | None,
) -> RunResult:
    env = os.environ.copy()
    if threads:
        env["RAYON_NUM_THREADS"] = str(threads)
    started = time.perf_counter()
    completed = subprocess.run(
        [str(executable), str(request_path), str(catalog_path), str(weights_path)],
        check=True,
        capture_output=True,
        text=True,
        env=env,
    )
    return RunResult(
        wall_ms=(time.perf_counter() - started) * 1000,
        response=json.loads(completed.stdout),
    )


def percentile(values: list[float], percent: float) -> float:
    if not values:
        return 0.0
    ordered = sorted(values)
    index = min(len(ordered) - 1, round((percent / 100) * (len(ordered) - 1)))
    return ordered[index]


def stats(values: list[float]) -> dict[str, float]:
    return {
        "mean": statistics.fmean(values) if values else 0.0,
        "p50": percentile(values, 50),
        "p95": percentile(values, 95),
        "min": min(values) if values else 0.0,
        "max": max(values) if values else 0.0,
    }


def first_plan(response: dict[str, Any]) -> dict[str, Any]:
    moves = response.get("bestMoves") if isinstance(response.get("bestMoves"), list) else []
    first = moves[0] if moves else {}
    breakdown = first.get("scoreBreakdown") if isinstance(first, dict) else {}
    return {
        "group": first.get("centralGroupIndex"),
        "utility": first.get("utilityEstimate"),
        "future": first.get("scoreEstimate"),
        "immediate": sum(
            int(breakdown.get(key, 0))
            for key in ("trees", "mountains", "fields", "buildings", "water", "animals", "spirits")
        )
        if isinstance(breakdown, dict)
        else None,
    }


def summarize_request(request_path: Path, runs: list[RunResult]) -> dict[str, Any]:
    responses = [run.response for run in runs]
    progress = [response.get("progress", {}) for response in responses]
    first_plans = [first_plan(response) for response in responses]
    top_groups = [plan["group"] for plan in first_plans]
    engine_elapsed = [float(response.get("elapsedMs", 0)) for response in responses]
    nodes = [float(item.get("nodesEvaluated", 0)) for item in progress]
    depths = [int(item.get("depthCompleted", 0)) for item in progress]
    return {
        "request": str(request_path),
        "runs": len(runs),
        "wallMs": stats([run.wall_ms for run in runs]),
        "engineElapsedMs": stats(engine_elapsed),
        "nodesEvaluated": stats(nodes),
        "depthCompleted": {"min": min(depths), "max": max(depths), "values": depths},
        "topGroups": top_groups,
        "uniqueTopGroups": sorted({group for group in top_groups if group is not None}),
        "firstPlans": first_plans,
        "stoppedEarlyRuns": sum(1 for item in progress if item.get("stoppedEarly")),
    }


def benchmark(args: argparse.Namespace) -> dict[str, Any]:
    executable = build_cli()
    request_paths = args.request or DEFAULT_REQUESTS
    cases = []
    with tempfile.TemporaryDirectory(prefix="harmonies_benchmark_requests_") as tmp:
        tmp_dir = Path(tmp)
        for request_path in request_paths:
            effective_request = effective_request_path(request_path, tmp_dir, args.time_budget_ms)
            runs = [
                run_once(executable, effective_request, args.catalog, args.weights, args.threads)
                for _ in range(args.runs)
            ]
            cases.append(summarize_request(request_path, runs))
    return {
        "schemaVersion": 1,
        "runsPerRequest": args.runs,
        "catalog": str(args.catalog),
        "weights": str(args.weights),
        "timeBudgetMsOverride": args.time_budget_ms,
        "rayonNumThreads": args.threads or os.environ.get("RAYON_NUM_THREADS") or "default",
        "cases": cases,
    }


def effective_request_path(
    request_path: Path,
    tmp_dir: Path,
    time_budget_ms: int | None,
) -> Path:
    if time_budget_ms is None:
        return request_path
    request = json.loads(request_path.read_text(encoding="utf-8"))
    if not isinstance(request, dict):
        raise SystemExit(f"{request_path} must contain AdvisorRequestV1 object")
    request["timeBudgetMs"] = time_budget_ms
    output = tmp_dir / request_path.name
    output.write_text(json.dumps(request, indent=2) + "\n", encoding="utf-8")
    return output


def main() -> int:
    parser = argparse.ArgumentParser(description="Benchmark harmonies-cli advisor requests.")
    parser.add_argument("request", nargs="*", type=Path)
    parser.add_argument("--runs", type=int, default=3)
    parser.add_argument("--catalog", type=Path, default=DEFAULT_CATALOG)
    parser.add_argument("--weights", type=Path, default=DEFAULT_WEIGHTS)
    parser.add_argument("--threads", type=int)
    parser.add_argument("--time-budget-ms", type=int)
    parser.add_argument("--out", type=Path)
    args = parser.parse_args()

    report = benchmark(args)
    output = json.dumps(report, indent=2)
    if args.out:
        args.out.parent.mkdir(parents=True, exist_ok=True)
        args.out.write_text(output + "\n", encoding="utf-8")
    print(output)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
