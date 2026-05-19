# mojave-calibrate Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build an offline Python calibration pipeline that wraps py-irt, deepirtools, and semopy behind a click CLI, emitting mojave-compatible JSON (item pool and factor structure files).

**Architecture:** `python/` directory at repo root. `src/mojave_calibrate/` package with a `Calibrator` protocol, three implementations (IRT, factors, CFA), a reeval stub, schema validation, and a click CLI entry point. Subprocess + JSON boundary — no PyO3.

**Tech Stack:** Python 3.10+, uv, click, py-irt, deepirtools, semopy, PyTorch, Pyro, numpy, pandas, pytest

---

## File structure

```
python/
  pyproject.toml
  src/
    mojave_calibrate/
      __init__.py
      protocol.py
      schema.py
      irt.py
      factors.py
      cfa.py
      reeval_stub.py
      cli.py
  tests/
    conftest.py
    test_schema.py
    test_irt.py
    test_factors.py
    test_cfa.py
    test_cli.py
```

**Responsibilities:**
- `protocol.py` — `CalibrationResult` frozen dataclass + `Calibrator` typing.Protocol
- `schema.py` — validate + serialize item pool JSON and factor structure JSON
- `irt.py` — `IrtCalibrator` wrapping py-irt `IrtModelTrainer`
- `factors.py` — `FactorCalibrator` wrapping deepirtools `IWAVE`
- `cfa.py` — `CfaCalibrator` wrapping semopy `Model`
- `reeval_stub.py` — `ReEvalCalibrator` that raises `NotImplementedError`
- `cli.py` — click group with `irt`, `factors`, `cfa` subcommands
- `conftest.py` — synthetic data fixtures shared across test files

---

### Task 1: Project scaffold

**Files:**
- Create: `python/pyproject.toml`
- Create: `python/src/mojave_calibrate/__init__.py`

- [ ] **Step 1: Create directory structure**

```bash
mkdir -p python/src/mojave_calibrate python/tests
```

- [ ] **Step 2: Write pyproject.toml**

Create `python/pyproject.toml`:

```toml
[project]
name = "mojave-calibrate"
version = "0.1.0"
description = "Offline calibration pipeline for mojave measurement engine"
requires-python = ">=3.10"
dependencies = [
    "click>=8.0",
    "py-irt>=0.7",
    "deepirtools>=0.3",
    "semopy>=2.3",
    "torch>=2.0",
    "pyro-ppl>=1.8",
    "numpy>=1.24",
    "pandas>=2.0",
]

[project.scripts]
mojave-calibrate = "mojave_calibrate.cli:main"

[build-system]
requires = ["hatchling"]
build-backend = "hatchling.build"

[tool.hatch.build.targets.wheel]
packages = ["src/mojave_calibrate"]

[tool.pytest.ini_options]
testpaths = ["tests"]

[dependency-groups]
dev = [
    "pytest>=8.0",
    "pytest-timeout>=2.0",
    "scipy>=1.10",
]
```

- [ ] **Step 3: Write `__init__.py`**

Create `python/src/mojave_calibrate/__init__.py`:

```python
"""mojave-calibrate: offline calibration pipeline for the mojave measurement engine."""

__version__ = "0.1.0"
```

- [ ] **Step 4: Initialize uv and sync**

```bash
cd python
uv sync
```

Expected: creates `uv.lock`, installs all dependencies into `.venv`.
This will take a while — PyTorch and Pyro are large.

- [ ] **Step 5: Verify pytest runs (no tests yet)**

```bash
cd python
uv run pytest --co -q
```

Expected: `no tests ran` (or `0 items collected`). Confirms the toolchain works.

- [ ] **Step 6: Commit**

```bash
git add python/pyproject.toml python/uv.lock python/src/mojave_calibrate/__init__.py
git commit -m "chore: scaffold mojave-calibrate Python package with uv"
```

Note: do NOT commit `.venv/`. Add `python/.venv/` to `.gitignore` if not
already covered by a global pattern.

---

### Task 2: Protocol and CalibrationResult

**Files:**
- Create: `python/src/mojave_calibrate/protocol.py`
- Create: `python/tests/test_schema.py` (partial — protocol-level tests)

- [ ] **Step 1: Write test for CalibrationResult**

Create `python/tests/test_schema.py`:

```python
from mojave_calibrate.protocol import CalibrationResult


def test_calibration_result_is_frozen():
    result = CalibrationResult(
        items=[{"id": "i1", "difficulty": 0.5, "discrimination": 1.0}],
        factors=None,
        metadata={"model": "2pl"},
    )
    assert result.items is not None
    assert result.factors is None
    assert result.metadata["model"] == "2pl"

    # frozen: assignment should raise
    try:
        result.items = []
        assert False, "should have raised FrozenInstanceError"
    except AttributeError:
        pass


def test_calibration_result_factors_only():
    result = CalibrationResult(
        items=None,
        factors={"latent_factors": ["f0"], "loadings": [[0.8]]},
        metadata={"model_type": "grm"},
    )
    assert result.items is None
    assert result.factors["latent_factors"] == ["f0"]
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cd python
uv run pytest tests/test_schema.py -v
```

Expected: FAIL with `ModuleNotFoundError: No module named 'mojave_calibrate.protocol'`

- [ ] **Step 3: Write protocol.py**

Create `python/src/mojave_calibrate/protocol.py`:

```python
from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path
from typing import Any, Protocol


@dataclass(frozen=True)
class CalibrationResult:
    items: list[dict[str, Any]] | None
    factors: dict[str, Any] | None
    metadata: dict[str, Any]


class Calibrator(Protocol):
    def fit(self, data: Path, **kwargs: Any) -> CalibrationResult: ...

    def name(self) -> str: ...
```

- [ ] **Step 4: Run tests to verify they pass**

```bash
cd python
uv run pytest tests/test_schema.py -v
```

Expected: 2 passed.

- [ ] **Step 5: Commit**

```bash
git add python/src/mojave_calibrate/protocol.py python/tests/test_schema.py
git commit -m "feat(calibrate): add CalibrationResult dataclass and Calibrator protocol"
```

---

### Task 3: Schema validation

