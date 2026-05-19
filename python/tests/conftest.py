from __future__ import annotations

import json
import math
import random
from typing import TYPE_CHECKING

import pytest

if TYPE_CHECKING:
    from pathlib import Path


@pytest.fixture()
def irt_response_file(tmp_path: Path) -> tuple[Path, dict[str, dict[str, float]]]:
    """Generate a synthetic JSONL response file from known 2PL parameters.

    Returns (path_to_jsonl, {item_id: {"a": disc, "b": diff}}).
    """
    true_params: dict[str, dict[str, float]] = {
        "item_00": {"a": 1.0, "b": -2.0},
        "item_01": {"a": 0.8, "b": -1.5},
        "item_02": {"a": 1.2, "b": -1.0},
        "item_03": {"a": 0.6, "b": -0.5},
        "item_04": {"a": 1.5, "b": 0.0},
        "item_05": {"a": 0.9, "b": 0.5},
        "item_06": {"a": 1.1, "b": 1.0},
        "item_07": {"a": 0.7, "b": 1.5},
        "item_08": {"a": 1.3, "b": 2.0},
        "item_09": {"a": 1.0, "b": 0.0},
    }
    n_subjects = 200
    rng = random.Random(42)

    lines: list[str] = []
    for i in range(n_subjects):
        theta = -3.0 + 6.0 * i / (n_subjects - 1)
        responses: dict[str, int] = {}
        for item_id, params in true_params.items():
            p = 1.0 / (1.0 + math.exp(-params["a"] * (theta - params["b"])))
            responses[item_id] = 1 if rng.random() < p else 0
        lines.append(json.dumps({"subject_id": f"s_{i:03d}", "responses": responses}))

    path = tmp_path / "responses.jsonl"
    path.write_text("\n".join(lines))
    return path, true_params
