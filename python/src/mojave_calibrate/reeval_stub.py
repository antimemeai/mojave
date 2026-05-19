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

from typing import TYPE_CHECKING, Any

if TYPE_CHECKING:
    from pathlib import Path

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