**Files:**
- Create: `python/src/mojave_calibrate/schema.py`
- Modify: `python/tests/test_schema.py` (add schema validation tests)

- [ ] **Step 1: Write schema validation tests**

Append to `python/tests/test_schema.py`:

```python
import json
import math

import pytest

from mojave_calibrate.schema import (
    validate_item_pool,
    validate_factor_structure,
    write_item_pool,
    write_factor_structure,
    SchemaError,
)


class TestValidateItemPool:
    def test_valid_item_pool(self):
        items = [
            {
                "id": "task_001",
                "difficulty": 0.5,
                "discrimination": 1.2,
                "content_domain": "math",
                "exposure_count": 0,
            },
            {
                "id": "task_002",
                "difficulty": -1.0,
                "discrimination": 0.8,
                "content_domain": "math",
                "exposure_count": 0,
            },
        ]
        metadata = {"model": "2pl", "package": "py-irt"}
        validate_item_pool(items, metadata)

    def test_rejects_zero_discrimination(self):
        items = [
            {
                "id": "bad",
                "difficulty": 0.0,
                "discrimination": 0.0,
                "content_domain": "x",
                "exposure_count": 0,
            }
        ]
        with pytest.raises(SchemaError, match="discrimination"):
            validate_item_pool(items, {})

    def test_rejects_negative_discrimination(self):
        items = [
            {
                "id": "bad",
                "difficulty": 0.0,
                "discrimination": -0.5,
                "content_domain": "x",
                "exposure_count": 0,
            }
        ]
        with pytest.raises(SchemaError, match="discrimination"):
            validate_item_pool(items, {})

    def test_rejects_nan_difficulty(self):
        items = [
            {
                "id": "bad",
                "difficulty": float("nan"),
                "discrimination": 1.0,
                "content_domain": "x",
                "exposure_count": 0,
            }
        ]
        with pytest.raises(SchemaError, match="finite"):
            validate_item_pool(items, {})

    def test_rejects_nan_discrimination(self):
        items = [
            {
                "id": "bad",
                "difficulty": 0.0,
                "discrimination": float("nan"),
                "content_domain": "x",
                "exposure_count": 0,
            }
        ]
        with pytest.raises(SchemaError, match="finite"):
            validate_item_pool(items, {})

    def test_rejects_empty_id(self):
        items = [
            {
                "id": "",
                "difficulty": 0.0,
                "discrimination": 1.0,
                "content_domain": "x",
                "exposure_count": 0,
            }
        ]
        with pytest.raises(SchemaError, match="id"):
            validate_item_pool(items, {})

    def test_rejects_duplicate_ids(self):
        items = [
            {
                "id": "dup",
                "difficulty": 0.0,
                "discrimination": 1.0,
                "content_domain": "x",
                "exposure_count": 0,
            },
            {
                "id": "dup",
                "difficulty": 0.5,
                "discrimination": 1.2,
                "content_domain": "x",
                "exposure_count": 0,
            },
        ]
        with pytest.raises(SchemaError, match="duplicate"):
            validate_item_pool(items, {})

    def test_rejects_empty_items(self):
        with pytest.raises(SchemaError, match="empty"):
            validate_item_pool([], {})


class TestValidateFactorStructure:
    def test_valid_factor_structure(self):
        factors = {
            "latent_factors": ["f0", "f1"],
            "loadings": [[0.8, 0.1], [0.1, 0.9]],
            "intercepts": [1.0, 0.5],
            "covariance": [[1.0, 0.3], [0.3, 1.0]],
            "fit_indices": {"log_likelihood": -100.0},
        }
        validate_factor_structure(factors, {})

    def test_rejects_mismatched_loadings_rows(self):
        factors = {
            "latent_factors": ["f0", "f1"],
            "loadings": [[0.8, 0.1]],
            "intercepts": [1.0, 0.5],
            "covariance": [[1.0, 0.3], [0.3, 1.0]],
            "fit_indices": {},
        }
        with pytest.raises(SchemaError, match="intercept"):
            validate_factor_structure(factors, {})

    def test_rejects_mismatched_loadings_cols(self):
        factors = {
            "latent_factors": ["f0", "f1"],
            "loadings": [[0.8, 0.1, 0.0], [0.1, 0.9, 0.0]],
            "intercepts": [1.0, 0.5],
            "covariance": [[1.0, 0.3], [0.3, 1.0]],
            "fit_indices": {},
        }
        with pytest.raises(SchemaError, match="latent_factors"):
            validate_factor_structure(factors, {})


class TestWriteItemPool:
    def test_roundtrip(self, tmp_path):
        items = [
            {
                "id": "t1",
                "difficulty": 0.5,
                "discrimination": 1.0,
                "content_domain": "math",
                "exposure_count": 0,
            }
        ]
        metadata = {"model": "2pl"}
        out = tmp_path / "pool.json"
        write_item_pool(items, metadata, out)

        with open(out) as f:
            data = json.load(f)

        assert len(data["items"]) == 1
        assert data["items"][0]["id"] == "t1"
        assert data["calibration_metadata"]["model"] == "2pl"


class TestWriteFactorStructure:
    def test_roundtrip(self, tmp_path):
        factors = {
            "latent_factors": ["f0"],
            "loadings": [[0.8]],
            "intercepts": [1.0],
            "covariance": [[1.0]],
            "fit_indices": {"CFI": 0.95},
        }
        metadata = {"model_type": "grm"}
        out = tmp_path / "factors.json"
        write_factor_structure(factors, metadata, out)

        with open(out) as f:
            data = json.load(f)

        assert data["latent_factors"] == ["f0"]
        assert data["calibration_metadata"]["model_type"] == "grm"
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cd python
uv run pytest tests/test_schema.py -v -k "TestValidate or TestWrite"
```

Expected: FAIL with `ImportError: cannot import name 'validate_item_pool' from 'mojave_calibrate.schema'`

- [ ] **Step 3: Write schema.py**

Create `python/src/mojave_calibrate/schema.py`:

