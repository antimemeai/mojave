#!/usr/bin/env python3
"""Full mojave analysis of extracted eval results.

Computes per-eval:
  1. Aggregate accuracy + Wilson CIs (overall and per-temperature)
  2. Per-item perturbation stability (fraction correct across variants)
  3. Sensitive vs stable item classification
  4. Temperature effect decomposition
  5. Retrospective sequential stopping via permutation (Python fallback)

Usage:
    python analyze_results.py
"""

from __future__ import annotations

import json
import math
import random
import sys
from collections import defaultdict
from pathlib import Path
from typing import Any

RESULTS_FILES = {
    "arc_challenge": "data/arc-workup/results.json",
    "cybermetric_2000": "data/cybermetric/results.json",
    "mmlu_0_shot": "data/mmlu/results.json",
    "hellaswag": "data/hellaswag/results.json",
    "truthfulqa": "data/truthfulqa/results.json",
    "gsm8k": "data/gsm8k/results.json",
}

ANALYSIS_DIR = Path("data/analysis")


def wilson_ci(k: int, n: int, z: float = 1.96) -> tuple[float, float, float]:
    """Wilson score interval for binomial proportion."""
    if n == 0:
        return 0.0, 0.0, 0.0
    p_hat = k / n
    denom = 1 + z * z / n
    center = (p_hat + z * z / (2 * n)) / denom
    margin = z * math.sqrt(p_hat * (1 - p_hat) / n + z * z / (4 * n * n)) / denom
    return p_hat, max(0.0, center - margin), min(1.0, center + margin)


def bernoulli_msprt_log_lr(
    successes: int, n: int, p0: float, beta_a: float = 1.0, beta_b: float = 1.0
) -> float:
    """Bernoulli mSPRT log-likelihood ratio with Beta mixing (Johari 2022)."""
    k = successes
    from math import lgamma

    log_null = k * math.log(p0) + (n - k) * math.log(1 - p0) if 0 < p0 < 1 else 0
    log_alt = (
        lgamma(beta_a + beta_b)
        - lgamma(beta_a)
        - lgamma(beta_b)
        + lgamma(beta_a + k)
        + lgamma(beta_b + n - k)
        - lgamma(beta_a + beta_b + n)
    )
    return log_alt - log_null


