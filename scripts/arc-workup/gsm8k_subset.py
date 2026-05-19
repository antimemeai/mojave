"""GSM8K 500-item pseudo-random subset for mojave perturbation analysis.

Wraps the standard inspect_evals/gsm8k task but restricts the dataset to
500 items selected pseudo-randomly with a fixed seed. The selection is
reproducible and documented in data/gsm8k/item-selection.json.

Usage:
    inspect eval scripts/arc-workup/gsm8k_subset.py \
        --model openai/Qwen/Qwen2.5-7B-Instruct \
        --log-dir data/gsm8k/logs/v000
"""

from __future__ import annotations

import hashlib
import json
import random
from pathlib import Path

from inspect_ai import Task, task
from inspect_ai.scorer import match
from inspect_ai.solver import generate, prompt_template, system_message
from inspect_evals.constants import DEFAULT_FEWSHOT_SEED
from inspect_evals.gsm8k.gsm8k import (
    DATASET_PATH,
    EVAL_VERSION,
    GSM8K_DATASET_REVISION,
    MATH_PROMPT_TEMPLATE,
    record_to_sample,
    sample_to_fewshot,
)
from inspect_evals.utils.huggingface import hf_dataset

SUBSET_SIZE = 500
SUBSET_SEED = 20260519


def _select_indices(n_total: int, n_select: int, seed: int) -> list[int]:
    rng = random.Random(seed)
    indices = list(range(n_total))
    rng.shuffle(indices)
    return sorted(indices[:n_select])


def _write_selection_doc(indices: list[int], n_total: int) -> None:
    doc_path = Path("data/gsm8k/item-selection.json")
    doc_path.parent.mkdir(parents=True, exist_ok=True)
    idx_hash = hashlib.sha256(json.dumps(indices).encode()).hexdigest()[:16]
    doc = {
        "method": "pseudo_random_subset",
        "description": (
            f"500 items selected pseudo-randomly from {n_total} GSM8K test items. "
            f"Selection uses Python random.Random(seed={SUBSET_SEED}).shuffle() "
            f"over range({n_total}), then takes the first {SUBSET_SIZE} indices "
            f"(sorted). All variants see the same 500 items."
        ),
        "seed": SUBSET_SEED,
        "n_total": n_total,
        "n_selected": len(indices),
        "indices_sha256_prefix": idx_hash,
        "selected_indices": indices,
        "dataset": DATASET_PATH,
        "dataset_revision": GSM8K_DATASET_REVISION,
        "split": "test",
        "reproducibility": (
            "Deterministic given the same seed and dataset revision. "
            "The dataset order is fixed by HuggingFace; indices reference "
            "positions in that canonical order."
        ),
    }
    doc_path.write_text(json.dumps(doc, indent=2) + "\n")


@task
def gsm8k_subset(
    fewshot: int = 10,
    fewshot_seed: int = DEFAULT_FEWSHOT_SEED,
    shuffle_fewshot: bool = True,
    shuffle_seed: int = 0,
) -> Task:
    full_dataset = hf_dataset(
        path=DATASET_PATH,
        data_dir="main",
        split="test",
        sample_fields=record_to_sample,
        revision=GSM8K_DATASET_REVISION,
    )

    n_total = len(full_dataset)
    indices = _select_indices(n_total, SUBSET_SIZE, SUBSET_SEED)
    subset = [full_dataset[i] for i in indices]

    _write_selection_doc(indices, n_total)

    solver = [prompt_template(MATH_PROMPT_TEMPLATE), generate()]
    if fewshot:
        fewshots = hf_dataset(
            path=DATASET_PATH,
            data_dir="main",
            split="train",
            sample_fields=record_to_sample,
            shuffle=shuffle_fewshot,
            seed=fewshot_seed,
            limit=fewshot,
            revision=GSM8K_DATASET_REVISION,
        )
        solver.insert(
            0,
            system_message("\n\n".join([sample_to_fewshot(sample) for sample in fewshots])),
        )

    return Task(
        dataset=subset,
        solver=solver,
        scorer=match(numeric=True),
        version=EVAL_VERSION.comparability_version,
        metadata=EVAL_VERSION.to_metadata(),
    )