```python
from __future__ import annotations

import json
import math
from pathlib import Path
from typing import Any


class SchemaError(Exception):
    pass


def validate_item_pool(
    items: list[dict[str, Any]], metadata: dict[str, Any]
) -> None:
    if not items:
        raise SchemaError("items list must not be empty")

    seen_ids: set[str] = set()
    for i, item in enumerate(items):
        item_id = item.get("id", "")
        if not item_id:
            raise SchemaError(f"item {i}: id must be non-empty")
        if item_id in seen_ids:
            raise SchemaError(f"item {i}: duplicate id '{item_id}'")
        seen_ids.add(item_id)

        diff = item.get("difficulty", 0.0)
        disc = item.get("discrimination", 0.0)

        if not math.isfinite(diff):
            raise SchemaError(f"item '{item_id}': difficulty must be finite, got {diff}")
        if not math.isfinite(disc):
            raise SchemaError(
                f"item '{item_id}': discrimination must be finite, got {disc}"
            )
        if disc <= 0:
            raise SchemaError(
                f"item '{item_id}': discrimination must be > 0, got {disc}"
            )


def validate_factor_structure(
    factors: dict[str, Any], metadata: dict[str, Any]
) -> None:
    latent = factors.get("latent_factors", [])
    loadings = factors.get("loadings", [])
    intercepts = factors.get("intercepts", [])
    covariance = factors.get("covariance", [])

    n_factors = len(latent)
    n_items = len(intercepts)

    if loadings and n_items != len(loadings):
        raise SchemaError(
            f"loadings has {len(loadings)} rows but intercepts has {n_items} "
            f"entries — row count must match"
        )

    if loadings and loadings[0]:
        cols = len(loadings[0])
        if cols != n_factors:
            raise SchemaError(
                f"loadings has {cols} columns but latent_factors has "
                f"{n_factors} entries — column count must match"
            )

    if covariance and len(covariance) != n_factors:
        raise SchemaError(
            f"covariance is {len(covariance)}x{len(covariance)} but expected "
            f"{n_factors}x{n_factors}"
        )


def write_item_pool(
    items: list[dict[str, Any]],
    metadata: dict[str, Any],
    output: Path,
) -> None:
    validate_item_pool(items, metadata)
    doc = {"items": items, "calibration_metadata": metadata}
    output.write_text(json.dumps(doc, indent=2))


def write_factor_structure(
    factors: dict[str, Any],
    metadata: dict[str, Any],
    output: Path,
) -> None:
    validate_factor_structure(factors, metadata)
    doc = {**factors, "calibration_metadata": metadata}
    output.write_text(json.dumps(doc, indent=2))
```

- [ ] **Step 4: Run tests to verify they pass**

```bash
cd python
uv run pytest tests/test_schema.py -v
```

Expected: all tests pass.

- [ ] **Step 5: Commit**

```bash
git add python/src/mojave_calibrate/schema.py python/tests/test_schema.py
git commit -m "feat(calibrate): add schema validation for item pool and factor structure JSON"
```

---

### Task 4: IrtCalibrator

**Files:**
- Create: `python/src/mojave_calibrate/irt.py`
- Create: `python/tests/conftest.py`
- Create: `python/tests/test_irt.py`

**Reference docs:**
- py-irt API: `IrtConfig` from `py_irt.config`, `IrtModelTrainer` from `py_irt.training`
- py-irt output: `trainer.best_params` dict with keys `"diff"`, `"disc"`, `"item_ids"`, `"ability"`, `"subject_ids"`
- py-irt input: JSONL with `{"subject_id": str, "responses": {item_id: 0|1}}`
- Rust contract: `ItemMetadata` requires `id` (String), `difficulty` (f64), `discrimination` (f64 > 0), `content_domain` (String), `exposure_count` (u64)

- [ ] **Step 1: Write conftest.py with IRT fixture**

Create `python/tests/conftest.py`:

```python
from __future__ import annotations

import json
import math
import random
from pathlib import Path

import pytest


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
```

- [ ] **Step 2: Write IRT test**

Create `python/tests/test_irt.py`:

```python
from __future__ import annotations

import logging
from pathlib import Path

import pytest
from scipy.stats import spearmanr

from mojave_calibrate.irt import IrtCalibrator
from mojave_calibrate.schema import validate_item_pool


def test_irt_calibrator_fits_2pl(irt_response_file):
    path, true_params = irt_response_file

    calibrator = IrtCalibrator(
        model_type="2pl",
        epochs=500,
        device="cpu",
        content_domain="test",
    )
    assert calibrator.name() == "irt"

    result = calibrator.fit(path)

    assert result.items is not None
    assert result.factors is None
    assert len(result.items) > 0

    validate_item_pool(result.items, result.metadata)

    # Check recovered difficulty correlates with ground truth
    recovered = {item["id"]: item for item in result.items}
    true_diffs = []
    est_diffs = []
    for item_id, params in true_params.items():
        if item_id in recovered:
            true_diffs.append(params["b"])
            est_diffs.append(recovered[item_id]["difficulty"])

    if len(true_diffs) >= 5:
        corr, _ = spearmanr(true_diffs, est_diffs)
        assert corr > 0.7, f"difficulty rank correlation {corr:.3f} too low"


def test_irt_calibrator_metadata(irt_response_file):
    path, _ = irt_response_file

    calibrator = IrtCalibrator(
        model_type="2pl",
        epochs=100,
        device="cpu",
        content_domain="general",
    )
    result = calibrator.fit(path)

    assert result.metadata["model"] == "2pl"
    assert result.metadata["package"] == "py-irt"
    assert "n_items" in result.metadata
    assert "n_subjects" in result.metadata
    assert "timestamp" in result.metadata


def test_irt_calibrator_filters_bad_discrimination(irt_response_file, caplog):
    """If py-irt returns disc <= 0 for any item, it should be excluded."""
    path, _ = irt_response_file

    calibrator = IrtCalibrator(
        model_type="2pl",
        epochs=100,
        device="cpu",
        content_domain="test",
    )
    result = calibrator.fit(path)

    for item in result.items:
        assert item["discrimination"] > 0, (
            f"item {item['id']} has discrimination {item['discrimination']}"
        )
```

- [ ] **Step 3: Run tests to verify they fail**

```bash
cd python
uv run pytest tests/test_irt.py -v
```