def permutation_stopping_times(
    outcomes: list[int],
    p0: float,
    alpha: float = 0.05,
    n_perms: int = 1000,
    seed: int = 42,
) -> dict:
    """Retrospective sequential stopping analysis via permutation."""
    threshold = math.log(1.0 / alpha)
    n = len(outcomes)
    rng = random.Random(seed)

    stop_times = []
    for _ in range(n_perms):
        perm = outcomes[:]
        rng.shuffle(perm)
        successes = 0
        stopped = False
        for t in range(1, n + 1):
            successes += perm[t - 1]
            log_lr = bernoulli_msprt_log_lr(successes, t, p0)
            if log_lr >= threshold:
                stop_times.append(t)
                stopped = True
                break
        if not stopped:
            stop_times.append(n)

    stop_times.sort()
    median_stop = stop_times[len(stop_times) // 2]
    q25 = stop_times[len(stop_times) // 4]
    q75 = stop_times[3 * len(stop_times) // 4]
    frac_quarter = sum(1 for t in stop_times if t <= n // 4) / len(stop_times)
    frac_half = sum(1 for t in stop_times if t <= n // 2) / len(stop_times)
    frac_full = sum(1 for t in stop_times if t < n) / len(stop_times)

    return {
        "n_items": n,
        "p0": p0,
        "alpha": alpha,
        "n_permutations": n_perms,
        "median_stop": median_stop,
        "iqr": [q25, q75],
        "frac_stopped_by_quarter": round(frac_quarter, 3),
        "frac_stopped_by_half": round(frac_half, 3),
        "frac_stopped_by_end": round(frac_full, 3),
    }


def analyze_eval(name: str, data: dict[str, Any]) -> dict[str, Any]:
    print(f"\n  Analyzing: {name}", file=sys.stderr)

    variants = data["variants"]
    item_matrix = data["item_matrix"]

    accs = [v["accuracy"] for v in variants if v["accuracy"] is not None]
    n_variants = len(accs)
    n_items = len(item_matrix)

    # --- 1. Aggregate accuracy + Wilson CIs ---
    # Overall: pool all item responses across all variants
    all_correct = 0
    all_total = 0
    for _item_id, responses in item_matrix.items():
        for _vid, correct in responses.items():
            all_correct += int(correct)
            all_total += 1

    overall_acc, overall_lo, overall_hi = wilson_ci(all_correct, all_total)

    # Per-temperature breakdown
    temp_groups: dict[float, list[float]] = defaultdict(list)
    for v in variants:
        if v["accuracy"] is not None and v["temperature"] is not None:
            temp_groups[v["temperature"]].append(v["accuracy"])

    temp_summary = {}
    for temp in sorted(temp_groups.keys()):
        vals = temp_groups[temp]
        mean_acc = sum(vals) / len(vals)
        sd = (sum((x - mean_acc) ** 2 for x in vals) / max(1, len(vals) - 1)) ** 0.5
        temp_summary[str(temp)] = {
            "n_variants": len(vals),
            "mean_accuracy": round(mean_acc, 6),
            "sd": round(sd, 6),
            "min": round(min(vals), 6),
            "max": round(max(vals), 6),
        }

    # --- 2. Per-item perturbation stability ---
    item_stats = []
    for item_id, responses in item_matrix.items():
        vals = list(responses.values())
        n_resp = len(vals)
        frac_correct = sum(vals) / n_resp if n_resp > 0 else 0
        item_stats.append(
            {
                "item_id": item_id,
                "frac_correct": frac_correct,
                "n_variants": n_resp,
            }
        )

    # --- 3. Stable vs sensitive classification ---
    stable_items = [
        it for it in item_stats if it["frac_correct"] >= 0.9 or it["frac_correct"] <= 0.1
    ]
    sensitive_items = [it for it in item_stats if 0.25 <= it["frac_correct"] <= 0.75]
    floor_items = [it for it in item_stats if it["frac_correct"] <= 0.1]
    ceiling_items = [it for it in item_stats if it["frac_correct"] >= 0.9]

    # Histogram of frac_correct (10 bins from 0 to 1)
    hist_bins = [0] * 10
    for it in item_stats:
        bin_idx = min(9, int(it["frac_correct"] * 10))
        hist_bins[bin_idx] += 1

    # --- 4. Temperature effect ---
    # For each item, compute accuracy at each temperature level
    temp_item_accs: dict[str, dict[str, float | None]] = {}
    variant_temp_map: dict[str, float] = {
        v["variant_id"]: v["temperature"] for v in variants if v["temperature"] is not None
    }

    for item_id, responses in item_matrix.items():
        by_temp: dict[float, list[float]] = defaultdict(list)
        for vid, correct in responses.items():
            item_temp: float | None = variant_temp_map.get(vid)
            if item_temp is not None:
                by_temp[item_temp].append(correct)
        temp_item_accs[item_id] = {
            str(t): sum(vals) / len(vals) if vals else None for t, vals in by_temp.items()
        }

    # Temperature sensitivity: items where accuracy changes > 0.2 across temps
    temp_sensitive = 0
    for _item_id, taccs in temp_item_accs.items():
        valid = [v for v in taccs.values() if v is not None]
        if len(valid) >= 2 and (max(valid) - min(valid)) > 0.2:
            temp_sensitive += 1

    # --- 5. Retrospective sequential stopping ---
    # Use baseline variant's per-item outcomes
    baseline_outcomes = []
    baseline_vid = "v000"
    if baseline_vid in next(iter(item_matrix.values()), {}):
        for item_id in sorted(item_matrix.keys()):
            if baseline_vid in item_matrix[item_id]:
                baseline_outcomes.append(int(item_matrix[item_id][baseline_vid]))

    stopping = None
    if len(baseline_outcomes) >= 50:
        p0 = 0.5
        stopping = permutation_stopping_times(baseline_outcomes, p0=p0)
        print(
            f"    Sequential stopping: median={stopping['median_stop']}/{stopping['n_items']}, "
            f"frac@half={stopping['frac_stopped_by_half']}",
            file=sys.stderr,
        )

    # --- Compile analysis ---
    analysis = {
        "eval": name,
        "model": data["model"],
        "n_variants": n_variants,
        "n_items": n_items,
        "aggregate": {
            "pooled_accuracy": round(overall_acc, 6),
            "pooled_n": all_total,
            "wilson_ci_95": [round(overall_lo, 6), round(overall_hi, 6)],
            "variant_mean": round(sum(accs) / len(accs), 6) if accs else None,
            "variant_sd": round(
                (sum((x - sum(accs) / len(accs)) ** 2 for x in accs) / max(1, len(accs) - 1))
                ** 0.5,
                6,
            )
            if len(accs) > 1
            else None,
            "variant_min": round(min(accs), 6) if accs else None,
            "variant_max": round(max(accs), 6) if accs else None,
            "variant_spread": round(max(accs) - min(accs), 6) if accs else None,
        },
        "by_temperature": temp_summary,
        "perturbation_stability": {
            "stability_histogram": hist_bins,
            "n_stable": len(stable_items),
            "n_sensitive": len(sensitive_items),
            "n_floor": len(floor_items),
            "n_ceiling": len(ceiling_items),
            "n_temp_sensitive": temp_sensitive,
            "pct_sensitive": round(len(sensitive_items) / max(1, n_items) * 100, 1),
        },
        "sequential_stopping": stopping,
    }

    return analysis


def main() -> None:
    ANALYSIS_DIR.mkdir(parents=True, exist_ok=True)
    all_analyses = {}

    for name, path in RESULTS_FILES.items():
        p = Path(path)
        if not p.exists():
            print(f"SKIP {name}: {path} not found", file=sys.stderr)
            continue

        data = json.loads(p.read_text())
        analysis = analyze_eval(name, data)
        all_analyses[name] = analysis

        per_eval_path = ANALYSIS_DIR / f"{name}_analysis.json"
        per_eval_path.write_text(json.dumps(analysis, indent=2) + "\n")
        print(f"    -> {per_eval_path}", file=sys.stderr)

    # Write combined summary
    summary_path = ANALYSIS_DIR / "full_summary.json"
    summary_path.write_text(json.dumps(all_analyses, indent=2) + "\n")
    print(f"\nFull summary -> {summary_path}", file=sys.stderr)

    # Print human-readable summary
    print("\n" + "=" * 70, file=sys.stderr)
    print("  MOJAVE MEASUREMENT WORKUP — Qwen/Qwen2.5-7B-Instruct", file=sys.stderr)
    print("=" * 70, file=sys.stderr)

    for name, a in all_analyses.items():
        agg = a["aggregate"]
        stab = a["perturbation_stability"]
        seq = a["sequential_stopping"]
        print(f"\n  {name}", file=sys.stderr)
        print(f"  {'─' * 50}", file=sys.stderr)
        print(f"  Items: {a['n_items']}  Variants: {a['n_variants']}", file=sys.stderr)
        print(
            f"  Pooled accuracy: {agg['pooled_accuracy']:.4f}  "
            f"95% CI [{agg['wilson_ci_95'][0]:.4f}, {agg['wilson_ci_95'][1]:.4f}]",
            file=sys.stderr,
        )
        print(
            f"  Variant spread:  {agg['variant_min']:.4f} - {agg['variant_max']:.4f}  "
            f"(range: {agg['variant_spread']:.4f})",
            file=sys.stderr,
        )
        print(f"  Stability hist:  {stab['stability_histogram']}", file=sys.stderr)
        print(
            f"  Sensitive items: {stab['n_sensitive']}/{a['n_items']} ({stab['pct_sensitive']}%)",
            file=sys.stderr,
        )
        print(f"  Temp-sensitive:  {stab['n_temp_sensitive']}", file=sys.stderr)
        if seq:
            print(
                f"  Seq stopping:    median {seq['median_stop']}/{seq['n_items']}  "
                f"IQR [{seq['iqr'][0]}, {seq['iqr'][1]}]  "
                f"@half: {seq['frac_stopped_by_half']:.0%}",
                file=sys.stderr,
            )

        # Per-temperature breakdown
        print("  By temperature:", file=sys.stderr)
        for temp, ts in a["by_temperature"].items():
            print(
                f"    T={temp}: mean={ts['mean_accuracy']:.4f} "
                f"SD={ts['sd']:.4f} [{ts['min']:.4f}, {ts['max']:.4f}] "
                f"(n={ts['n_variants']})",
                file=sys.stderr,
            )


if __name__ == "__main__":
    main()
