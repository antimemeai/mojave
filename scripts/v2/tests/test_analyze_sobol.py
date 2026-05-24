"""Tests for Sobol' index analysis wrapper."""

import sys
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).parent.parent))

from analyze_sobol import (
    compute_wilson_ci,
    reconstruct_output_vector,
)


class TestOutputVector:
    def test_output_ordered_by_saltelli_index(self) -> None:
        cells = [
            {"saltelli_index": 2, "accuracy": 0.8},
            {"saltelli_index": 0, "accuracy": 0.5},
            {"saltelli_index": 1, "accuracy": 0.6},
        ]
        y = reconstruct_output_vector(cells)
        assert y == [0.5, 0.6, 0.8]

    def test_missing_accuracy_raises(self) -> None:
        cells = [
            {"saltelli_index": 0, "accuracy": None},
        ]
        with pytest.raises(ValueError, match="missing accuracy"):
            reconstruct_output_vector(cells)


class TestWilsonCI:
    def test_perfect_score(self) -> None:
        p, lo, hi = compute_wilson_ci(100, 100)
        assert p == 1.0
        assert lo > 0.95
        assert hi > 0.99

    def test_zero_n(self) -> None:
        p, lo, hi = compute_wilson_ci(0, 0)
        assert p == 0.0

    def test_half_score(self) -> None:
        p, lo, hi = compute_wilson_ci(50, 100)
        assert abs(p - 0.5) < 1e-10
        assert lo < 0.5
        assert hi > 0.5

    def test_ci_bounds_valid(self) -> None:
        p, lo, hi = compute_wilson_ci(30, 100)
        assert 0.0 <= lo <= p
        assert p <= hi <= 1.0