Expected: FAIL with `ModuleNotFoundError: No module named 'mojave_calibrate.irt'`

- [ ] **Step 4: Write irt.py**

Create `python/src/mojave_calibrate/irt.py`:

```python
from __future__ import annotations

import logging
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

from py_irt.config import IrtConfig
from py_irt.training import IrtModelTrainer

from mojave_calibrate.protocol import CalibrationResult

logger = logging.getLogger(__name__)


class IrtCalibrator:
    def __init__(
        self,
        model_type: str = "2pl",
        epochs: int = 2000,
        lr: float = 0.1,
        lr_decay: float = 0.9999,
        priors: str = "vague",
        device: str = "cpu",
        seed: int | None = None,
        content_domain: str = "general",
    ) -> None:
        self._model_type = model_type
        self._epochs = epochs
        self._lr = lr
        self._lr_decay = lr_decay
        self._priors = priors
        self._device = device
        self._seed = seed
        self._content_domain = content_domain

    def name(self) -> str:
        return "irt"

    def fit(self, data: Path, **kwargs: Any) -> CalibrationResult:
        config = IrtConfig(
            model_type=self._model_type,
            epochs=self._epochs,
            lr=self._lr,
            lr_decay=self._lr_decay,
            priors=self._priors,
            seed=self._seed,
        )

        trainer = IrtModelTrainer(config=config, data_path=data)
        trainer.train(device=self._device)
        params = trainer.best_params

        item_ids_map: dict[str, str] = params.get("item_ids", {})
        diffs: list[float] = params.get("diff", [])
        discs: list[float] = params.get("disc", [])

        items: list[dict[str, Any]] = []
        n_skipped = 0
        for idx_str, item_id in sorted(item_ids_map.items(), key=lambda kv: int(kv[0])):
            idx = int(idx_str)
            if idx >= len(diffs):
                continue

            disc = discs[idx] if idx < len(discs) else 1.0
            if disc <= 0:
                logger.warning("item '%s': discrimination %.4f <= 0, skipping", item_id, disc)
                n_skipped += 1
                continue

            items.append(
                {
                    "id": item_id,
                    "difficulty": diffs[idx],
                    "discrimination": disc,
                    "content_domain": self._content_domain,
                    "exposure_count": 0,
                }
            )

        if n_skipped:
            logger.info("skipped %d items with non-positive discrimination", n_skipped)

        n_subjects = len(params.get("ability", []))

        metadata: dict[str, Any] = {
            "model": self._model_type,
            "n_items": len(items),
            "n_subjects": n_subjects,
            "timestamp": datetime.now(timezone.utc).isoformat(),
            "package": "py-irt",
            "package_version": _pyirt_version(),
        }

        return CalibrationResult(items=items, factors=None, metadata=metadata)


def _pyirt_version() -> str:
    try:
        from importlib.metadata import version
        return version("py-irt")
    except Exception:
        return "unknown"
```

- [ ] **Step 5: Run tests to verify they pass**

```bash
cd python
uv run pytest tests/test_irt.py -v --timeout=120
```

Expected: 3 passed. Note: py-irt training on CPU is slow — the 500-epoch test
may take 30-60 seconds.

- [ ] **Step 6: Commit**

```bash
git add python/src/mojave_calibrate/irt.py python/tests/conftest.py python/tests/test_irt.py
git commit -m "feat(calibrate): add IrtCalibrator wrapping py-irt with 2PL support"
```

---

### Task 5: FactorCalibrator

**Files:**
- Create: `python/src/mojave_calibrate/factors.py`
- Create: `python/tests/test_factors.py`
- Modify: `python/tests/conftest.py` (add factor data fixture)

**Reference docs:**
- deepirtools API: `IWAVE` class, `model.fit(data, iw_samples=N)`,
  `model.loadings`, `model.intercepts`, `model.cov`, `model.log_likelihood(data)`
- Input: `torch.Tensor` of shape `(n_subjects, n_items)`, integer-coded
- Q-matrix: `torch.Tensor` of shape `(n_items, latent_size)`, binary constraint

- [ ] **Step 1: Add factor data fixture to conftest.py**

Append to `python/tests/conftest.py`:

```python
import numpy as np


@pytest.fixture()
def factor_response_file(tmp_path: Path) -> tuple[Path, np.ndarray]:
    """Generate ordinal response CSV from a known 3-factor structure.

    Returns (path_to_csv, true_loadings array of shape (12, 3)).
    12 items, 4 per factor. 3 ordinal categories (0, 1, 2).
    """
    rng = np.random.default_rng(42)

    true_loadings = np.array(
        [
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
    )
    n_subjects = 500
    n_items = 12

    factors = rng.standard_normal((n_subjects, 3))
    latent = factors @ true_loadings.T + rng.standard_normal((n_subjects, n_items)) * 0.3

    data = np.zeros((n_subjects, n_items), dtype=int)
    data[latent > 0.3] = 1
    data[latent > 1.0] = 2

    columns = [f"item_{i:02d}" for i in range(n_items)]
    path = tmp_path / "responses.csv"
    header = ",".join(columns)
    rows = [",".join(str(v) for v in row) for row in data]
    path.write_text(header + "\n" + "\n".join(rows))

    return path, true_loadings
```

- [ ] **Step 2: Write factor test**

Create `python/tests/test_factors.py`:

