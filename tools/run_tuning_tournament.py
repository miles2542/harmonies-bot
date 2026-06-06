from __future__ import annotations

import argparse
import concurrent.futures
import datetime
import json
import os
import random
import subprocess
import sys
import uuid
from pathlib import Path
from typing import Any

# Define the tuning bounds for all parameters
PARAM_BOUNDS: dict[str, tuple[int, int]] = {
    "opponentDenialPercent": (0, 100),
    "selfScorePercentEarly": (50, 150),
    "selfScorePercentLate": (50, 150),
    "opponentDenialPercentEarly": (0, 100),
    "opponentDenialPercentLate": (0, 100),
    "completionProximityEarly": (0, 50),
    "completionProximityLate": (0, 50),
    "heightVarianceEarly": (-30, 0),
    "heightVarianceLate": (-30, 0),
    "wastedHeightPenaltyEarly": (-50, 0),
    "wastedHeightPenaltyLate": (-50, 0),
    "spiritOffsetEarly": (0, 50),
    "spiritOffsetLate": (0, 50),
    "spiritAbandonmentThreshold": (0, 10),
    "denialExponent": (50, 300),
}


def load_weights(path: Path) -> dict[str, Any]:
    """Load weights from a JSON file."""
    with path.open("r", encoding="utf-8") as file:
        return json.load(file)


def save_weights(path: Path, weights: dict[str, Any]) -> None:
    """Save weights to a JSON file."""
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8") as file:
        json.dump(weights, file, indent=2)


def clamp(val: int, min_val: int, max_val: int) -> int:
    """Clamp a value to the specified range."""
    return max(min_val, min(max_val, val))


def extract_snapshot(fixture_path: Path, temp_dir: Path) -> Path:
    """Extract GameSnapshotV1 from an AdvisorRequestV1 if nested, and return path to snapshot."""
    data = json.loads(fixture_path.read_text(encoding="utf-8"))
    snapshot = data.get("snapshot") if isinstance(data, dict) else None
    out_path = temp_dir / f"{fixture_path.stem}_snapshot.json"
    out_path.parent.mkdir(parents=True, exist_ok=True)
    if isinstance(snapshot, dict):
        with out_path.open("w", encoding="utf-8") as file:
            json.dump(snapshot, file, indent=2)
        return out_path
    if isinstance(data, dict) and data.get("schemaVersion") == 1:
        with out_path.open("w", encoding="utf-8") as file:
            json.dump(data, file, indent=2)
        return out_path
    raise ValueError(f"{fixture_path} is neither AdvisorRequestV1 nor GameSnapshotV1")


def run_single_game(
    cli_path: Path,
    snapshot_path: Path,
    candidate_weights: dict[str, Any],
    opponent_weights: dict[str, Any],
    seed: int,
    candidate_is_player_1: bool,
    turn_budget_ms: int,
    max_turns: int,
    temp_dir: Path,
) -> tuple[float, float]:
    """
    Run a single self-play game between candidate and opponent.
    Returns: (score_margin, win_points) where win_points is 1.0 for candidate win, 0.5 for draw, 0.0 for loss.
    """
    task_id: str = uuid.uuid4().hex
    candidate_file: Path = temp_dir / f"cand_{task_id}.json"
    opponent_file: Path = temp_dir / f"opp_{task_id}.json"

    try:
        # Save temp weights files
        save_weights(candidate_file, candidate_weights)
        save_weights(opponent_file, opponent_weights)

        # Seat assignment
        p1_weights: Path = candidate_file if candidate_is_player_1 else opponent_file
        p2_weights: Path = opponent_file if candidate_is_player_1 else candidate_file

        cmd: list[str] = [
            str(cli_path),
            "self-play",
            str(snapshot_path),
            "--weights",
            str(p1_weights),
            "--opponent-weights",
            str(p2_weights),
            "--seed",
            str(seed),
            "--turn-budget-ms",
            str(turn_budget_ms),
            "--max-turns",
            str(max_turns),
            "--validated-scorer",
        ]

        result = subprocess.run(cmd, capture_output=True, text=True, check=True)
        report: dict[str, Any] = json.loads(result.stdout)

        final_scores: list[dict[str, Any]] = report.get("finalScores", [])
        if len(final_scores) < 2:
            return 0.0, 0.0

        p1_score: int = final_scores[0]["total"]
        p2_score: int = final_scores[1]["total"]

        if candidate_is_player_1:
            candidate_score = p1_score
            opponent_score = p2_score
        else:
            candidate_score = p2_score
            opponent_score = p1_score

        margin: float = float(candidate_score - opponent_score)
        if candidate_score > opponent_score:
            outcome = 1.0
        elif candidate_score == opponent_score:
            outcome = 0.5
        else:
            outcome = 0.0

        return margin, outcome
    except Exception as e:
        print(f"Error executing game (seed={seed}): {e}", file=sys.stderr)
        return 0.0, 0.0
    finally:
        # Cleanup
        if candidate_file.exists():
            candidate_file.unlink()
        if opponent_file.exists():
            opponent_file.unlink()


