#!/usr/bin/env python3
"""Generate variant manifests for destructive perturbation runs.

Partially-crossed block design that goes beyond cosmetic (option-order x
temperature) to test perturbations that *should not* change the answer but
often do: prompt templates, system prompts, chain-of-thought, few-shot
exemplars, and label formatting.

Usage:
    python gen_destructive_manifest.py <task_name> <output_path>
    python gen_destructive_manifest.py inspect_evals/arc_challenge \
        data/destructive/arc/manifest.json
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path

PROMPT_TEMPLATES = {
    "default": None,
    "cot": "__COT__",
    "bare": ("{question}\n\n{choices}\n\nRespond with only the letter."),
    "academic": (
        "Consider the following question carefully.\n\n"
        "{question}\n\n{choices}\n\n"
        "Select the best answer. The entire content of your response "
        "should be of the following format: 'ANSWER: $LETTER' "
        "(without quotes) where LETTER is one of {letters}."
    ),
    "aggressive": (
        "QUESTION:\n{question}\n\nOPTIONS:\n{choices}\n\n"
        "Pick exactly one. Reply with ONLY the letter, nothing else."
    ),
}

SYSTEM_PROMPTS = {
    "none": "",
    "helpful": "You are a helpful assistant.",
    "concise": "Answer concisely and directly.",
    "expert": "You are an expert test-taker. Read each question carefully before answering.",
    "cautious": "If you are unsure, reason through the options before committing to an answer.",
}

FEW_SHOT_LEVELS = [0, 1, 3, 5]

LABEL_FORMATS = {
    "letter_paren": "A)",
    "letter_dot": "A.",
    "number": "1)",
    "roman": "I.",
    "lower": "a)",
}

TEMPERATURES = [0.0, 0.3, 0.7, 1.0]

ORDER_SEEDS = list(range(1, 13))


def generate_variants() -> list[dict]:
    variants: list[dict] = []
    vid = 0

    # Block 0: deterministic baseline
    variants.append(
        {
            "variant_id": f"v{vid:03d}",
            "block": 0,
            "description": "deterministic baseline",
            "temperature": 0.0,
            "order_seed": 0,
            "prompt_template": "default",
            "system_prompt": "none",
            "few_shot": 0,
            "label_format": "letter_paren",
        }
    )
    vid += 1

    # Block 1: prompt template x temperature (seed=1, system=none)
    for tmpl_name in PROMPT_TEMPLATES:
        for temp in TEMPERATURES:
            variants.append(
                {
                    "variant_id": f"v{vid:03d}",
                    "block": 1,
                    "description": f"prompt={tmpl_name}, T={temp}",
                    "temperature": temp,
                    "order_seed": 1,
                    "prompt_template": tmpl_name,
                    "system_prompt": "none",
                    "few_shot": 0,
                    "label_format": "letter_paren",
                }
            )
            vid += 1

    # Block 2: system prompt x prompt template (T=0.7, seed=1)
    for sys_name in SYSTEM_PROMPTS:
        for tmpl_name in PROMPT_TEMPLATES:
            variants.append(
                {
                    "variant_id": f"v{vid:03d}",
                    "block": 2,
                    "description": f"sys={sys_name}, prompt={tmpl_name}",
                    "temperature": 0.7,
                    "order_seed": 1,
                    "prompt_template": tmpl_name,
                    "system_prompt": sys_name,
                    "few_shot": 0,
                    "label_format": "letter_paren",
                }
            )
            vid += 1

    # Block 3: few-shot x temperature (seed=1, default prompt, no system)
    for n_shot in FEW_SHOT_LEVELS:
        for temp in TEMPERATURES:
            variants.append(
                {
                    "variant_id": f"v{vid:03d}",
                    "block": 3,
                    "description": f"few_shot={n_shot}, T={temp}",
                    "temperature": temp,
                    "order_seed": 1,
                    "prompt_template": "default",
                    "system_prompt": "none",
                    "few_shot": n_shot,
                    "label_format": "letter_paren",
                }
            )
            vid += 1

    # Block 4: label format x temperature (seed=1, default prompt, no system)
    for label_name in LABEL_FORMATS:
        for temp in TEMPERATURES:
            variants.append(
                {
                    "variant_id": f"v{vid:03d}",
                    "block": 4,
                    "description": f"label={label_name}, T={temp}",
                    "temperature": temp,
                    "order_seed": 1,
                    "prompt_template": "default",
                    "system_prompt": "none",
                    "few_shot": 0,
                    "label_format": label_name,
                }
            )
            vid += 1

    # Block 5: option-order x temperature (cosmetic baseline for comparison)
    for seed in ORDER_SEEDS:
        for temp in [0.3, 0.7]:
            variants.append(
                {
                    "variant_id": f"v{vid:03d}",
                    "block": 5,
                    "description": f"order={seed}, T={temp}",
                    "temperature": temp,
                    "order_seed": seed,
                    "prompt_template": "default",
                    "system_prompt": "none",
                    "few_shot": 0,
                    "label_format": "letter_paren",
                }
            )
            vid += 1

    return variants


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("task", help="Inspect task name")
    parser.add_argument("output", help="Output manifest path")
    args = parser.parse_args()

    variants = generate_variants()

    block_counts: dict[int, int] = {}
    for v in variants:
        b = v["block"]
        block_counts[b] = block_counts.get(b, 0) + 1

    manifest = {
        "task": args.task,
        "model": "Qwen/Qwen2.5-7B-Instruct",
        "total_variants": len(variants),
        "design": {
            "name": "destructive perturbation workup",
            "block_0": f"deterministic baseline ({block_counts.get(0, 0)})",
            "block_1": f"prompt template x temperature ({block_counts.get(1, 0)})",
            "block_2": f"system prompt x prompt template ({block_counts.get(2, 0)})",
            "block_3": f"few-shot level x temperature ({block_counts.get(3, 0)})",
            "block_4": f"label format x temperature ({block_counts.get(4, 0)})",
            "block_5": f"option-order x temperature cosmetic control ({block_counts.get(5, 0)})",
        },
        "prompt_templates": {k: v for k, v in PROMPT_TEMPLATES.items()},
        "system_prompts": SYSTEM_PROMPTS,
        "few_shot_levels": FEW_SHOT_LEVELS,
        "label_formats": LABEL_FORMATS,
        "runs": variants,
    }

    output = Path(args.output)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(manifest, indent=2) + "\n")

    print(f"Generated {len(variants)} variants for {args.task} -> {output}")
    for b, c in sorted(block_counts.items()):
        print(f"  Block {b}: {c} variants")


if __name__ == "__main__":
    main()