```python
from __future__ import annotations

import numpy as np
import pytest

from mojave_calibrate.factors import FactorCalibrator
from mojave_calibrate.schema import validate_factor_structure


def test_factor_calibrator_fits_grm(factor_response_file):
    path, true_loadings = factor_response_file

    calibrator = FactorCalibrator(
        latent_size=3,
        model_type="grm",
        n_cats=3,
        device="cpu",
        max_epochs=200,
        iw_samples=5,
    )
    assert calibrator.name() == "factors"

    result = calibrator.fit(path)

    assert result.items is None
    assert result.factors is not None

    factors = result.factors
    validate_factor_structure(factors, result.metadata)

    assert len(factors["latent_factors"]) == 3
    assert len(factors["loadings"]) == 12
    assert len(factors["loadings"][0]) == 3
    assert len(factors["covariance"]) == 3


def test_factor_calibrator_custom_names(factor_response_file):
    path, _ = factor_response_file

    calibrator = FactorCalibrator(
        latent_size=3,
        model_type="grm",
        n_cats=3,
        device="cpu",
        max_epochs=50,
        iw_samples=5,
        factor_names=["reasoning", "code", "retrieval"],
    )
    result = calibrator.fit(path)
    assert result.factors["latent_factors"] == ["reasoning", "code", "retrieval"]


def test_factor_calibrator_metadata(factor_response_file):
    path, _ = factor_response_file

    calibrator = FactorCalibrator(
        latent_size=3,
        model_type="grm",
        n_cats=3,
        device="cpu",
        max_epochs=50,
        iw_samples=5,
    )
    result = calibrator.fit(path)

    assert result.metadata["model_type"] == "grm"
    assert result.metadata["latent_size"] == 3
    assert result.metadata["package"] == "deepirtools"
    assert "n_subjects" in result.metadata
    assert "n_items" in result.metadata
    assert "timestamp" in result.metadata
```

- [ ] **Step 3: Run tests to verify they fail**

```bash
cd python
uv run pytest tests/test_factors.py -v
```

Expected: FAIL with `ModuleNotFoundError: No module named 'mojave_calibrate.factors'`

- [ ] **Step 4: Write factors.py**

Create `python/src/mojave_calibrate/factors.py`:

```python
from __future__ import annotations

import csv
import logging
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

import numpy as np
import torch
from deepirtools import IWAVE

from mojave_calibrate.protocol import CalibrationResult

logger = logging.getLogger(__name__)


class FactorCalibrator:
    def __init__(
        self,
        latent_size: int,
        model_type: str = "grm",
        n_cats: int = 3,
        q_matrix_path: Path | None = None,
        correlated_factors: list[int] | None = None,
        device: str = "cpu",
        max_epochs: int = 100_000,
        iw_samples: int = 5000,
        factor_names: list[str] | None = None,
    ) -> None:
        self._latent_size = latent_size
        self._model_type = model_type
        self._n_cats = n_cats
        self._q_matrix_path = q_matrix_path
        self._correlated_factors = correlated_factors
        self._device = device
        self._max_epochs = max_epochs
        self._iw_samples = iw_samples
        self._factor_names = factor_names

    def name(self) -> str:
        return "factors"

    def fit(self, data: Path, **kwargs: Any) -> CalibrationResult:
        tensor, n_items = _load_csv_as_tensor(data)
        n_subjects = tensor.shape[0]

        q_matrix = None
        if self._q_matrix_path is not None:
            q_matrix = _load_q_matrix(self._q_matrix_path)

        correlated = self._correlated_factors
        if correlated is None:
            correlated = list(range(self._latent_size))

        model = IWAVE(
            model_type=self._model_type,
            latent_size=self._latent_size,
            n_cats=[self._n_cats] * n_items,
            Q=q_matrix,
            correlated_factors=correlated,
            device=self._device,
        )

        model.fit(tensor, max_epochs=self._max_epochs, iw_samples=self._iw_samples)

        loadings = model.loadings.detach().cpu().tolist()
        intercepts = model.intercepts.detach().cpu().tolist()
        cov = model.cov.detach().cpu().tolist()
        log_lik = model.log_likelihood(tensor)

        factor_names = self._factor_names
        if factor_names is None:
            factor_names = [f"factor_{i}" for i in range(self._latent_size)]

        factors: dict[str, Any] = {
            "latent_factors": factor_names,
            "loadings": loadings,
            "intercepts": intercepts,
            "covariance": cov,
            "fit_indices": {"log_likelihood": log_lik},
        }

        metadata: dict[str, Any] = {
            "model_type": self._model_type,
            "latent_size": self._latent_size,
            "n_subjects": n_subjects,
            "n_items": n_items,
            "timestamp": datetime.now(timezone.utc).isoformat(),
            "package": "deepirtools",
            "package_version": _deepirtools_version(),
        }

        return CalibrationResult(items=None, factors=factors, metadata=metadata)


def _load_csv_as_tensor(path: Path) -> tuple[torch.Tensor, int]:
    with open(path) as f:
        reader = csv.reader(f)
        header = next(reader)
        n_items = len(header)
        rows = [[int(v) for v in row] for row in reader]
    return torch.tensor(rows, dtype=torch.long), n_items


def _load_q_matrix(path: Path) -> torch.Tensor:
    with open(path) as f:
        reader = csv.reader(f)
        rows = [[float(v) for v in row] for row in reader]
    return torch.tensor(rows)


def _deepirtools_version() -> str:
    try:
        from importlib.metadata import version
        return version("deepirtools")
    except Exception:
        return "unknown"
```

- [ ] **Step 5: Run tests to verify they pass**

```bash
cd python
uv run pytest tests/test_factors.py -v --timeout=180
```

Expected: 3 passed. deepirtools on CPU with low `max_epochs` and `iw_samples`
should complete in under a minute.

- [ ] **Step 6: Commit**

```bash
git add python/src/mojave_calibrate/factors.py python/tests/conftest.py python/tests/test_factors.py
git commit -m "feat(calibrate): add FactorCalibrator wrapping deepirtools IWAVE"
```

---

### Task 6: CfaCalibrator

**Files:**
- Create: `python/src/mojave_calibrate/cfa.py`
- Create: `python/tests/test_cfa.py`
- Modify: `python/tests/conftest.py` (add CFA data fixture)

**Reference docs:**
- semopy API: `Model(desc_string).fit(dataframe, obj="MLW")`,
  `model.inspect()` returns DataFrame (lval, op, rval, Estimate, Std. Err, z-value, p-value),
  `semopy.calc_stats(model)` returns DataFrame with CFI, RMSEA, chi2, etc.
- Input: `pd.DataFrame` with columns matching variable names in the model spec

- [ ] **Step 1: Add CFA data fixture to conftest.py**

Append to `python/tests/conftest.py`:

```python
import pandas as pd


@pytest.fixture()
def cfa_data_file(tmp_path: Path) -> tuple[Path, str]:
    """Generate continuous data from a known 3-factor model.

    Returns (path_to_csv, lavaan_model_string).
    9 items (3 per factor), 500 subjects.
    """
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
```

