#!/usr/bin/env python3
"""Generate variant manifests for any MCQ eval.

Usage:
    python gen_manifest.py <task_name> <output_path> [--n-orders N] [--temps T1,T2,...]

Examples:
    python gen_manifest.py inspect_evals/mmlu_0_shot manifests/mmlu.json
    python gen_manifest.py inspect_evals/hellaswag manifests/hellaswag.json \
        --n-orders 36 --temps 0.3,0.5,0.7,1.0
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path


def generate_variants(
    n_orders: int,
    temperatures: list[float],
) -> list[dict]:
    variants: list[dict] = []
    vid = 0

    # Baseline: temp 0.0, default order
    variants.append(
        {
            "variant_id": f"v{vid:03d}",
            "block": 0,
            "order_seed": 0,
            "temperature": 0.0,
        }
    )
    vid += 1

    # Full cross: orders x temperatures
    for temp in temperatures:
        for seed in range(1, n_orders + 1):
            variants.append(
                {
                    "variant_id": f"v{vid:03d}",
                    "block": 1,
                    "order_seed": seed,
                    "temperature": temp,
                }
            )
            vid += 1

    return variants


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("task", help="Inspect task name (e.g. inspect_evals/mmlu_0_shot)")
    parser.add_argument("output", help="Output manifest path")
    parser.add_argument("--n-orders", type=int, default=36)
    parser.add_argument("--temps", default="0.3,0.5,0.7,1.0")
    args = parser.parse_args()

    temps = [float(t) for t in args.temps.split(",")]
    variants = generate_variants(args.n_orders, temps)

    manifest = {
        "task": args.task,
        "model": "Qwen/Qwen2.5-7B-Instruct",
        "total_variants": len(variants),
        "design": {
            "n_orders": args.n_orders,
            "temperatures": temps,
            "block_0": "deterministic baseline",
            "block_1": f"{args.n_orders} orders x {len(temps)} temperatures",
        },
        "runs": variants,
    }

    output = Path(args.output)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(manifest, indent=2) + "\n")
    print(f"Generated {len(variants)} variants for {args.task} -> {output}")


if __name__ == "__main__":
    main()
