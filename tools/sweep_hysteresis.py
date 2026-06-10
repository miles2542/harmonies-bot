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

def run_single_game(cli_path, weights_path, seed, search_settings, config):
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
        temp_snapshot_path = temp_dir / f"temp_hysteresis_snapshot_{seed}.json"
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
        "2000",
        "--validated-scorer",
    ]
    
    env = os.environ.copy()
    env["RAYON_NUM_THREADS"] = "2"
    env["HARMONIES_ROOT_BEAM"] = str(search_settings["root_beam"])
    env["HARMONIES_FUTURE_BEAM"] = str(search_settings["future_beam"])
    env["HARMONIES_FUTURE_BRANCH"] = str(search_settings["future_branch"])
    env["HARMONIES_FUTURE_DEPTH"] = str(search_settings["future_depth"])
    
    env["HARMONIES_HEURISTIC_MODE"] = "dynamic_demand_tuned"
    env["HARMONIES_FORCE_SPIRIT_LIMIT"] = "5.0"
    env["HARMONIES_SPIRIT_PROX_MULT"] = "2.5"
    env["HARMONIES_SPIRIT_PENALTY_COEFF"] = "-4.0"
    env["HARMONIES_COMPENSATION_COEFF"] = "-0.15" # Locked-in best Extreme Comp
    
    env["HARMONIES_COMMIT_WEIGHT"] = str(config["commit_weight"])
    env["HARMONIES_CLOG_WEIGHT"] = str(config["clog_weight"])
    
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
    tree_vp = []
    mountain_vp = []
    field_vp = []
    building_vp = []
    water_vp = []
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
            tree_vp.append(bk.get("trees", 0))
            mountain_vp.append(bk.get("mountains", 0))
            field_vp.append(bk.get("fields", 0))
            building_vp.append(bk.get("buildings", 0))
            water_vp.append(bk.get("water", 0))
            
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
    avg_tree = sum(tree_vp) / n_runs
    avg_mountain = sum(mountain_vp) / n_runs
    avg_field = sum(field_vp) / n_runs
    avg_building = sum(building_vp) / n_runs
    avg_water = sum(water_vp) / n_runs
    completion_rate = (completed_spirits / n_runs) * 100
    avg_elapsed = total_elapsed / successful
    
    return {
        "mean_vp": mean_vp,
        "min_vp": min_vp,
        "max_vp": max_vp,
        "avg_animal": avg_animal,
        "avg_tree": avg_tree,
        "avg_mountain": avg_mountain,
        "avg_field": avg_field,
        "avg_building": avg_building,
        "avg_water": avg_water,
        "completion_rate": completion_rate,
        "avg_elapsed_sec": avg_elapsed
    }

def main():
    parser = argparse.ArgumentParser(description="Sweep Hysteresis and Diversification Penalty under Force 5")
    parser.add_argument("--games", type=int, default=20, help="Games per config")
    parser.add_argument("--workers", type=int, default=6, help="Concurrent workers")
    args = parser.parse_args()

    cli_path = Path("target/release/harmonies-cli.exe")
    if not cli_path.exists():
        cli_path = Path("target/release/harmonies-cli")
    if not cli_path.exists():
        print("Error: Compile binary first")
        sys.exit(1)

    settings = {
        "root_beam": 1024,
        "future_beam": 256,
        "future_branch": 32,
        "future_depth": 1
    }

    scenarios = [
        ("Control (Commit 0.0, Clog 0.0)", {"commit_weight": 0.0, "clog_weight": 0.0}),
        ("Light (Commit 5.0, Clog 10.0)", {"commit_weight": 5.0, "clog_weight": 10.0}),
        ("Medium (Commit 15.0, Clog 25.0)", {"commit_weight": 15.0, "clog_weight": 25.0}),
        ("Strong (Commit 30.0, Clog 50.0)", {"commit_weight": 30.0, "clog_weight": 50.0}),
        ("Bonus Only (Commit 15.0, Clog 0.0)", {"commit_weight": 15.0, "clog_weight": 0.0}),
        ("Penalty Only (Commit 0.0, Clog 25.0)", {"commit_weight": 0.0, "clog_weight": 25.0})
    ]
    
    seeds = list(range(101, 101 + args.games))
    summary = []

    print(f"Sweeping Hysteresis under Force 5 limit over {args.games} games per config.\n")

    for name, config in scenarios:
        print(f"--- Running Scenario: {name} ---")
        results = []
        with ThreadPoolExecutor(max_workers=args.workers) as executor:
            futures = {
                executor.submit(run_single_game, cli_path, DEFAULT_WEIGHTS, seed, settings, config): seed
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
            summary.append((name, metrics))
            print(f"Result for {name}: Mean VP = {metrics['mean_vp']:.2f} | Spirit Comp = {metrics['completion_rate']:.1f}% | Avg Game Time = {metrics['avg_elapsed_sec']:.1f}s\n")

    print("\n================ HYSTERESIS SWEEP RESULTS ================")
    print("| Scenario | Mean VP | Min VP | Max VP | Animals | Trees | Mountains | Fields | Buildings | Water | Spirit Comp % | Avg Game Time (s) |")
    print("|---|---|---|---|---|---|---|---|---|---|---|---|")
    for name, m in summary:
        print(f"| **{name}** | {m['mean_vp']:.2f} | {m['min_vp']} | {m['max_vp']} | {m['avg_animal']:.2f} | {m['avg_tree']:.2f} | {m['avg_mountain']:.2f} | {m['avg_field']:.2f} | {m['avg_building']:.2f} | {m['avg_water']:.2f} | {m['completion_rate']:.1f}% | {m['avg_elapsed_sec']:.1f}s |")
    print("==========================================================")

if __name__ == "__main__":
    main()
