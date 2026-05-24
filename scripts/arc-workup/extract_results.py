#!/usr/bin/env python3
"""Extract per-variant accuracy from all Inspect .eval logs.

Produces one JSON file per eval with per-variant and per-item results.

Usage:
    python extract_results.py
"""

from __future__ import annotations

import json
import sys
from pathlib import Path
from typing import Any

from inspect_ai.log import read_eval_log

EVALS = {
    "arc_challenge": {
        "log_dir": "data/destructive/arc/logs",
        "manifest": "data/destructive/arc/manifest.json",
        "output": "data/destructive/arc/results.json",
    },
    "cybermetric_2000": {
        "log_dir": "data/destructive/cybermetric/logs",
        "manifest": "data/destructive/cybermetric/manifest.json",
        "output": "data/destructive/cybermetric/results.json",
    },
    "mmlu_0_shot": {
        "log_dir": "data/destructive/mmlu/logs",
        "manifest": "data/destructive/mmlu/manifest.json",
        "output": "data/destructive/mmlu/results.json",
    },
    "hellaswag": {
        "log_dir": "data/destructive/hellaswag/logs",
        "manifest": "data/destructive/hellaswag/manifest.json",
        "output": "data/destructive/hellaswag/results.json",
    },
    "truthfulqa": {
        "log_dir": "data/destructive/truthfulqa/logs",
        "manifest": "data/destructive/truthfulqa/manifest.json",
        "output": "data/destructive/truthfulqa/results.json",
    },
    "wmdp_chem": {
        "log_dir": "data/destructive/wmdp_chem/logs",
        "manifest": "data/destructive/wmdp_chem/manifest.json",
        "output": "data/destructive/wmdp_chem/results.json",
    },
}


def extract_eval(name: str, cfg: dict[str, str]) -> dict[str, Any] | None:
    log_dir = Path(cfg["log_dir"])
    if not log_dir.exists():
        print(f"  SKIP {name}: {log_dir} not found", file=sys.stderr)
        return None

    manifest = json.loads(Path(cfg["manifest"]).read_text())
    variants = {v["variant_id"]: v for v in manifest["runs"]}

    results: dict[str, Any] = {
        "eval": name,
        "model": "Qwen/Qwen2.5-7B-Instruct",
        "variants": [],
        "item_matrix": {},
    }

    variant_dirs = sorted(d for d in log_dir.iterdir() if d.is_dir())
    total = len(variant_dirs)

    for i, vdir in enumerate(variant_dirs):
        vid = vdir.name
        eval_files = sorted(vdir.glob("*.eval"))
        if not eval_files:
            continue

        try:
            log = read_eval_log(str(eval_files[-1]))
        except Exception as e:
            print(f"  ERROR {vid}: {e}", file=sys.stderr)
            continue

        acc: float | None = None
        stderr_val: float | None = None
        if log.results and log.results.scores:
            score = log.results.scores[0]
            acc_metric = score.metrics.get("accuracy")
            if acc_metric:
                acc = acc_metric.value
                stderr_metric = score.metrics.get("stderr", None)
                if stderr_metric:
                    stderr_val = stderr_metric.value

        var_meta = variants.get(vid, {})
        temp = var_meta.get("temperature", None)
        order_seed = var_meta.get("order_seed", None)

        variant_result = {
            "variant_id": vid,
            "accuracy": acc,
            "stderr": stderr_val,
            "temperature": temp,
            "order_seed": order_seed,
            "n_samples": len(log.samples) if log.samples else 0,
        }
        results["variants"].append(variant_result)

        if log.samples:
            for sample in log.samples:
                item_id = str(sample.id)
                correct: float | None = None

                if sample.scores and isinstance(sample.scores, dict):
                    for score_val in sample.scores.values():
                        if hasattr(score_val, "value"):
                            v = score_val.value
                        elif isinstance(score_val, dict):
                            v = score_val.get("value")
                        else:
                            continue
                        correct = 1.0 if v == "C" or v == 1 else 0.0

                if correct is not None:
                    if item_id not in results["item_matrix"]:
                        results["item_matrix"][item_id] = {}
                    results["item_matrix"][item_id][vid] = correct

        if (i + 1) % 25 == 0 or i + 1 == total:
            print(f"  {name}: {i + 1}/{total} variants extracted", file=sys.stderr)

    return results


def main() -> None:
    for name, cfg in EVALS.items():
        print(f"\n{'=' * 60}", file=sys.stderr)
        print(f"  Extracting: {name}", file=sys.stderr)
        print(f"{'=' * 60}", file=sys.stderr)

        results = extract_eval(name, cfg)
        if results is None:
            continue

        n_variants = len(results["variants"])
        n_items = len(results["item_matrix"])
        accs = [v["accuracy"] for v in results["variants"] if v["accuracy"] is not None]

        print(f"\n  {name} summary:", file=sys.stderr)
        print(f"    Variants: {n_variants}", file=sys.stderr)
        print(f"    Items in matrix: {n_items}", file=sys.stderr)
        if accs:
            print(
                f"    Accuracy: mean={sum(accs) / len(accs):.4f} "
                f"min={min(accs):.4f} max={max(accs):.4f} "
                f"spread={max(accs) - min(accs):.4f}",
                file=sys.stderr,
            )

        output_path = Path(cfg["output"])
        output_path.write_text(json.dumps(results, indent=2) + "\n")
        print(f"    Written to: {output_path}", file=sys.stderr)


if __name__ == "__main__":
    main()