- [ ] **Step 2: Write CFA test**

Create `python/tests/test_cfa.py`:

```python
from __future__ import annotations

import pytest

from mojave_calibrate.cfa import CfaCalibrator
from mojave_calibrate.schema import validate_factor_structure


def test_cfa_calibrator_fits_model(cfa_data_file):
    path, model_spec = cfa_data_file

    calibrator = CfaCalibrator(model=model_spec, objective="MLW")
    assert calibrator.name() == "cfa"

    result = calibrator.fit(path)

    assert result.items is None
    assert result.factors is not None

    factors = result.factors
    validate_factor_structure(factors, result.metadata)

    assert factors["latent_factors"] == ["f1", "f2", "f3"]
    assert len(factors["loadings"]) == 9
    assert len(factors["loadings"][0]) == 3

    fi = factors["fit_indices"]
    assert "CFI" in fi
    assert "RMSEA" in fi


def test_cfa_calibrator_model_from_file(cfa_data_file, tmp_path):
    path, model_spec = cfa_data_file

    model_file = tmp_path / "model.sem"
    model_file.write_text(model_spec)

    calibrator = CfaCalibrator(model_file=model_file, objective="MLW")
    result = calibrator.fit(path)

    assert result.factors is not None
    assert result.factors["latent_factors"] == ["f1", "f2", "f3"]


def test_cfa_calibrator_metadata(cfa_data_file):
    path, model_spec = cfa_data_file

    calibrator = CfaCalibrator(model=model_spec, objective="MLW")
    result = calibrator.fit(path)

    assert result.metadata["package"] == "semopy"
    assert result.metadata["objective"] == "MLW"
    assert "n_subjects" in result.metadata
    assert "timestamp" in result.metadata


def test_cfa_calibrator_rejects_no_model():
    with pytest.raises(ValueError, match="model"):
        CfaCalibrator(objective="MLW")
```

- [ ] **Step 3: Run tests to verify they fail**

```bash
cd python
uv run pytest tests/test_cfa.py -v
```

Expected: FAIL with `ModuleNotFoundError: No module named 'mojave_calibrate.cfa'`

- [ ] **Step 4: Write cfa.py**

Create `python/src/mojave_calibrate/cfa.py`:

```python
from __future__ import annotations

import logging
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

import numpy as np
import pandas as pd
import semopy

from mojave_calibrate.protocol import CalibrationResult

logger = logging.getLogger(__name__)


class CfaCalibrator:
    def __init__(
        self,
        model: str | None = None,
        model_file: Path | None = None,
        objective: str = "MLW",
    ) -> None:
        if model is None and model_file is None:
            raise ValueError("must provide either model string or model_file path")
        self._model_spec = model
        self._model_file = model_file
        self._objective = objective

    def name(self) -> str:
        return "cfa"

    def fit(self, data: Path, **kwargs: Any) -> CalibrationResult:
        spec = self._model_spec
        if spec is None and self._model_file is not None:
            spec = self._model_file.read_text()

        df = pd.read_csv(data)
        n_subjects = len(df)

        mod = semopy.Model(spec)
        mod.fit(df, obj=self._objective)

        estimates = mod.inspect()
        stats = semopy.calc_stats(mod)

        factor_names, loadings_matrix = _extract_loadings(estimates, spec)
        n_factors = len(factor_names)

        fit_indices = _extract_fit_indices(stats)

        cov = _extract_factor_covariance(estimates, factor_names)

        intercepts = _extract_intercepts(estimates, loadings_matrix)

        factors: dict[str, Any] = {
            "latent_factors": factor_names,
            "loadings": loadings_matrix,
            "intercepts": intercepts,
            "covariance": cov,
            "fit_indices": fit_indices,
        }

        metadata: dict[str, Any] = {
            "objective": self._objective,
            "n_subjects": n_subjects,
            "timestamp": datetime.now(timezone.utc).isoformat(),
            "package": "semopy",
            "package_version": _semopy_version(),
        }

        return CalibrationResult(items=None, factors=factors, metadata=metadata)


def _extract_loadings(
    estimates: pd.DataFrame, spec: str
) -> tuple[list[str], list[list[float]]]:
    measurement = estimates[estimates["op"] == "=~"].copy()

    factor_names: list[str] = []
    for line in spec.strip().splitlines():
        line = line.strip()
        if "=~" in line:
            fname = line.split("=~")[0].strip()
            if fname not in factor_names:
                factor_names.append(fname)

    observed_vars: list[str] = []
    for _, row in measurement.iterrows():
        rval = row["rval"]
        if rval not in observed_vars:
            observed_vars.append(rval)

    n_items = len(observed_vars)
    n_factors = len(factor_names)
    matrix = [[0.0] * n_factors for _ in range(n_items)]

    factor_idx = {f: i for i, f in enumerate(factor_names)}
    item_idx = {v: i for i, v in enumerate(observed_vars)}

    for _, row in measurement.iterrows():
        fi = factor_idx.get(row["lval"])
        ii = item_idx.get(row["rval"])
        if fi is not None and ii is not None:
            matrix[ii][fi] = float(row["Estimate"])

    return factor_names, matrix


def _extract_fit_indices(stats: pd.DataFrame) -> dict[str, float]:
    indices: dict[str, float] = {}
    for col in ["CFI", "RMSEA", "chi2", "DoF", "AIC", "BIC", "LogLik"]:
        if col in stats.columns:
            val = stats[col].iloc[0]
            key = "df" if col == "DoF" else col
            try:
                indices[key] = float(val)
            except (ValueError, TypeError):
                pass
    return indices


def _extract_factor_covariance(
    estimates: pd.DataFrame, factor_names: list[str]
) -> list[list[float]]:
    n = len(factor_names)
    cov = [[0.0] * n for _ in range(n)]
    for i in range(n):
        cov[i][i] = 1.0

    covariances = estimates[estimates["op"] == "~~"]
    factor_idx = {f: i for i, f in enumerate(factor_names)}

    for _, row in covariances.iterrows():
        li = factor_idx.get(row["lval"])
        ri = factor_idx.get(row["rval"])
        if li is not None and ri is not None and li != ri:
            val = float(row["Estimate"])
            cov[li][ri] = val
            cov[ri][li] = val

    return cov


def _extract_intercepts(
    estimates: pd.DataFrame, loadings_matrix: list[list[float]]
) -> list[float]:
    n_items = len(loadings_matrix)
    intercepts = estimates[estimates["op"] == "~"]
    if intercepts.empty:
        return [0.0] * n_items
    return [0.0] * n_items


def _semopy_version() -> str:
    try:
        from importlib.metadata import version
        return version("semopy")
    except Exception:
        return "unknown"
```

