#!/usr/bin/env python3
"""Generate and seal the v2 pre-registration into the audit chain.

Loads the WMDP dataset, draws the exemplar pool, serializes pool metadata,
creates the pre-registration JSON, and emits an audit event to seal it.

Must be run BEFORE the first eval cell. The audit chain integrity check
verifies this ordering.

Usage:
    python seal_pre_registration.py --task inspect_evals/wmdp_chem \
        --output scripts/v2/pre_registration.json
"""

from __future__ import annotations

import argparse
import hashlib
import json
import sys
import time
from pathlib import Path
from typing import Any

sys.path.insert(0, str(Path(__file__).parent.parent))

from audit_emit import emit as audit
from inspect_ai._util.registry import registry_lookup
from nshot import N_SHOT_FRACS, draw_exemplar_pool, serialize_pool


def build_pre_registration(
    task_name: str,
    pool_metadata: dict[str, Any],
    dataset_size: int,
) -> dict[str, Any]:
    eval_name = task_name.split("/")[-1]
    return {
        "study_id": "mojave-v2-wmdp",
        "date": time.strftime("%Y-%m-%d"),
        "task": task_name,
        "eval_name": eval_name,
        "models": [
            "Qwen/Qwen2.5-7B-Instruct",
            "Qwen/Qwen2.5-72B-Instruct",
        ],
        "hypothesis": {
            "H1": (
                f"{eval_name} exhibits perturbation-induced accuracy spread "
                f">= 30 percentage points across the Saltelli matrix for "
                f"at least one model"
            ),
            "direction": "one-sided",
            "threshold_pp": 30,
            "alpha": 0.05,
            "rationale": (
                "v1 found 70-92pp spread on MMLU/ARC/HellaSwag/"
                "CyberMetric/TruthfulQA; WMDP uses the same MCQ format"
            ),
        },
        "stopping_rule": {
            "method": "confseq_betting",
            "half_width_threshold": 0.02,
            "description": (
                "Stop a cell when the betting-based confidence sequence "
                "half-width is below 0.02 (per spec section 1.C)"
            ),
        },
        "perturbation_design": {
            "method": "saltelli_radial",
            "implementation": "salib-rs via mojave-gsa binary (NOT Python SALib)",
            "N_base": 1024,
            "k": 6,
            "total_cells_per_model": 8192,
            "axes": {
                "A1_prompt_template": [
                    "lm-eval-default",
                    "bare",
                    "cot",
                    "letter-only",
                    "verbose-rationale",
                ],
                "A2_system_prompt": [
                    "none",
                    "helpful",
                    "domain-expert",
                    "safety-aware",
                ],
                "A3_n_shot_frac": [0.0, 0.01, 0.025, 0.05],
                "A4_choice_order": ["original", "shuffled"],
                "A5_decoding": ["greedy", "T=0.7", "T=1.0"],
                "A6_quantization": ["bf16", "fp8"],
            },
        },
        "exemplar_pool": pool_metadata,
        "dataset": {
            "name": task_name,
            "size": dataset_size,
        },
    }


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--task",
        default="inspect_evals/wmdp_chem",
        help="Inspect task name",
    )
    parser.add_argument(
        "--output",
        type=Path,
        default=Path("scripts/v2/pre_registration.json"),
    )
    args = parser.parse_args()

    print(f"Loading {args.task}...", file=sys.stderr)
    base_fn = registry_lookup("task", args.task)
    base = base_fn()  # type: ignore[operator]
    all_samples = list(base.dataset)
    dataset_size = len(all_samples)
    print(f"  Dataset size: {dataset_size}", file=sys.stderr)

    print("Drawing exemplar pool...", file=sys.stderr)
    pool = draw_exemplar_pool(all_samples, max_frac=max(N_SHOT_FRACS))
    pool_meta = serialize_pool(pool, dataset_size)
    print(
        f"  Pool size: {pool_meta['max_pool_size']}, SHA256: {pool_meta['pool_sha256'][:16]}...",
        file=sys.stderr,
    )

    pre_reg = build_pre_registration(args.task, pool_meta, dataset_size)

    args.output.parent.mkdir(parents=True, exist_ok=True)
    pre_reg_text = json.dumps(pre_reg, indent=2) + "\n"
    args.output.write_text(pre_reg_text)
    print(f"  Written to: {args.output}", file=sys.stderr)

    pre_reg_hash = hashlib.sha256(pre_reg_text.encode()).hexdigest()

    print("Sealing into audit chain...", file=sys.stderr)
    audit(
        "run_card.sealed",
        resource_kind="pre_registration",
        resource_id=pre_reg["study_id"],
        detail={
            "kind": "pre_registration",
            "task": args.task,
            "pre_registration_sha256": pre_reg_hash,
            "exemplar_pool_sha256": pool_meta["pool_sha256"],
            "dataset_size": dataset_size,
            "n_shot_fracs": N_SHOT_FRACS,
            "exemplar_pool_size": pool_meta["max_pool_size"],
        },
    )
    print("  Sealed.", file=sys.stderr)


if __name__ == "__main__":
    main()
