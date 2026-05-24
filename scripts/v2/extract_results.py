#!/usr/bin/env python3
"""Extract per-cell accuracy from v2 Inspect .eval logs.

Reads a Saltelli manifest (produced by mojave-gsa generate-manifest) and the
corresponding eval log directories, produces a results JSON with per-cell
accuracy and per-item response matrix. The output format is consumed by
mojave-gsa analyze.

Usage:
    python extract_results.py <manifest.json> <log_dir> <output.json>
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any

from inspect_ai.log import read_eval_log


def build_results_skeleton(eval_name: str, model: str) -> dict[str, Any]:
    return {
        "eval": eval_name,
        "model": model,
        "cells": [],
        "item_matrix": {},
    }


def extract(manifest_path: Path, log_dir: Path) -> dict[str, Any]:
    manifest = json.loads(manifest_path.read_text())
    eval_name = manifest["task"].split("/")[-1]
    results = build_results_skeleton(eval_name, manifest["model"])

    cells_by_id = {c["cell_id"]: c for c in manifest["cells"]}
    cell_dirs = sorted(d for d in log_dir.iterdir() if d.is_dir())
    total = len(cell_dirs)

    for i, cdir in enumerate(cell_dirs):
        cell_id = cdir.name
        eval_files = sorted(cdir.glob("*.eval"))
        if not eval_files:
            continue

        try:
            log = read_eval_log(str(eval_files[-1]))
        except Exception as e:
            print(f"  ERROR {cell_id}: {e}", file=sys.stderr)
            continue

        acc: float | None = None
        if log.results and log.results.scores:
            score = log.results.scores[0]
            acc_metric = score.metrics.get("accuracy")
            if acc_metric:
                acc = acc_metric.value

        cell_meta = cells_by_id.get(cell_id, {})
        cell_result: dict[str, Any] = {
            "cell_id": cell_id,
            "saltelli_index": cell_meta.get("saltelli_index"),
            "accuracy": acc,
            "prompt_template": cell_meta.get("prompt_template"),
            "system_prompt": cell_meta.get("system_prompt"),
            "n_shot_frac": cell_meta.get("n_shot_frac"),
            "choice_order": cell_meta.get("choice_order"),
            "decoding": cell_meta.get("decoding"),
            "quantization": cell_meta.get("quantization"),
            "n_samples": len(log.samples) if log.samples else 0,
        }
        results["cells"].append(cell_result)

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
                    results["item_matrix"][item_id][cell_id] = correct

        if (i + 1) % 500 == 0 or i + 1 == total:
            print(
                f"  {eval_name}: {i + 1}/{total} cells extracted",
                file=sys.stderr,
            )

    return results


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("manifest", type=Path)
    parser.add_argument("log_dir", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()

    results = extract(args.manifest, args.log_dir)

    accs = [c["accuracy"] for c in results["cells"] if c["accuracy"] is not None]
    print(f"\n  {results['eval']} summary:", file=sys.stderr)
    print(f"    Cells: {len(results['cells'])}", file=sys.stderr)
    print(f"    Items: {len(results['item_matrix'])}", file=sys.stderr)
    if accs:
        print(
            f"    Accuracy: mean={sum(accs) / len(accs):.4f} "
            f"min={min(accs):.4f} max={max(accs):.4f} "
            f"spread={max(accs) - min(accs):.4f}",
            file=sys.stderr,
        )

    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(results, indent=2) + "\n")
    print(f"    -> {args.output}", file=sys.stderr)


if __name__ == "__main__":
    main()