- [ ] **Step 5: Run tests to verify they pass**

```bash
cd python
uv run pytest tests/test_cfa.py -v
```

Expected: 4 passed. semopy is fast on CPU — should complete in seconds.

- [ ] **Step 6: Commit**

```bash
git add python/src/mojave_calibrate/cfa.py python/tests/conftest.py python/tests/test_cfa.py
git commit -m "feat(calibrate): add CfaCalibrator wrapping semopy for CFA/SEM"
```

---

### Task 7: ReEval stub

**Files:**
- Create: `python/src/mojave_calibrate/reeval_stub.py`

- [ ] **Step 1: Write reeval_stub.py**

Create `python/src/mojave_calibrate/reeval_stub.py`:

```python
"""Stub interface for Stanford AIMS REEval amortized calibration.

Not yet implemented — REEval is a research repo (not pip-installable) and
requires CUDA 12.2 + flash-attention + Llama 8B embeddings.

Expected input:
    Binary response matrix (subjects x items) as CSV, plus optional text
    embeddings CSV of shape (n_items, embed_dim) from a language model.

Expected output:
    Item easiness parameters (Rasch / 1PL) as item pool JSON. Note that
    REEval uses easiness (positive = easier), which must be negated to
    produce standard IRT difficulty (positive = harder).

See: https://github.com/aims-foundations/reeval
See: BEAD-0005 in .context/beads/
"""

from __future__ import annotations

from pathlib import Path
from typing import Any

from mojave_calibrate.protocol import CalibrationResult


class ReEvalCalibrator:
    def __init__(self, **kwargs: Any) -> None:
        self._kwargs = kwargs

    def name(self) -> str:
        return "reeval"

    def fit(self, data: Path, **kwargs: Any) -> CalibrationResult:
        raise NotImplementedError(
            "REEval integration is not yet implemented. "
            "See reeval_stub.py docstring for expected interface."
        )
```

- [ ] **Step 2: Commit**

```bash
git add python/src/mojave_calibrate/reeval_stub.py
git commit -m "feat(calibrate): add ReEvalCalibrator stub interface"
```

---

### Task 8: CLI and integration tests

**Files:**
- Create: `python/src/mojave_calibrate/cli.py`
- Create: `python/tests/test_cli.py`

- [ ] **Step 1: Write CLI integration tests**

Create `python/tests/test_cli.py`:

```python
from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path

import pytest
from click.testing import CliRunner

from mojave_calibrate.cli import main


@pytest.fixture()
def runner():
    return CliRunner()


class TestIrtSubcommand:
    def test_produces_valid_json(self, runner, irt_response_file, tmp_path):
        input_path, _ = irt_response_file
        output_path = tmp_path / "out.json"

        result = runner.invoke(
            main,
            [
                "irt",
                "--input", str(input_path),
                "--output", str(output_path),
                "--model-type", "2pl",
                "--epochs", "100",
                "--device", "cpu",
                "--content-domain", "test",
            ],
        )

        assert result.exit_code == 0, f"stderr: {result.output}"
        assert output_path.exists()

        with open(output_path) as f:
            data = json.load(f)

        assert "items" in data
        assert "calibration_metadata" in data
        assert len(data["items"]) > 0

    def test_missing_input_exits_nonzero(self, runner, tmp_path):
        result = runner.invoke(
            main,
            [
                "irt",
                "--input", str(tmp_path / "nonexistent.jsonl"),
                "--output", str(tmp_path / "out.json"),
                "--content-domain", "test",
                "--device", "cpu",
            ],
        )
        assert result.exit_code != 0


class TestFactorsSubcommand:
    def test_produces_valid_json(self, runner, factor_response_file, tmp_path):
        input_path, _ = factor_response_file
        output_path = tmp_path / "out.json"

        result = runner.invoke(
            main,
            [
                "factors",
                "--input", str(input_path),
                "--output", str(output_path),
                "--latent-size", "3",
                "--model-type", "grm",
                "--n-cats", "3",
                "--device", "cpu",
                "--max-epochs", "50",
                "--iw-samples", "5",
            ],
        )

        assert result.exit_code == 0, f"stderr: {result.output}"
        assert output_path.exists()

        with open(output_path) as f:
            data = json.load(f)

        assert "latent_factors" in data
        assert "calibration_metadata" in data


class TestCfaSubcommand:
    def test_produces_valid_json(self, runner, cfa_data_file, tmp_path):
        input_path, model_spec = cfa_data_file
        output_path = tmp_path / "out.json"

        result = runner.invoke(
            main,
            [
                "cfa",
                "--input", str(input_path),
                "--output", str(output_path),
                "--model", model_spec,
            ],
        )

        assert result.exit_code == 0, f"stderr: {result.output}"
        assert output_path.exists()

        with open(output_path) as f:
            data = json.load(f)

        assert "latent_factors" in data
        assert "fit_indices" in data

    def test_model_from_file(self, runner, cfa_data_file, tmp_path):
        input_path, model_spec = cfa_data_file
        model_file = tmp_path / "spec.sem"
        model_file.write_text(model_spec)
        output_path = tmp_path / "out.json"

        result = runner.invoke(
            main,
            [
                "cfa",
                "--input", str(input_path),
                "--output", str(output_path),
                "--model-file", str(model_file),
            ],
        )

        assert result.exit_code == 0, f"stderr: {result.output}"


class TestVerboseFlag:
    def test_verbose_produces_output(self, runner, cfa_data_file, tmp_path):
        input_path, model_spec = cfa_data_file
        output_path = tmp_path / "out.json"

        result = runner.invoke(
            main,
            [
                "--verbose",
                "cfa",
                "--input", str(input_path),
                "--output", str(output_path),
                "--model", model_spec,
            ],
        )

        assert result.exit_code == 0
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cd python
uv run pytest tests/test_cli.py -v
```

