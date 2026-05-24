#!/usr/bin/env python3
"""Generate full_items.json and pre-sampled subsets for MCQ evals.

Supports: wmdp_bio, wmdp_chem, truthfulqa_mc1

Usage:
    python generate_subsets.py wmdp_chem --output-dir data/v2/wmdp_chem
    python generate_subsets.py truthfulqa_mc1 --output-dir data/v2/truthfulqa_mc1
"""

from __future__ import annotations

import argparse
import json
import random
from pathlib import Path

from datasets import load_dataset


def load_wmdp(subset: str) -> list[dict]:
    ds = load_dataset("cais/wmdp", subset, split="test")
    items = []
    for i, row in enumerate(ds):
        items.append(
            {
                "id": f"wmdp_{subset.replace('wmdp-', '')}_{i:04d}",
                "question": row["question"],
                "choices": row["choices"],
                "answer": row["answer"],
            }
        )
    return items


def load_truthfulqa_mc1() -> list[dict]:
    ds = load_dataset("truthful_qa", "multiple_choice", split="validation")
    items = []
    for i, row in enumerate(ds):
        choices = row["mc1_targets"]["choices"]
        labels = row["mc1_targets"]["labels"]
        answer = labels.index(1)
        items.append(
            {
                "id": f"truthfulqa_mc1_{i:04d}",
                "question": row["question"],
                "choices": choices,
                "answer": answer,
            }
        )
    return items


LOADERS = {
    "wmdp_bio": lambda: load_wmdp("wmdp-bio"),
    "wmdp_chem": lambda: load_wmdp("wmdp-chem"),
    "truthfulqa_mc1": load_truthfulqa_mc1,
}


def generate_subsets(
    items: list[dict],
    eval_name: str,
    output_dir: Path,
    n_subsets: int = 25,
    subset_size: int = 100,
    base_seed: int = 20260524,
) -> None:
    output_dir.mkdir(parents=True, exist_ok=True)

    full = {
        "eval": eval_name,
        "n_items": len(items),
        "items": items,
    }
    (output_dir / "full_items.json").write_text(json.dumps(full, indent=2))
    print(f"Wrote full_items.json: {len(items)} items")

    for idx in range(n_subsets):
        rng = random.Random(base_seed + idx)
        sampled = rng.sample(items, min(subset_size, len(items)))
        subset = {
            "eval": eval_name,
            "subset_index": idx,
            "subset_size": len(sampled),
            "seed": base_seed + idx,
            "source_n_items": len(items),
            "items": sampled,
        }
        fname = f"subset_{idx:02d}.json"
        (output_dir / fname).write_text(json.dumps(subset, indent=2))

    print(f"Wrote {n_subsets} subsets of {subset_size} items each")


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("eval", choices=list(LOADERS.keys()))
    parser.add_argument("--output-dir", type=Path, required=True)
    parser.add_argument("--n-subsets", type=int, default=25)
    parser.add_argument("--subset-size", type=int, default=100)
    parser.add_argument("--base-seed", type=int, default=20260524)
    args = parser.parse_args()

    items = LOADERS[args.eval]()
    print(f"Loaded {len(items)} items for {args.eval}")
    generate_subsets(
        items,
        args.eval,
        args.output_dir,
        args.n_subsets,
        args.subset_size,
        args.base_seed,
    )


if __name__ == "__main__":
    main()