def evaluate_candidate(
    cli_path: Path,
    snapshot_path: Path,
    candidate_weights: dict[str, Any],
    baseline_weights: dict[str, Any],
    num_games: int,
    turn_budget_ms: int,
    max_turns: int,
    temp_dir: Path,
    parallelism: int,
    start_seed: int = 1000,
) -> tuple[float, float]:
    """
    Run num_games balanced matches of candidate vs baseline.
    Returns: (average_win_rate, average_score_margin)
    """
    futures: list[concurrent.futures.Future[tuple[float, float]]] = []
    # Half the games candidate is Player 1, half Player 2
    games_to_run: int = (num_games // 2) * 2

    with concurrent.futures.ThreadPoolExecutor(max_workers=parallelism) as executor:
        for idx in range(games_to_run):
            candidate_is_player_1: bool = idx % 2 == 0
            seed: int = start_seed + idx
            futures.append(
                executor.submit(
                    run_single_game,
                    cli_path,
                    snapshot_path,
                    candidate_weights,
                    baseline_weights,
                    seed,
                    candidate_is_player_1,
                    turn_budget_ms,
                    max_turns,
                    temp_dir,
                )
            )

        results: list[tuple[float, float]] = [f.result() for f in futures]

    margins: list[float] = [r[0] for r in results]
    win_points: list[float] = [r[1] for r in results]

    avg_margin: float = sum(margins) / len(margins) if margins else 0.0
    avg_win_rate: float = sum(win_points) / len(win_points) if win_points else 0.0

    return avg_win_rate, avg_margin


def build_binary() -> Path:
    """Build harmonies-cli binary synchronously in release mode."""
    print("Compiling harmonies-cli in release mode...", flush=True)
    subprocess.run(["cargo", "build", "--release"], check=True)
    cli_path = Path("target/release/harmonies-cli.exe")
    if not cli_path.exists():
        cli_path = Path("target/release/harmonies-cli")
    if not cli_path.exists():
        raise FileNotFoundError("Could not find compiled harmonies-cli binary.")
    return cli_path


def main() -> None:
    parser = argparse.ArgumentParser(description="Run SPSA tuning tournament for overnight weights optimization.")
    parser.add_argument("--baseline", type=Path, default=Path("docs/weights.baseline.json"))
    parser.add_argument("--fixture", type=Path, default=Path("fixtures/advisor_requests/sidea_2p_nature_match12_early_spirit_choice_request.json"))
    parser.add_argument("--log-dir", type=Path, default=Path("logs/tuning"))
    parser.add_argument("--temp-dir", type=Path, default=Path("temp/tuning"))
    parser.add_argument("--games-per-eval", type=int, default=40, help="Number of games to evaluate a perturbed candidate (must be even).")
    parser.add_argument("--turn-budget-ms", type=int, default=20, help="Turn budget limit in milliseconds for fast simulations.")
    parser.add_argument("--max-turns", type=int, default=80)
    parser.add_argument("--iterations", type=int, default=500)
    parser.add_argument("--a", type=float, default=25.0, help="SPSA step scale a (parameter update factor)")
    parser.add_argument("--c", type=float, default=4.0, help="SPSA perturbation step size c")
    parser.add_argument("--parallelism", type=int, default=max(1, (os.cpu_count() or 4) - 1))
    args = parser.parse_args()

    # Compile native binary first
    cli_path = build_binary()

    # Setup directories
    args.log_dir.mkdir(parents=True, exist_ok=True)
    args.temp_dir.mkdir(parents=True, exist_ok=True)

    log_file: Path = args.log_dir / f"tuning_run_{datetime.datetime.now().strftime('%Y%m%d_%H%M%S')}.jsonl"
    snapshot_path = extract_snapshot(args.fixture, args.temp_dir)

    print(f"Loading baseline weights from {args.baseline}...")
    baseline = load_weights(args.baseline)

    current_best = dict(baseline)
    keys_to_tune = list(PARAM_BOUNDS.keys())

    print(f"Tuning parameters: {keys_to_tune}")
    print(f"Writing tournament logs to {log_file}")
    print(f"Using parallelism of {args.parallelism} threads")

    # Initial evaluation of baseline vs itself to establish noise level
    print("Running initial baseline self-check...", flush=True)
    base_wr, base_margin = evaluate_candidate(
        cli_path,
        snapshot_path,
        current_best,
        baseline,
        args.games_per_eval,
        args.turn_budget_ms,
        args.max_turns,
        args.temp_dir,
        args.parallelism,
        start_seed=1000,
    )
    print(f"Baseline vs Baseline: Win Rate = {base_wr:.2%}, Margin = {base_margin:.2f}")

    # Main SPSA Loop
    alpha = 0.602
    gamma = 0.101
    A = args.iterations * 0.1

    for k in range(args.iterations):
        # Calculate SPSA step sizes for iteration k
        ak = args.a / ((k + 1 + A) ** alpha)
        ck = args.c / ((k + 1) ** gamma)

        # Generate simultaneous perturbation vector Delta
        # Each Delta[i] is +1 or -1 with equal probability
        delta = {key: random.choice([1, -1]) for key in keys_to_tune}

        # Generate perturbed candidates theta_plus and theta_minus
        weights_plus = dict(current_best)
        weights_minus = dict(current_best)
        for key, sign in delta.items():
            min_val, max_val = PARAM_BOUNDS[key]
            val_plus = int(round(current_best[key] + ck * sign))
            val_minus = int(round(current_best[key] - ck * sign))
            weights_plus[key] = clamp(val_plus, min_val, max_val)
            weights_minus[key] = clamp(val_minus, min_val, max_val)

        weights_plus["version"] = f"tuning-spsa-k{k}-plus"
        weights_minus["version"] = f"tuning-spsa-k{k}-minus"

        # Evaluate both perturbations against the baseline
        # Use different start seeds for evaluations to reduce cross-correlation bias
        eval_seed = 2000 + k * 100
        wr_plus, margin_plus = evaluate_candidate(
            cli_path,
            snapshot_path,
            weights_plus,
            baseline,
            args.games_per_eval,
            args.turn_budget_ms,
            args.max_turns,
            args.temp_dir,
            args.parallelism,
            start_seed=eval_seed,
        )

        wr_minus, margin_minus = evaluate_candidate(
            cli_path,
            snapshot_path,
            weights_minus,
            baseline,
            args.games_per_eval,
            args.turn_budget_ms,
            args.max_turns,
            args.temp_dir,
            args.parallelism,
            start_seed=eval_seed,
        )

        # Calculate gradient using win rate (maximize win rate against baseline)
        diff = wr_plus - wr_minus

        # Log event structure
        log_entry = {
            "iteration": k,
            "timestamp": datetime.datetime.now().isoformat(),
            "step_size_ak": ak,
            "pert_size_ck": ck,
            "win_rate_plus": wr_plus,
            "win_rate_minus": wr_minus,
            "margin_plus": margin_plus,
            "margin_minus": margin_minus,
            "current_best": dict(current_best),
            "perturbation": delta,
        }

        # Update parameters
        new_best = dict(current_best)
        for key, sign in delta.items():
            # Estimate gradient component
            # g_i = (wr_plus - wr_minus) / (2 * ck * sign)
            g_i = (diff / (2.0 * ck)) * sign
            val_next = current_best[key] + ak * g_i
            min_val, max_val = PARAM_BOUNDS[key]
            new_best[key] = clamp(int(round(val_next)), min_val, max_val)

        current_best = new_best
        current_best["version"] = f"refined-spsa-k{k}"

        log_entry["new_best"] = dict(current_best)

        # Append structured JSON event line
        with log_file.open("a", encoding="utf-8") as f:
            f.write(json.dumps(log_entry) + "\n")

        print(
            f"Iteration {k:03d} | wr+: {wr_plus:.2%}, wr-: {wr_minus:.2%} | "
            f"Best Denial: {current_best['opponentDenialPercent']}%, DenialEarly: {current_best['opponentDenialPercentEarly']}%",
            flush=True,
        )

    # Save final optimized weights
    final_path = Path("docs/weights.optimized.json")
    save_weights(final_path, current_best)
    print(f"Optimized weights saved to {final_path}!")


if __name__ == "__main__":
    main()
