"""Generate frozen ANOVA reference CSVs for the deterministic fixtures.

Uses SciPy's linear-algebra solver as an external oracle rather than
replaying the Rust mean-table formulas directly. The factorial fixtures
are balanced and orthogonally contrast-coded, so each term's projection
energy yields its variance fraction.

This script is not run in CI; it is the audit-trail companion for the
frozen CSVs committed alongside it.
"""

from __future__ import annotations

import csv
from pathlib import Path
from typing import Any, cast

import numpy as np
from scipy import linalg

ROOT = Path(__file__).resolve().parent


def two_way_grid() -> np.ndarray:
    return np.array([[9.0, 5.0], [7.0, 19.0]], dtype=float)


def three_way_grid() -> np.ndarray:
    grid = np.zeros((2, 2, 2), dtype=float)
    levels = (-1.0, 1.0)
    for i, a in enumerate(levels):
        for j, b in enumerate(levels):
            for k, c in enumerate(levels):
                grid[i, j, k] = (
                    50.0
                    + 5.0 * a
                    + 3.0 * b
                    + 2.0 * c
                    + 4.0 * a * b
                    + 1.5 * a * c
                    + 1.0 * b * c
                    + 2.5 * a * b * c
                )
    return grid


def write_csv(name: str, rows: list[tuple[str, float]]) -> None:
    with (ROOT / name).open("w", newline="") as handle:
        writer = csv.writer(handle, lineterminator="\n")
        writer.writerow(["component", "variance_fraction"])
        writer.writerows(rows)


def estimate_two_way_via_lstsq(grid: np.ndarray) -> list[tuple[str, float]]:
    y = grid.reshape(-1)
    a = np.array([-1.0, -1.0, 1.0, 1.0])
    b = np.array([-1.0, 1.0, -1.0, 1.0])
    design = np.column_stack([np.ones(4), a, b, a * b])
    beta = cast("Any", linalg.lstsq)(design, y)[0]
    centered = y - y.mean()
    ss_total = float(np.dot(centered, centered))
    return [
        ("row", float(np.dot(a * beta[1], a * beta[1]) / ss_total)),
        ("column", float(np.dot(b * beta[2], b * beta[2]) / ss_total)),
        ("interaction", float(np.dot((a * b) * beta[3], (a * b) * beta[3]) / ss_total)),
        ("residual", 0.0),
    ]


def estimate_three_way_via_lstsq(grid: np.ndarray) -> list[tuple[str, float]]:
    rows: list[list[float]] = []
    y: list[float] = []
    levels = (-1.0, 1.0)
    for i, a in enumerate(levels):
        for j, b in enumerate(levels):
            for k, c in enumerate(levels):
                rows.append([a, b, c, a * b, a * c, b * c, a * b * c])
                y.append(float(grid[i, j, k]))
    y_arr = np.array(y, dtype=float)
    effects = np.array(rows, dtype=float).T
    design = np.column_stack([np.ones(len(y_arr)), effects.T])
    beta = cast("Any", linalg.lstsq)(design, y_arr)[0]
    centered = y_arr - y_arr.mean()
    ss_total = float(np.dot(centered, centered))
    names = [
        "data",
        "brittleness",
        "inference",
        "data_brittleness",
        "data_inference",
        "brittleness_inference",
        "data_brittleness_inference",
    ]
    rows_out = []
    for name, column, coef in zip(names, effects, beta[1:], strict=False):
        contribution = column * coef
        rows_out.append((name, float(np.dot(contribution, contribution) / ss_total)))
    rows_out.append(("residual", 0.0))
    return rows_out


def main() -> None:
    write_csv("anova_two_way_reference.csv", estimate_two_way_via_lstsq(two_way_grid()))
    write_csv(
        "anova_three_way_reference.csv",
        estimate_three_way_via_lstsq(three_way_grid()),
    )


if __name__ == "__main__":
    main()
