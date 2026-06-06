from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any

DEFAULT_DENIAL_GRID: list[int] = [0, 15, 35, 50, 75]

REQUIRED_FIELDS: dict[str, type] = {
    "version": str,
    "selfScorePercent": int,
    "opponentDenialPercent": int,
    "selfScorePercentEarly": int,
    "selfScorePercentLate": int,
    "opponentDenialPercentEarly": int,
    "opponentDenialPercentLate": int,
    "completionProximityEarly": int,
    "completionProximityLate": int,
    "heightVarianceEarly": int,
    "heightVarianceLate": int,
    "wastedHeightPenaltyEarly": int,
    "wastedHeightPenaltyLate": int,
    "spiritOffsetEarly": int,
    "spiritOffsetLate": int,
    "spiritAbandonmentThreshold": int,
    "denialExponent": int,
}


def validate_weights(data: dict[str, Any]) -> None:
    """Validate that weights match the expected EvalWeights schema and types."""
    for field, expected_type in REQUIRED_FIELDS.items():
        if field not in data:
            raise ValueError(f"Missing required weight field: {field}")
        val = data[field]
        if expected_type is int:
            if isinstance(val, bool) or not isinstance(val, (int, float)):
                raise ValueError(f"Field {field} must be integer, got {type(val).__name__}")
            # Ensure it is equivalent to an int
            if isinstance(val, float) and not val.is_integer():
                raise ValueError(f"Field {field} must be integer, got float {val}")
        else:
            if not isinstance(val, expected_type):
                raise ValueError(f"Field {field} must be {expected_type.__name__}, got {type(val).__name__}")


def load_weights(path: Path) -> dict[str, Any]:
    """Load and validate weights from a JSON file."""
    with path.open("r", encoding="utf-8") as file:
        data = json.load(file)
    validate_weights(data)
    return data


def parse_grid(value: str) -> list[int]:
    """Parse a comma-separated list of integers."""
    return [int(part.strip()) for part in value.split(",") if part.strip()]


def generate_candidates(baseline: dict[str, Any], denial_grid: list[int]) -> list[dict[str, Any]]:
    """Generate candidate weights by varying the opponentDenialPercent parameter."""
    candidates: list[dict[str, Any]] = []
    for index, denial_percent in enumerate(denial_grid):
        candidate = dict(baseline)
        candidate["version"] = f"candidate-denial-{denial_percent:03d}"
        candidate["opponentDenialPercent"] = denial_percent
        candidate["candidateIndex"] = index
        # Validate each generated candidate to ensure it fits the schema
        validate_weights(candidate)
        candidates.append(candidate)
    return candidates


def write_jsonl(path: Path, rows: list[dict[str, Any]]) -> None:
    """Write list of dictionaries to a JSONL file."""
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

