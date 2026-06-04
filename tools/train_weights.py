from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any

DEFAULT_DENIAL_GRID = [0, 15, 35, 50, 75]


def load_weights(path: Path) -> dict[str, Any]:
    with path.open("r", encoding="utf-8") as file:
        data = json.load(file)
    required = {"version", "selfScorePercent", "opponentDenialPercent"}
    missing = sorted(required - set(data))
    if missing:
        raise ValueError(f"weights missing fields: {', '.join(missing)}")
    return data


def parse_grid(value: str) -> list[int]:
    return [int(part.strip()) for part in value.split(",") if part.strip()]


def generate_candidates(baseline: dict[str, Any], denial_grid: list[int]) -> list[dict[str, Any]]:
    candidates = []
    for index, denial_percent in enumerate(denial_grid):
        candidate = dict(baseline)
        candidate["version"] = f"candidate-denial-{denial_percent:03d}"
        candidate["opponentDenialPercent"] = denial_percent
        candidate["candidateIndex"] = index
        candidates.append(candidate)
    return candidates


def write_jsonl(path: Path, rows: list[dict[str, Any]]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8", newline="\n") as file:
        for row in rows:
            file.write(json.dumps(row, sort_keys=True) + "\n")


def main() -> None:
    parser = argparse.ArgumentParser(description="Prepare eval-weight candidates for later tuning.")
    parser.add_argument("--baseline", type=Path, default=Path("docs/weights.baseline.json"))
    parser.add_argument("--out", type=Path, default=Path("temp/training/weight_candidates.jsonl"))
    parser.add_argument(
        "--denial-grid",
        default=",".join(str(value) for value in DEFAULT_DENIAL_GRID),
    )
    parser.add_argument(
        "--validated-scorer",
        action="store_true",
        help="Acknowledge BGA final-score parity passed before using these for training.",
    )
    args = parser.parse_args()

    baseline = load_weights(args.baseline)
    candidates = generate_candidates(baseline, parse_grid(args.denial_grid))
    write_jsonl(args.out, candidates)
    print(
        json.dumps(
            {
                "written": str(args.out),
                "candidates": len(candidates),
                "validatedScorer": args.validated_scorer,
                "warning": None
                if args.validated_scorer
                else "Do not run self-play training until Side A 2p BGA score parity passes.",
            },
            indent=2,
        )
    )


if __name__ == "__main__":
    main()
