"""N-shot exemplar pool management for v2 perturbation matrix."""

from __future__ import annotations

import hashlib
import random
from typing import TYPE_CHECKING, Any

if TYPE_CHECKING:
    from inspect_ai.dataset import Sample

LETTERS = "ABCDEFGHIJKLMNOP"
N_SHOT_FRACS = [0.0, 0.01, 0.025, 0.05]
EXEMPLAR_SEED = 42


def draw_exemplar_pool(
    samples: list[Sample],
    max_frac: float = 0.05,
    seed: int = EXEMPLAR_SEED,
) -> list[Sample]:
    """Draw a seed-pinned random exemplar pool from the dataset."""
    n = max(1, int(max_frac * len(samples)))
    rng = random.Random(seed)
    return rng.sample(list(samples), k=n)


def compute_n_for_frac(frac: float, dataset_size: int) -> int:
    """Compute the number of exemplars for a given fraction."""
    if frac <= 0:
        return 0
    return max(1, int(frac * dataset_size))


def format_exemplar(sample: Sample) -> str:
    """Format a single sample as a completed Q&A exemplar."""
    question = sample.input if isinstance(sample.input, str) else str(sample.input)
    lines: list[str] = [question, ""]
    choices = sample.choices or []
    for i, choice in enumerate(choices):
        lines.append(f"{LETTERS[i]}) {choice}")
    lines.append(f"ANSWER: {sample.target}")
    return "\n".join(lines)


def build_nshot_prefix(pool: list[Sample], n: int) -> str:
    """Build the n-shot prefix string from the first n items in the pool."""
    if n <= 0:
        return ""
    exemplars = pool[:n]
    parts = [format_exemplar(ex) for ex in exemplars]
    return "\n\n".join(parts) + "\n\n"


def serialize_pool(pool: list[Sample], dataset_size: int) -> dict[str, Any]:
    """Serialize the exemplar pool for pre-registration and audit sealing."""
    levels: dict[str, dict[str, Any]] = {}
    for frac in N_SHOT_FRACS:
        count = compute_n_for_frac(frac, dataset_size)
        levels[str(frac)] = {
            "n_exemplars": count,
            "item_ids": [s.id for s in pool[:count]] if count > 0 else [],
        }

    pool_text = "\n".join(f"{s.id}:{s.target}" for s in pool)
    pool_hash = hashlib.sha256(pool_text.encode()).hexdigest()

    return {
        "seed": EXEMPLAR_SEED,
        "max_pool_size": len(pool),
        "dataset_size": dataset_size,
        "pool_item_ids": [s.id for s in pool],
        "pool_sha256": pool_hash,
        "levels": levels,
    }
