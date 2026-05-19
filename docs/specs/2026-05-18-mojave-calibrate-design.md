# mojave-calibrate: Python Calibration Pipeline

## Goal

Offline calibration pipeline that fits IRT, factor, and CFA models from
response data using Python packages (py-irt, deepirtools, semopy), emitting
mojave-compatible JSON consumed by the Rust engine.

## Decisions

- **Subprocess + JSON boundary** — no PyO3. Reaffirmed 2026-05-18 in
  `decisions/2026-05-11-language-and-boundary-architecture.md`.
- **Offline calibration** — batch job, decoupled from Rust runtime. Python
  writes item parameter / factor structure JSON files. Rust reads them
  independently.
- **uv + lockfile** — deterministic, offline-capable via `--find-links` with
  pre-downloaded wheels.
- **Monorepo** — lives in `python/` at repo root alongside `crates/`.
- **reeval** — stub interface only. Full implementation deferred until it
  matures into an installable package.

## BEAD coverage

- BEAD-0005: IRT Python layer (py-irt wrapper → item pool JSON)
- BEAD-0006: Factor models Python (deepirtools + semopy → factor structure JSON)

---

## Architecture

```
python/
  pyproject.toml              # uv project, entry point: mojave-calibrate
  uv.lock
  src/
    mojave_calibrate/
      __init__.py
      cli.py                  # click CLI entry point
      protocol.py             # Calibrator protocol + CalibrationResult dataclass
      schema.py               # Output JSON validation + mojave format helpers
      irt.py                  # IrtCalibrator (wraps py-irt)
      factors.py              # FactorCalibrator (wraps deepirtools)
      cfa.py                  # CfaCalibrator (wraps semopy)
      reeval_stub.py          # Interface only, raises NotImplementedError
  tests/
    conftest.py
    test_irt.py
    test_factors.py
    test_cfa.py
    test_schema.py
    test_cli.py
```

## Output JSON contracts

### Item pool file (IRT → eval-design CAT engine)

Matches Rust `ItemPool` / `ItemMetadata` structs in `crates/eval-design/src/item_pool.rs`.

```json
{
  "items": [
    {
      "id": "task_001",
      "difficulty": 0.5,
      "discrimination": 1.2,
      "content_domain": "math",
      "exposure_count": 0
    }
  ],
  "calibration_metadata": {
    "model": "2pl",
    "n_subjects": 500,
    "n_items": 50,
    "timestamp": "2026-05-18T12:00:00Z",
    "package": "py-irt",
    "package_version": "0.7.1"
  }
}
```

Constraints enforced by `schema.py`:
- `discrimination` > 0 (mirrors Rust `ItemMetadata::new` validation)
- `difficulty` and `discrimination` must be finite
- `id` must be non-empty, unique across items

### Factor structure file (factor/CFA → future eval-orchestrator routing)

```json
{
  "latent_factors": ["reasoning", "code", "retrieval"],
  "loadings": [[0.8, 0.1, 0.0], [0.1, 0.9, 0.0]],
  "intercepts": [1.2, 0.8],
  "covariance": [[1.0, 0.3, 0.2], [0.3, 1.0, 0.1], [0.2, 0.1, 1.0]],
  "fit_indices": {
    "CFI": 0.95,
    "RMSEA": 0.04,
    "chi2": 45.2,
    "df": 24,
    "log_likelihood": -1234.5
  },
  "calibration_metadata": {
    "model_type": "grm",
    "latent_size": 3,
    "n_subjects": 500,
    "n_items": 50,
    "timestamp": "2026-05-18T12:00:00Z",
    "package": "deepirtools",
    "package_version": "0.3.0"
  }
}
```

Matrix dimensions: `loadings` is `n_items × latent_size`, `covariance` is
`latent_size × latent_size`, `intercepts` is `n_items` (continuous) or
`n_items × (n_categories - 1)` (ordinal).

---

## Calibrator protocol

```python
from dataclasses import dataclass
from pathlib import Path
from typing import Protocol

@dataclass(frozen=True)
class CalibrationResult:
    items: list[dict] | None       # item pool (IRT calibrators)
    factors: dict | None           # factor structure (factor/CFA calibrators)
    metadata: dict                 # calibration_metadata block

class Calibrator(Protocol):
    def fit(self, data: Path, **kwargs) -> CalibrationResult: ...
    def name(self) -> str: ...
```

Structural subtyping via `typing.Protocol`. No inheritance, no ABCs.

---

## Calibrator implementations

### IrtCalibrator (wraps py-irt)

- **Input**: JSONL response file — py-irt's native format:
  `{"subject_id": "s1", "responses": {"item1": 1, "item2": 0}}`
- **Config**: `model_type` (1pl/2pl/4pl), `epochs`, `lr`, `lr_decay`,
  `priors` (vague/hierarchical), `device` (cpu/cuda), `seed`
- **Fitting**: `IrtModelTrainer(config=IrtConfig(...), data_path=path).train(device=device)`
- **Output mapping**: py-irt `diff` → `difficulty`, `disc` → `discrimination`,
  `item_ids` int→string map → `id`. `content_domain` set via `--content-domain`
  flag (required).
- **Validation**: reject items where `disc` ≤ 0 (log warning, exclude from output)

