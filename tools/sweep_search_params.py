from __future__ import annotations

import argparse
import json
import tempfile
from dataclasses import dataclass
from pathlib import Path
from typing import Any

from tools.benchmark_cli import (
    DEFAULT_CATALOG,
    DEFAULT_WEIGHTS,
    build_cli,
    effective_request_path,
    run_once,
    summarize_request,
)

DEFAULT_FIXTURES = [
    Path("fixtures/advisor_requests/sidea_2p_nature_match14_full_hand_request.json"),
    Path("fixtures/advisor_requests/sidea_2p_nature_match14_after_completion_near_end_request.json"),
]


@dataclass(frozen=True)
class Candidate:
    name: str
    env: dict[str, str]


CANDIDATES = [
    Candidate("baseline", {}),
    Candidate(
        "balanced_narrow",
        {
            "HARMONIES_FUTURE_BEAM": "16",
            "HARMONIES_FUTURE_BRANCH": "8",
            "HARMONIES_REFILL_SAMPLES": "3",
            "HARMONIES_CARD_REFILL_SAMPLES": "1",
            "HARMONIES_HARD_STOP_MARGIN_MS": "3000",
            "HARMONIES_MIN_FUTURE_EXPAND_MS": "1500",
        },
    ),
    Candidate(
        "aggressive_narrow",
        {
            "HARMONIES_FUTURE_BEAM": "10",
            "HARMONIES_FUTURE_BRANCH": "5",
            "HARMONIES_REFILL_SAMPLES": "2",
            "HARMONIES_CARD_REFILL_SAMPLES": "1",
            "HARMONIES_HARD_STOP_MARGIN_MS": "3000",
            "HARMONIES_MIN_FUTURE_EXPAND_MS": "1500",
        },
    ),
]


def selected_candidates(names: list[str] | None) -> list[Candidate]:
    if not names:
        return CANDIDATES
    by_name = {candidate.name: candidate for candidate in CANDIDATES}
    missing = [name for name in names if name not in by_name]
    if missing:
        raise SystemExit(f"unknown candidates: {', '.join(missing)}")
    return [by_name[name] for name in names]


def sweep(args: argparse.Namespace) -> dict[str, Any]:
    executable = build_cli()
    fixtures = args.fixture or DEFAULT_FIXTURES
    candidates = selected_candidates(args.candidate)
    cases = []
    with tempfile.TemporaryDirectory(prefix="harmonies_search_sweep_") as tmp:
        tmp_dir = Path(tmp)
        for candidate in candidates:
            for fixture in fixtures:
                request_path = effective_request_path(fixture, tmp_dir, args.time_budget_ms)
                runs = [
                    run_once(
                        executable,
                        request_path,
                        args.catalog,
                        args.weights,
                        args.threads,
                        candidate.env,
                    )
                    for _ in range(args.runs)
                ]
                case = summarize_request(fixture, runs)
                case["candidate"] = candidate.name
                case["searchEnv"] = candidate.env
                cases.append(case)
    return {
        "schemaVersion": 1,
        "runsPerCase": args.runs,
        "threads": args.threads,
        "timeBudgetMs": args.time_budget_ms,
        "cases": cases,
        "summary": summarize_cases(cases),
    }


def summarize_cases(cases: list[dict[str, Any]]) -> list[dict[str, Any]]:
    rows = []
    for case in cases:
        first_plan = case["firstPlans"][0] if case.get("firstPlans") else {}
        rows.append(
            {
                "candidate": case["candidate"],
                "request": case["request"],
                "engineMeanMs": case["engineElapsedMs"]["mean"],
                "nodesMean": case["nodesEvaluated"]["mean"],
                "depthMax": case["depthCompleted"]["max"],
                "stoppedEarlyRuns": case["stoppedEarlyRuns"],
                "topGroup": first_plan.get("group"),
                "future": first_plan.get("future"),
                "utility": first_plan.get("utility"),
            }
        )
    return rows


def main() -> int:
    parser = argparse.ArgumentParser(description="Sweep advisor search parameter candidates.")
    parser.add_argument("--fixture", action="append", type=Path)
    parser.add_argument("--candidate", action="append")
    parser.add_argument("--runs", type=int, default=1)
    parser.add_argument("--threads", type=int, default=12)
    parser.add_argument("--time-budget-ms", type=int, default=30000)
    parser.add_argument("--catalog", type=Path, default=DEFAULT_CATALOG)
    parser.add_argument("--weights", type=Path, default=DEFAULT_WEIGHTS)
    parser.add_argument("--out", type=Path, default=Path("logs/benchmarks/search-param-sweep.json"))
    args = parser.parse_args()

    report = sweep(args)
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    print(json.dumps({"written": str(args.out), "summary": report["summary"]}, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
