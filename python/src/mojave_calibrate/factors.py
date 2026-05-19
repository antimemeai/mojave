from __future__ import annotations

import csv
import logging
from datetime import UTC, datetime
from typing import TYPE_CHECKING, Any

import torch
from deepirtools import IWAVE

from mojave_calibrate.protocol import CalibrationResult

if TYPE_CHECKING:
    from pathlib import Path

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
            "timestamp": datetime.now(UTC).isoformat(),
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
