from __future__ import annotations

import argparse
import json
import statistics
import subprocess
import time
from pathlib import Path


def run_once(request_path: Path) -> float:
    started = time.perf_counter()
    subprocess.run(
        ["cargo", "run", "-q", "-p", "harmonies-cli", "--", str(request_path)],
        check=True,
        capture_output=True,
        text=True,
    )
    return (time.perf_counter() - started) * 1000


def percentile(values: list[float], percent: float) -> float:
    if not values:
        return 0.0
    ordered = sorted(values)
    index = min(len(ordered) - 1, round((percent / 100) * (len(ordered) - 1)))
    return ordered[index]


def main() -> None:
    parser = argparse.ArgumentParser(description="Benchmark harmonies-cli request latency.")
    parser.add_argument("request", type=Path)
    parser.add_argument("--runs", type=int, default=10)
    args = parser.parse_args()

    timings = [run_once(args.request) for _ in range(args.runs)]
    report = {
        "runs": args.runs,
        "meanMs": statistics.fmean(timings),
        "p50Ms": percentile(timings, 50),
        "p95Ms": percentile(timings, 95),
        "minMs": min(timings),
        "maxMs": max(timings),
    }
    print(json.dumps(report, indent=2))


if __name__ == "__main__":
    main()
