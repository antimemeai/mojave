from __future__ import annotations

import contextlib
import logging
from datetime import UTC, datetime
from typing import TYPE_CHECKING, Any

import pandas as pd
import semopy

from mojave_calibrate.protocol import CalibrationResult

if TYPE_CHECKING:
    from pathlib import Path

logger = logging.getLogger(__name__)


class CfaCalibrator:
    def __init__(
        self,
        model: str | None = None,
        model_file: Path | None = None,
        objective: str = "MLW",
    ) -> None:
        if model is None and model_file is None:
            msg = "must provide either model string or model_file path"
            raise ValueError(msg)
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

        assert spec is not None
        factor_names, loadings_matrix = _extract_loadings(estimates, spec)

        fit_indices = _extract_fit_indices(stats)

        cov = _extract_factor_covariance(estimates, factor_names)

        intercepts = [0.0] * len(loadings_matrix)

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
            "timestamp": datetime.now(UTC).isoformat(),
            "package": "semopy",
            "package_version": _semopy_version(),
        }

        return CalibrationResult(items=None, factors=factors, metadata=metadata)


def _extract_loadings(estimates: pd.DataFrame, spec: str) -> tuple[list[str], list[list[float]]]:
    # semopy uses "~" for measurement (not "=~"), with lval=observed, rval=factor
    measurement = estimates[estimates["op"] == "~"].copy()

    factor_names: list[str] = []
    for line in spec.strip().splitlines():
        line = line.strip()
        if "=~" in line:
            fname = line.split("=~")[0].strip()
            if fname not in factor_names:
                factor_names.append(fname)

    observed_vars: list[str] = []
    for _, row in measurement.iterrows():
        lval = str(row["lval"])
        if lval not in observed_vars:
            observed_vars.append(lval)

    n_items = len(observed_vars)
    n_factors = len(factor_names)
    matrix = [[0.0] * n_factors for _ in range(n_items)]

    factor_idx = {f: i for i, f in enumerate(factor_names)}
    item_idx = {v: i for i, v in enumerate(observed_vars)}

    for _, row in measurement.iterrows():
        fi = factor_idx.get(str(row["rval"]))
        ii = item_idx.get(str(row["lval"]))
        if fi is not None and ii is not None:
            matrix[ii][fi] = float(row["Estimate"])

    return factor_names, matrix


def _extract_fit_indices(stats: pd.DataFrame) -> dict[str, float]:
    indices: dict[str, float] = {}
    for col in ["CFI", "RMSEA", "chi2", "DoF", "AIC", "BIC", "LogLik"]:
        if col in stats.columns:
            val = stats[col].iloc[0]
            key = "df" if col == "DoF" else col
            with contextlib.suppress(ValueError, TypeError):
                indices[key] = float(val)
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


def _semopy_version() -> str:
    try:
        from importlib.metadata import version

        return version("semopy")
    except Exception:
        return "unknown"
