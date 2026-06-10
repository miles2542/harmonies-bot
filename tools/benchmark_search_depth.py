import subprocess
import json
import sys
import argparse
from pathlib import Path
from concurrent.futures import ThreadPoolExecutor, as_completed
import time
import os
import random

# Use weights.spirit_focused.json as the baseline weights
DEFAULT_WEIGHTS = "docs/weights.spirit_focused.json"
SNAPSHOT_PATH = "temp/tuning/turn0_snapshot.json"

def run_single_game(cli_path, weights_path, seed, search_settings):
    # Setup temporary snapshot per seed (deterministic randomization per seed)
    try:
        with open(SNAPSHOT_PATH, "r") as f:
            snapshot = json.load(f)
        
        local_random = random.Random(seed)
        
        # 1. Assign 2 random spirit cards (type arg 33-42) to both players
        snapshot["players"][0]["spiritCardChoices"] = [
            {"cardId": 30001, "typeArg": int(t), "remainingCubes": 1, "isSpirit": True}
            for t in local_random.sample(range(33, 43), 2)
        ]
        snapshot["players"][1]["spiritCardChoices"] = [
            {"cardId": 30003, "typeArg": int(t), "remainingCubes": 1, "isSpirit": True}
            for t in local_random.sample(range(33, 43), 2)
        ]
        
        # 2. Randomize starting river cards (5 unique standard cards from 1-32)
        with open("docs/cards_database.json", "r") as db_file:
            db = json.load(db_file)
        river_types = local_random.sample(range(1, 33), 5)
        snapshot["riverCards"] = []
        for i, t in enumerate(river_types):
            cubes = len(db[str(t)]["pointLocations"])
            snapshot["riverCards"].append({
                "cardId": 20000 + i,
                "typeArg": t,
                "remainingCubes": cubes,
                "isSpirit": False
            })
            
        # 3. Randomize central groups and bag counts (standard bag = 135 total)
        bag = (
            ["water"] * 24 +
            ["mountain"] * 24 +
            ["trunk"] * 22 +
            ["foliage"] * 20 +
            ["field"] * 20 +
            ["building"] * 16
        )
        local_random.shuffle(bag)
        
        snapshot["centralTokenGroups"] = []
        for _ in range(5):
            snapshot["centralTokenGroups"].append([bag.pop() for _ in range(3)])
            
        snapshot["bagCounts"] = {
            "water": bag.count("water"),
            "mountain": bag.count("mountain"),
            "trunk": bag.count("trunk"),
            "foliage": bag.count("foliage"),
            "field": bag.count("field"),
            "building": bag.count("building"),
            "unknown": 0
        }
        
        temp_dir = Path("temp/tuning")
        temp_dir.mkdir(parents=True, exist_ok=True)
        temp_snapshot_path = temp_dir / f"temp_benchmark_snapshot_{seed}.json"
        with open(temp_snapshot_path, "w") as f:
            json.dump(snapshot, f)
    except Exception as e:
        return {
            "seed": seed,
            "elapsed": 0.0,
            "scores": [],
            "error": f"Setup error: {e}"
        }

    cmd = [
        str(cli_path),
        "self-play",
        str(temp_snapshot_path),
        "--weights",
        str(weights_path),
        "--seed",
        str(seed),
        "--turn-budget-ms",
        "1500",
        "--validated-scorer",
    ]
    
    env = os.environ.copy()
    env["RAYON_NUM_THREADS"] = "2"
    env["HARMONIES_ROOT_BEAM"] = str(search_settings["root_beam"])
    env["HARMONIES_FUTURE_BEAM"] = str(search_settings["future_beam"])
    env["HARMONIES_FUTURE_BRANCH"] = str(search_settings["future_branch"])
    env["HARMONIES_FUTURE_DEPTH"] = str(search_settings["future_depth"])
    
    # Tuned Spirit Focused settings (from Overnight/H4/Quick Validation results)
    env["HARMONIES_HEURISTIC_MODE"] = "dynamic_demand_tuned"
    env["HARMONIES_SPIRIT_PROX_MULT"] = "2.5"
    env["HARMONIES_SPIRIT_PENALTY_COEFF"] = "-4.0"
    env["HARMONIES_COMPENSATION_COEFF"] = "-0.05"
    
    start_time = time.time()
    try:
        result = subprocess.run(cmd, capture_output=True, text=True, check=True, env=env)
        elapsed = time.time() - start_time
        report = json.loads(result.stdout)
        final_scores = report.get("finalScores", [])
        
        if temp_snapshot_path.exists():
            temp_snapshot_path.unlink()
            
        return {
            "seed": seed,
            "elapsed": elapsed,
            "scores": final_scores,
            "error": None
        }
    except Exception as e:
        elapsed = time.time() - start_time
        if temp_snapshot_path.exists():
            try:
                temp_snapshot_path.unlink()
            except:
                pass
        return {
            "seed": seed,
            "elapsed": elapsed,
            "scores": [],
            "error": str(e)
        }

