import subprocess
import json
import sys
import argparse
from pathlib import Path
from concurrent.futures import ThreadPoolExecutor, as_completed
import time
import os
import random

DEFAULT_WEIGHTS = "docs/weights.spirit_focused.json"
SNAPSHOT_PATH = "temp/tuning/turn0_snapshot.json"

def run_single_game(cli_path, weights_path, seed, search_settings):
    try:
        with open(SNAPSHOT_PATH, "r") as f:
            snapshot = json.load(f)
        
        local_random = random.Random(seed)
        
        snapshot["players"][0]["spiritCardChoices"] = [
            {"cardId": 30001, "typeArg": int(t), "remainingCubes": 1, "isSpirit": True}
            for t in local_random.sample(range(33, 43), 2)
        ]
        snapshot["players"][1]["spiritCardChoices"] = [
            {"cardId": 30003, "typeArg": int(t), "remainingCubes": 1, "isSpirit": True}
            for t in local_random.sample(range(33, 43), 2)
        ]
        
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
        temp_snapshot_path = temp_dir / f"temp_sweep_snapshot_{seed}.json"
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
        "5000", # Higher turn budget to prevent preempting on exhaustive root
        "--validated-scorer",
    ]
    
    env = os.environ.copy()
    env["RAYON_NUM_THREADS"] = "2"
    env["HARMONIES_ROOT_BEAM"] = str(search_settings["root_beam"])
    env["HARMONIES_FUTURE_BEAM"] = "256"
    env["HARMONIES_FUTURE_BRANCH"] = "32"
    env["HARMONIES_FUTURE_DEPTH"] = "1" # depth 1 search as requested
    
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
        "avg_elapsed_sec": avg_elapsed
    }

def main():
    parser = argparse.ArgumentParser(description="Sweep root beam width")
    parser.add_argument("--games", type=int, default=10, help="Number of games per config")
    parser.add_argument("--workers", type=int, default=6, help="Concurrent workers")
    args = parser.parse_args()

    cli_path = Path("target/release/harmonies-cli.exe")
    if not cli_path.exists():
        cli_path = Path("target/release/harmonies-cli")
    if not cli_path.exists():
        print("Error: Compile binary first")
        sys.exit(1)

    beams = [256, 1024, 4096, 8192]
    seeds = list(range(101, 101 + args.games))
    summary = []

    print(f"Sweeping root beam values: {beams} over {args.games} games per value.\n")

    for rb in beams:
        settings = {"root_beam": rb}
        print(f"--- Running root_beam = {rb} ---")
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
            summary.append((rb, metrics))
            print(f"Result for root_beam={rb}: Mean VP = {metrics['mean_vp']:.2f} | Spirit Comp = {metrics['completion_rate']:.1f}% | Avg Game Time = {metrics['avg_elapsed_sec']:.1f}s\n")

    print("\n================ ROOT BEAM SWEEP RESULTS ================")
    print("| Root Beam | Mean VP | Min VP | Max VP | Animal VP | Spirit Comp % | Avg Game Time (s) |")
    print("|---|---|---|---|---|---|---|")
    for rb, m in summary:
        print(f"| **{rb}** | {m['mean_vp']:.2f} | {m['min_vp']} | {m['max_vp']} | {m['avg_animal']:.2f} | {m['completion_rate']:.1f}% | {m['avg_elapsed_sec']:.1f}s |")
    print("=========================================================")

if __name__ == "__main__":
    main()
