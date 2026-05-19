# mojave-calibrate

Offline calibration pipeline for the mojave measurement engine.

Wraps [py-irt](https://github.com/nd-ball/py-irt),
[deepirtools](https://github.com/cjangelo/deepirtools), and
[semopy](https://semopy.com) behind a unified `Calibrator` protocol,
validates output against mojave's schema, and emits JSON consumed
by the Rust engine.

## Installation

Requires Python 3.10+ and [uv](https://docs.astral.sh/uv/).

```bash
cd python
uv sync --group dev
```

## Usage

### CLI

```bash
# 2PL IRT calibration — emit item pool JSON
mojave-calibrate irt \
    --input responses.jsonl \
    --output item_pool.json \
    --model-type 2pl \
    --content-domain reasoning \
    --device cpu

# Multidimensional factor model (GRM) via deepirtools
mojave-calibrate factors \
    --input responses.csv \
    --output factors.json \
    --latent-size 3 \
    --model-type grm \
    --n-cats 3

# CFA/SEM via semopy
mojave-calibrate cfa \
    --input data.csv \
    --output cfa.json \
    --model "f1 =~ x1 + x2 + x3
f2 =~ x4 + x5 + x6"

# Or load model spec from a file
mojave-calibrate cfa \
    --input data.csv \
    --output cfa.json \
    --model-file model.sem
```

Add `--verbose` before any subcommand for debug logging to stderr.

### Python API

```python
from pathlib import Path
from mojave_calibrate.irt import IrtCalibrator
from mojave_calibrate.schema import write_item_pool

calibrator = IrtCalibrator(
    model_type="2pl",
    epochs=2000,
    device="cpu",
    content_domain="reasoning",
)
result = calibrator.fit(Path("responses.jsonl"))

# result.items  — list of item dicts (id, difficulty, discrimination, ...)
# result.metadata — fitting metadata (package, timestamp, n_subjects, ...)

write_item_pool(result.items, result.metadata, Path("pool.json"))
```

All calibrators implement the same protocol:

```python
class Calibrator(Protocol):
    def fit(self, data: Path, **kwargs) -> CalibrationResult: ...
    def name(self) -> str: ...
```

## Calibrators

### IrtCalibrator

Bayesian IRT via [py-irt](https://github.com/nd-ball/py-irt).
Supports 1PL, 2PL, and 4PL models. GPU-accelerated via Pyro.

**Input:** JSONL with `subject_id` and `responses` (dict of item_id to 0/1).

**Output:** Item pool with `id`, `difficulty`, `discrimination`,
`content_domain`, `exposure_count`. Items with non-positive
discrimination are filtered automatically.

### FactorCalibrator

Multidimensional IRT and factor models via
[deepirtools](https://github.com/cjangelo/deepirtools) IWAVE.
Supports GRM, GPCM, and nominal response models.

**Input:** CSV with ordinal response data (integer categories).

**Output:** Factor structure with `latent_factors`, `loadings` matrix,
`intercepts`, `covariance` matrix, and `fit_indices`.

### CfaCalibrator

Confirmatory factor analysis and structural equation modeling via
[semopy](https://semopy.com). Supports MLW, FIML, ULS, GLS, WLS,
and DWLS objective functions.

**Input:** CSV with continuous data. Model specified as lavaan-style
syntax (e.g., `f1 =~ x1 + x2 + x3`).

**Output:** Factor structure with loadings, covariance, fit indices
(CFI, RMSEA, chi2, AIC, BIC, etc.).

### ReEvalCalibrator (stub)

Placeholder for [Stanford AIMS REEval](https://github.com/aims-foundations/reeval)
amortized calibration. Not yet implemented — REEval is a research repo
requiring CUDA 12.2 + flash-attention + Llama 8B embeddings.

## Output formats

### Item pool JSON

```json
{
  "items": [
    {
      "id": "item_03",
      "difficulty": -0.52,
      "discrimination": 1.23,
      "content_domain": "reasoning",
      "exposure_count": 0
    }
  ],
  "calibration_metadata": {
    "model_type": "2pl",
    "package": "py-irt",
    "timestamp": "2026-05-19T..."
  }
}
```

### Factor structure JSON

```json
{
  "latent_factors": ["f1", "f2", "f3"],
  "loadings": [[0.8, 0.0, 0.0], [0.7, 0.0, 0.0], ...],
  "intercepts": [0.0, 0.0, ...],
  "covariance": [[1.0, 0.3, 0.1], [0.3, 1.0, 0.2], [0.1, 0.2, 1.0]],
  "fit_indices": {"CFI": 0.98, "RMSEA": 0.04},
  "calibration_metadata": {
    "package": "semopy",
    "timestamp": "2026-05-19T..."
  }
}
```

Both formats are validated by `schema.py` before writing.
The Rust engine (`eval-design`, `mojave-cli`) consumes these
files directly.

## Testing

```bash
uv run pytest -v              # all 31 tests
uv run pytest tests/test_irt.py -v   # just IRT
uv run pytest tests/test_cli.py -v   # CLI integration
```

## Pre-commit

Python files are checked by the repo's pre-commit hooks:

- **ruff format** — code formatting
- **ruff check** — linting (including import sorting, type-checking blocks)
- **mypy** — strict type checking

## License

MIT OR Apache-2.0