### FactorCalibrator (wraps deepirtools)

- **Input**: CSV response matrix (subjects × items, integer-coded)
- **Config**: `latent_size`, `model_type` (grm/gpcm/nominal), `q_matrix` path
  (optional CSV, binary constraint matrix), `correlated_factors`,
  `device` (cpu/cuda), `max_epochs`, `iw_samples`
- **Fitting**: load CSV → `torch.Tensor`, construct `IWAVE(...)`, call
  `model.fit(data, iw_samples=iw_samples)`
- **Output mapping**: `model.loadings.tolist()` → `loadings`,
  `model.intercepts.tolist()` → `intercepts`,
  `model.cov.tolist()` → `covariance`,
  `model.log_likelihood(data)` → `fit_indices.log_likelihood`
- **Factor naming**: positional (`factor_0`, `factor_1`, ...) unless
  `--factor-names` provides explicit names

### CfaCalibrator (wraps semopy)

- **Input**: CSV data file (columns = observed variables)
- **Config**: model specification as lavaan-syntax string (`--model`) or file
  (`--model-file`), `objective` (MLW/FIML/ULS/GLS/WLS/DWLS)
- **Fitting**: `semopy.Model(desc).fit(data, obj=objective)`,
  `model.inspect()` for loadings, `semopy.calc_stats(model)` for fit indices
- **Output mapping**: `inspect()` DataFrame rows where `op == "=~"` →
  `loadings` matrix, `calc_stats()` → `fit_indices` (CFI, RMSEA, chi2, df,
  AIC, BIC, LogLik)
- **Factor naming**: from the lavaan-syntax LHS (e.g., `visual`, `textual`)
- **No GPU**

### ReEvalStub

- Implements `Calibrator` protocol
- `fit()` raises `NotImplementedError`
- Docstring documents expected input (binary response matrix + optional
  text embeddings) and output (Rasch item easiness parameters) for future
  implementation

---

## CLI

Entry point: `mojave-calibrate`, via click.

```
mojave-calibrate irt \
  --input responses.jsonl \
  --output item_pool.json \
  --model-type 2pl \
  --epochs 2000 \
  --device cpu \
  --seed 42 \
  --content-domain general

mojave-calibrate factors \
  --input responses.csv \
  --output factor_structure.json \
  --latent-size 3 \
  --model-type grm \
  --q-matrix q.csv \
  --device cpu \
  --seed 42 \
  --factor-names "reasoning,code,retrieval"

mojave-calibrate cfa \
  --input data.csv \
  --output factor_structure.json \
  --model "visual =~ x1 + x2 + x3; textual =~ x4 + x5 + x6" \
  --objective MLW
```

Alternative for CFA with complex specifications:
```
mojave-calibrate cfa \
  --input data.csv \
  --output factor_structure.json \
  --model-file spec.sem \
  --objective MLW
```

**Common flags**: `--input` (required), `--output` (required, JSON file path),
`--seed` (optional), `--verbose` (debug logging to stderr).

**Exit codes**: 0 success, 1 bad input/config, 2 model fitting failure.

**Logging**: Python `logging` to stderr. Quiet by default, `--verbose` for
debug. No structured JSON logging — that's Rust's domain.

**Output**: always to file via `--output`, never stdout.

---

## Dependencies

```toml
[project]
name = "mojave-calibrate"
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
```

PyTorch + Pyro are heavy. Offline installs via `uv pip install --find-links
./wheels/` with pre-downloaded wheels in a gitignored `wheels/` directory.

---

## Testing

pytest. No TCK/Gherkin on the Python side — behavioral contract is simple
enough that pytest test names are the spec.

### Unit tests (per calibrator)

Synthetic datasets with known generating parameters. Fit model, assert
recovered parameters correlate > 0.9 with ground truth.

- `test_irt.py`: 20-subject × 10-item binary matrix, known a/b params.
  Fit 2PL, check discrimination/difficulty recovery.
- `test_factors.py`: simulated factor structure (3 factors, 12 items),
  fit IWAVE, check loading pattern matches generating structure.
- `test_cfa.py`: Holzinger-Swineford-style synthetic data, fit CFA,
  check fit indices within expected ranges (CFI > 0.90, RMSEA < 0.08).

### Schema tests

- Validate every calibrator's output passes `schema.py` validation
- Confirm item pool JSON is parseable by `mojave analyze` (subprocess
  round-trip: generate → feed to Rust → no parse error)
- Test constraint enforcement: discrimination ≤ 0 rejected, NaN rejected,
  duplicate IDs rejected

### CLI integration tests

- Invoke `mojave-calibrate irt` as subprocess, verify exit 0 + valid JSON
- Missing input file → exit 1
- Garbage input → exit 1
- `--verbose` produces stderr output

### Not in scope for Python-side testing

MC calibration cards and statistical validation of estimators — that's the
upstream packages' responsibility. We test faithful translation, not
estimator correctness.

---

## Scope boundaries

**In scope**: fitting IRT/factor/CFA models, emitting mojave-compatible JSON,
CLI entry point, schema validation, tests.

**Out of scope**: consuming item pool files (Rust), live orchestration, reeval
implementation, GPU environment management, UI/reporting, any Rust code changes.
