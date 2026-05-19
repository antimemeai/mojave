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


@pytest.fixture()
def factor_response_file(tmp_path: Path) -> tuple[Path, list[list[float]]]:
    """Generate ordinal response CSV from a known 3-factor structure.

    Returns (path_to_csv, true_loadings as nested list (12, 3)).
    12 items, 4 per factor. 3 ordinal categories (0, 1, 2).
    """
    import numpy as np

    rng = np.random.default_rng(42)

    true_loadings: list[list[float]] = [
        [0.8, 0.0, 0.0],
        [0.7, 0.0, 0.0],
        [0.9, 0.0, 0.0],
        [0.6, 0.0, 0.0],
        [0.0, 0.8, 0.0],
        [0.0, 0.7, 0.0],
        [0.0, 0.9, 0.0],
        [0.0, 0.6, 0.0],
        [0.0, 0.0, 0.8],
        [0.0, 0.0, 0.7],
        [0.0, 0.0, 0.9],
        [0.0, 0.0, 0.6],
    ]
    loading_arr = np.array(true_loadings)
    n_subjects = 500
    n_items = 12

    factors = rng.standard_normal((n_subjects, 3))
    latent = factors @ loading_arr.T + rng.standard_normal((n_subjects, n_items)) * 0.3

    data = np.zeros((n_subjects, n_items), dtype=int)
    data[latent > 0.3] = 1
    data[latent > 1.0] = 2

    columns = [f"item_{i:02d}" for i in range(n_items)]
    path = tmp_path / "responses.csv"
    header = ",".join(columns)
    rows = [",".join(str(v) for v in row) for row in data]
    path.write_text(header + "\n" + "\n".join(rows))

    return path, true_loadings


@pytest.fixture()
def cfa_data_file(tmp_path: Path) -> tuple[Path, str]:
    """Generate continuous data from a known 3-factor model.

    Returns (path_to_csv, lavaan_model_string).
    9 items (3 per factor), 500 subjects.
    """
    import numpy as np
    import pandas as pd

    rng = np.random.default_rng(42)
    n_subjects = 500

    loadings = np.array(
        [
            [0.8, 0.0, 0.0],
            [0.7, 0.0, 0.0],
            [0.6, 0.0, 0.0],
            [0.0, 0.8, 0.0],
            [0.0, 0.7, 0.0],
            [0.0, 0.6, 0.0],
            [0.0, 0.0, 0.8],
            [0.0, 0.0, 0.7],
            [0.0, 0.0, 0.6],
        ]
    )
    factors = rng.standard_normal((n_subjects, 3))
    noise = rng.standard_normal((n_subjects, 9)) * 0.4
    observed = factors @ loadings.T + noise

    columns = [f"x{i + 1}" for i in range(9)]
    df = pd.DataFrame(observed, columns=columns)
    path = tmp_path / "cfa_data.csv"
    df.to_csv(path, index=False)

    model_spec = "f1 =~ x1 + x2 + x3\nf2 =~ x4 + x5 + x6\nf3 =~ x7 + x8 + x9"
    return path, model_spec
