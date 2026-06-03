from __future__ import annotations

import argparse
import json
from pathlib import Path

try:
    from snapshot_qa_core import (
        Comparison,
        SnapshotSummary,
        auto_comparisons,
        compare_summaries,
        summarize_file,
    )
except ModuleNotFoundError:
    from tools.snapshot_qa_core import (
        Comparison,
        SnapshotSummary,
        auto_comparisons,
        compare_summaries,
        summarize_file,
    )


def format_summary(summary: SnapshotSummary) -> str:
    counts = "; ".join(f"{key}={value}" for key, value in summary.counts.items())
    metadata = "; ".join(f"{key}={value}" for key, value in summary.metadata.items())
    lines = [f"== {summary.path} ({summary.kind})", counts]
    if metadata:
        lines.append(metadata)
    for player in summary.players:
        lines.append(
            f"- {player.player_id}: cells={player.cells}; nonempty={player.nonempty_cells}; "
            f"locked={player.locked_cells}; tokens={player.tokens}; active={player.active_cards}; "
            f"done={player.completed_cards}; emptyHexes={player.empty_hexes}",
        )
    lines.extend(f"[{issue.severity}] {issue.message}" for issue in summary.issues)
    return "\n".join(lines)


def format_comparison(comparison: Comparison) -> str:
    lines = [f"== compare {comparison.raw_path} -> {comparison.normalized_path}"]
    if comparison.issues:
        lines.extend(f"[{issue.severity}] {issue.message}" for issue in comparison.issues)
    else:
        lines.append("ok")
    return "\n".join(lines)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Summarize raw BGA snapshots and normalized GameSnapshotV1 output.",
    )
    parser.add_argument("files", nargs="*", type=Path, help="Raw or normalized JSON files.")
    parser.add_argument(
        "--compare",
        nargs=2,
        action="append",
        type=Path,
        metavar=("RAW", "NORMALIZED"),
        help="Compare one raw snapshot with its normalized CLI output.",
    )
    parser.add_argument("--json", action="store_true", help="Emit machine-readable JSON report.")
    return parser.parse_args()


def collect_paths(files: list[Path], compare_pairs: list[list[Path]]) -> list[Path]:
    paths = list(files)
    for pair in compare_pairs:
        paths.extend(pair)
    return paths


def build_comparisons(
    compare_pairs: list[list[Path]],
    summaries_by_path: dict[Path, SnapshotSummary],
) -> list[Comparison]:
    if compare_pairs:
        return [
            compare_summaries(summaries_by_path[raw_path], summaries_by_path[normalized_path])
            for raw_path, normalized_path in compare_pairs
        ]
    return auto_comparisons(list(summaries_by_path.values()))


def main() -> None:
    args = parse_args()
    compare_pairs: list[list[Path]] = args.compare or []
    paths = collect_paths(args.files, compare_pairs)
    if not paths:
        raise SystemExit("usage: snapshot_qa.py [--json] [--compare RAW NORMALIZED] FILE...")

    summaries_by_path = {path: summarize_file(path) for path in dict.fromkeys(paths)}
    comparisons = build_comparisons(compare_pairs, summaries_by_path)
    if args.json:
        print(
            json.dumps(
                {
                    "summaries": [summary.to_dict() for summary in summaries_by_path.values()],
                    "comparisons": [comparison.to_dict() for comparison in comparisons],
                },
                indent=2,
            ),
        )
        return

    sections = [format_summary(summary) for summary in summaries_by_path.values()]
    sections.extend(format_comparison(comparison) for comparison in comparisons)
    print("\n\n".join(sections))


if __name__ == "__main__":
    main()
