#!/usr/bin/env python3
"""Generate 217 variant specifications for the ARC-Challenge measurement workup.

Partially crossed design:
  Block 1: 24 option orders x 5 temperatures = 120 (prompt = default)
  Block 2: 24 option orders x 4 prompts = 96 (temperature = 0.7)
  Block 3: 1 baseline (temp 0.0, default order, default prompt)
  Total: 217
"""

from __future__ import annotations

import itertools
import json
import sys
from pathlib import Path

OPTION_ORDERS: list[tuple[int, ...]] = list(itertools.permutations(range(4)))
TEMPERATURES: list[float] = [0.3, 0.5, 0.7, 1.0, 1.5]
PROMPT_TEMPLATES: list[str] = ["default", "minimal", "explicit_reasoning", "answer_only"]


def generate_variants() -> list[dict]:
    variants: list[dict] = []
    vid = 0

    # Block 3: deterministic baseline
    variants.append(
        {
            "variant_id": f"v{vid:03d}",
            "block": 3,
            "order_permutation_index": 0,
            "order_seed": 0,
            "temperature": 0.0,
            "prompt_template": "default",
        }
    )
    vid += 1

    # Block 1: order x temperature (prompt = default)
    for temp in TEMPERATURES:
        for perm_idx, _perm in enumerate(OPTION_ORDERS):
            variants.append(
                {
                    "variant_id": f"v{vid:03d}",
                    "block": 1,
                    "order_permutation_index": perm_idx,
                    "order_seed": perm_idx + 1,
                    "temperature": temp,
                    "prompt_template": "default",
                }
            )
            vid += 1

    # Block 2: order x prompt (temperature = 0.7)
    for prompt in PROMPT_TEMPLATES:
        for perm_idx, _perm in enumerate(OPTION_ORDERS):
            variants.append(
                {
                    "variant_id": f"v{vid:03d}",
                    "block": 2,
                    "order_permutation_index": perm_idx,
                    "order_seed": perm_idx + 1,
                    "temperature": 0.7,
                    "prompt_template": prompt,
                }
            )
            vid += 1

    return variants


def main() -> None:
    output = Path(sys.argv[1]) if len(sys.argv) > 1 else Path("scripts/arc-workup/manifest.json")
    variants = generate_variants()
    manifest = {
        "benchmark": "arc_challenge",
        "model": "Qwen/Qwen2.5-7B-Instruct",
        "total_variants": len(variants),
        "design": {
            "block_1": "order x temperature (prompt=default)",
            "block_2": "order x prompt (temperature=0.7)",
            "block_3": "deterministic baseline",
        },
        "runs": variants,
    }
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(manifest, indent=2) + "\n")
    print(f"Generated {len(variants)} variant specs -> {output}", file=sys.stderr)


if __name__ == "__main__":
    main()