Expected: FAIL with `ImportError: cannot import name 'main' from 'mojave_calibrate.cli'`

- [ ] **Step 3: Write cli.py**

Create `python/src/mojave_calibrate/cli.py`:

```python
from __future__ import annotations

import logging
import sys
from pathlib import Path

import click

from mojave_calibrate.schema import write_factor_structure, write_item_pool

logger = logging.getLogger("mojave_calibrate")


@click.group()
@click.option("--verbose", is_flag=True, help="Enable debug logging to stderr.")
def main(verbose: bool) -> None:
    """mojave-calibrate: offline calibration pipeline for the mojave measurement engine."""
    level = logging.DEBUG if verbose else logging.WARNING
    logging.basicConfig(
        level=level,
        format="%(name)s %(levelname)s: %(message)s",
        stream=sys.stderr,
    )


@main.command()
@click.option("--input", "input_path", required=True, type=click.Path(exists=True, path_type=Path))
@click.option("--output", "output_path", required=True, type=click.Path(path_type=Path))
@click.option("--model-type", default="2pl", type=click.Choice(["1pl", "2pl", "4pl"]))
@click.option("--epochs", default=2000, type=int)
@click.option("--lr", default=0.1, type=float)
@click.option("--lr-decay", default=0.9999, type=float)
@click.option("--priors", default="vague", type=click.Choice(["vague", "hierarchical"]))
@click.option("--device", default="cpu", type=str)
@click.option("--seed", default=None, type=int)
@click.option("--content-domain", required=True, type=str)
def irt(
    input_path: Path,
    output_path: Path,
    model_type: str,
    epochs: int,
    lr: float,
    lr_decay: float,
    priors: str,
    device: str,
    seed: int | None,
    content_domain: str,
) -> None:
    """Fit IRT model via py-irt and emit item pool JSON."""
    from mojave_calibrate.irt import IrtCalibrator

    try:
        calibrator = IrtCalibrator(
            model_type=model_type,
            epochs=epochs,
            lr=lr,
            lr_decay=lr_decay,
            priors=priors,
            device=device,
            seed=seed,
            content_domain=content_domain,
        )
        result = calibrator.fit(input_path)
        write_item_pool(result.items, result.metadata, output_path)
        logger.info("wrote item pool to %s", output_path)
    except Exception as exc:
        logger.error("IRT calibration failed: %s", exc)
        raise SystemExit(2) from exc


@main.command()
@click.option("--input", "input_path", required=True, type=click.Path(exists=True, path_type=Path))
@click.option("--output", "output_path", required=True, type=click.Path(path_type=Path))
@click.option("--latent-size", required=True, type=int)
@click.option("--model-type", default="grm", type=click.Choice(["grm", "gpcm", "nominal"]))
@click.option("--n-cats", default=3, type=int)
@click.option("--q-matrix", default=None, type=click.Path(exists=True, path_type=Path))
@click.option("--device", default="cpu", type=str)
@click.option("--max-epochs", default=100_000, type=int)
@click.option("--iw-samples", default=5000, type=int)
@click.option("--seed", default=None, type=int)
@click.option("--factor-names", default=None, type=str, help="Comma-separated factor names.")
def factors(
    input_path: Path,
    output_path: Path,
    latent_size: int,
    model_type: str,
    n_cats: int,
    q_matrix: Path | None,
    device: str,
    max_epochs: int,
    iw_samples: int,
    seed: int | None,
    factor_names: str | None,
) -> None:
    """Fit factor model via deepirtools IWAVE and emit factor structure JSON."""
    from mojave_calibrate.factors import FactorCalibrator

    try:
        names_list = factor_names.split(",") if factor_names else None
        calibrator = FactorCalibrator(
            latent_size=latent_size,
            model_type=model_type,
            n_cats=n_cats,
            q_matrix_path=q_matrix,
            device=device,
            max_epochs=max_epochs,
            iw_samples=iw_samples,
            factor_names=names_list,
        )
        result = calibrator.fit(input_path)
        write_factor_structure(result.factors, result.metadata, output_path)
        logger.info("wrote factor structure to %s", output_path)
    except Exception as exc:
        logger.error("factor calibration failed: %s", exc)
        raise SystemExit(2) from exc


@main.command()
@click.option("--input", "input_path", required=True, type=click.Path(exists=True, path_type=Path))
@click.option("--output", "output_path", required=True, type=click.Path(path_type=Path))
@click.option("--model", "model_spec", default=None, type=str)
@click.option("--model-file", default=None, type=click.Path(exists=True, path_type=Path))
@click.option("--objective", default="MLW", type=click.Choice(["MLW", "FIML", "ULS", "GLS", "WLS", "DWLS"]))
def cfa(
    input_path: Path,
    output_path: Path,
    model_spec: str | None,
    model_file: Path | None,
    objective: str,
) -> None:
    """Fit CFA/SEM model via semopy and emit factor structure JSON."""
    from mojave_calibrate.cfa import CfaCalibrator

    try:
        calibrator = CfaCalibrator(
            model=model_spec,
            model_file=model_file,
            objective=objective,
        )
        result = calibrator.fit(input_path)
        write_factor_structure(result.factors, result.metadata, output_path)
        logger.info("wrote factor structure to %s", output_path)
    except Exception as exc:
        logger.error("CFA calibration failed: %s", exc)
        raise SystemExit(2) from exc
```

- [ ] **Step 4: Run tests to verify they pass**

```bash
cd python
uv run pytest tests/test_cli.py -v --timeout=180
```

Expected: all tests pass. IRT subcommand test may take longest due to py-irt
training.

- [ ] **Step 5: Run the full test suite**

```bash
cd python
uv run pytest -v --timeout=300
```

Expected: all tests across all files pass.

- [ ] **Step 6: Commit**

```bash
git add python/src/mojave_calibrate/cli.py python/tests/test_cli.py
git commit -m "feat(calibrate): add click CLI with irt/factors/cfa subcommands"
```