def compute_metrics(results):
    successful = 0
    all_vp = []
    animal_vp = []
    spirit_vp = []
    completed_spirits = 0
    total_elapsed = 0.0
    
    for r in results:
        if r["error"] is not None:
            continue
        successful += 1
        total_elapsed += r["elapsed"]
        for p in r["scores"]:
            all_vp.append(p["total"])
            bk = p["breakdown"]
            animal_vp.append(bk.get("animals", 0))
            sv = bk.get("spirits", 0)
            spirit_vp.append(sv)
            if sv > 0:
                completed_spirits += 1

    if successful == 0:
        return None
        
    n_runs = len(all_vp)
    mean_vp = sum(all_vp) / n_runs
    min_vp = min(all_vp)
    max_vp = max(all_vp)
    avg_animal = sum(animal_vp) / n_runs
    completion_rate = (completed_spirits / n_runs) * 100
    avg_elapsed = total_elapsed / successful
    
    return {
        "mean_vp": mean_vp,
        "min_vp": min_vp,
        "max_vp": max_vp,
        "avg_animal": avg_animal,
        "completion_rate": completion_rate,
        "avg_elapsed_sec": avg_elapsed,
        "games_completed": successful
    }

def main():
    parser = argparse.ArgumentParser(description="Benchmark Harmonies search depth")
    parser.add_argument("--games", type=int, default=20, help="Number of games to run")
    parser.add_argument("--workers", type=int, default=6, help="Number of concurrent workers")
    parser.add_argument("--fast-only", action="store_true", help="Run only fast settings benchmark")
    parser.add_argument("--exhaustive-only", action="store_true", help="Run only exhaustive settings benchmark")
    args = parser.parse_args()

    cli_path = Path("target/release/harmonies-cli.exe")
    if not cli_path.exists():
        cli_path = Path("target/release/harmonies-cli")
    if not cli_path.exists():
        print("Error: Compile binary first (cargo build --release)")
        sys.exit(1)

    # Fast settings
    fast_base = {
        "root_beam": 256,
        "future_beam": 32,
        "future_branch": 16
    }
    
    # Exhaustive settings
    exhaustive_base = {
        "root_beam": 8192,
        "future_beam": 256,
        "future_branch": 32
    }

    scenarios = []
    
    if not args.exhaustive_only:
        scenarios.append(("Fast, Depth = 1", {**fast_base, "future_depth": 1}))
        scenarios.append(("Fast, Depth = 2", {**fast_base, "future_depth": 2}))
        
    if not args.fast_only:
        scenarios.append(("Exhaustive, Depth = 1", {**exhaustive_base, "future_depth": 1}))
        scenarios.append(("Exhaustive, Depth = 2", {**exhaustive_base, "future_depth": 2}))

    seeds = list(range(101, 101 + args.games))
    summary_data = []

    print(f"Benchmarking search depth over {args.games} games (seeds {seeds[0]}-{seeds[-1]}) with {args.workers} workers.\n")

    for name, settings in scenarios:
        print(f"--- Running Scenario: {name} ---")
        print(f"Settings: {settings}")
        results = []
        with ThreadPoolExecutor(max_workers=args.workers) as executor:
            futures = {
                executor.submit(run_single_game, cli_path, DEFAULT_WEIGHTS, seed, settings): seed
                for seed in seeds
            }
            for fut in as_completed(futures):
                seed = futures[fut]
                res = fut.result()
                results.append(res)
                if res["error"]:
                    print(f"  Seed {seed} failed: {res['error']}")
                else:
                    p1_val = res["scores"][0]["total"]
                    p2_val = res["scores"][1]["total"]
                    print(f"  Seed {seed} completed in {res['elapsed']:.1f}s: P1={p1_val} VP, P2={p2_val} VP")
        
        metrics = compute_metrics(results)
        if metrics:
            summary_data.append((name, metrics))
            print(f"Result for {name}: Mean VP = {metrics['mean_vp']:.2f} | Spirit Comp = {metrics['completion_rate']:.1f}% | Avg Game Time = {metrics['avg_elapsed_sec']:.1f}s\n")
        else:
            print(f"Error: No successful runs for scenario {name}.\n")

    # Output Markdown Table
    print("\n================ BENCHMARK RESULT SUMMARY ================")
    print("| Configuration | Mean VP | Min VP | Max VP | Animal VP | Spirit Comp % | Avg Game Time (s) |")
    print("|---|---|---|---|---|---|---|")
    for name, m in summary_data:
        print(f"| **{name}** | {m['mean_vp']:.2f} | {m['min_vp']} | {m['max_vp']} | {m['avg_animal']:.2f} | {m['completion_rate']:.1f}% | {m['avg_elapsed_sec']:.1f}s |")
    print("==========================================================")

if __name__ == "__main__":
    main()
