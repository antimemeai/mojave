#!/usr/bin/env python3
"""Sobol' sensitivity analysis wrapper for v2 perturbation matrix results.

Calls mojave-gsa analyze (Rust binary) for Sobol' Si/STi and Borgonovo delta
with bootstrap CIs. Adds Wilson CIs on per-cell accuracy (simple math in Python).

IMPORTANT: No Python SALib imports. All GSA computation uses salib-rs via
the mojave-gsa binary.

Usage:
    python analyze_sobol.py <manifest.json> <results.json> <output.json>
      [--mojave-gsa-bin PATH] [--bootstrap-resamples 1000]
"""

from __future__ import annotations

import argparse
import json
import math
import subprocess
import sys
import tempfile
from pathlib import Path
from typing import Any

from repo import cargo_bin


def reconstruct_output_vector(cells: list[dict[str, Any]]) -> list[float]:
    """Sort cells by saltelli_index and extract accuracy vector."""
    sorted_cells = sorted(cells, key=lambda c: c["saltelli_index"])
    y: list[float] = []
    for cell in sorted_cells:
        if cell["accuracy"] is None:
            msg = (
                f"Cell {cell.get('cell_id', '?')} has missing accuracy -- "
                f"cannot compute Sobol' indices with incomplete data"
            )
            raise ValueError(msg)
        y.append(cell["accuracy"])
    return y


def compute_wilson_ci(
    k: int,
    n: int,
    z: float = 1.96,
) -> tuple[float, float, float]:
    """Wilson score interval for binomial proportion."""
    if n == 0:
        return 0.0, 0.0, 0.0
    p_hat = k / n
    denom = 1 + z * z / n
    center = (p_hat + z * z / (2 * n)) / denom
    margin = z * math.sqrt(p_hat * (1 - p_hat) / n + z * z / (4 * n * n)) / denom
    return p_hat, max(0.0, center - margin), min(1.0, center + margin)


def run_mojave_gsa_analyze(
    manifest_path: Path,
    results_path: Path,
    output_path: Path,
    mojave_gsa_bin: str = "mojave-gsa",
    bootstrap_resamples: int = 1000,
    confidence_level: float = 0.95,
    seed: str = "mojave-gsa-default-seed-v1",
) -> dict[str, Any]:
    """Call mojave-gsa analyze and return the analysis JSON."""
    cmd = [
        mojave_gsa_bin,
        "analyze",
        "--manifest",
        str(manifest_path),
        "--results",
        str(results_path),
        "--output",
        str(output_path),
        "--bootstrap-resamples",
        str(bootstrap_resamples),
        "--confidence-level",
        str(confidence_level),
        "--seed",
        seed,
    ]

    result = subprocess.run(
        cmd,
        capture_output=True,
        text=True,
        timeout=300,
    )

    if result.returncode != 0:
        print(f"mojave-gsa analyze failed:\n{result.stderr}", file=sys.stderr)
        msg = f"mojave-gsa analyze exited with code {result.returncode}"
        raise RuntimeError(msg)

    return json.loads(output_path.read_text())  # type: ignore[no-any-return]


def analyze(
    manifest_path: Path,
    results_path: Path,
    mojave_gsa_bin: str = "mojave-gsa",
    bootstrap_resamples: int = 1000,
    confidence_level: float = 0.95,
    seed: str = "mojave-gsa-default-seed-v1",
) -> dict[str, Any]:
    """Run full analysis: mojave-gsa for GSA + Wilson CIs in Python."""
    results = json.loads(results_path.read_text())

    with tempfile.NamedTemporaryFile(suffix=".json", delete=False) as tmp:
        gsa_output_path = Path(tmp.name)

    gsa_analysis: dict[str, Any] = run_mojave_gsa_analyze(
        manifest_path,
        results_path,
        gsa_output_path,
        mojave_gsa_bin=mojave_gsa_bin,
        bootstrap_resamples=bootstrap_resamples,
        confidence_level=confidence_level,
        seed=seed,
    )

    gsa_output_path.unlink(missing_ok=True)

    cell_wilson_cis = []
    for cell in results["cells"]:
        if cell["accuracy"] is not None and cell.get("n_samples", 0) > 0:
            n_samples = cell["n_samples"]
            k_correct = round(cell["accuracy"] * n_samples)
            p, lo, hi = compute_wilson_ci(k_correct, n_samples)
            cell_wilson_cis.append(
                {
                    "cell_id": cell["cell_id"],
                    "accuracy": cell["accuracy"],
                    "wilson_ci_low": round(lo, 6),
                    "wilson_ci_high": round(hi, 6),
                    "n_samples": n_samples,
                }
            )

    gsa_analysis["cell_wilson_cis"] = cell_wilson_cis

    return gsa_analysis


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("manifest", type=Path)
    parser.add_argument("results", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument(
        "--mojave-gsa-bin",
        default=cargo_bin("mojave-gsa"),
        help="Path to mojave-gsa binary (default: auto-detected from repo)",
    )
    parser.add_argument("--bootstrap-resamples", type=int, default=1000)
    parser.add_argument("--confidence-level", type=float, default=0.95)
    parser.add_argument(
        "--seed",
        default="mojave-gsa-default-seed-v1",
        help="RNG seed (must match manifest generation seed)",
    )
    args = parser.parse_args()

    analysis = analyze(
        args.manifest,
        args.results,
        mojave_gsa_bin=args.mojave_gsa_bin,
        bootstrap_resamples=args.bootstrap_resamples,
        confidence_level=args.confidence_level,
        seed=args.seed,
    )

    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(analysis, indent=2) + "\n")

    print(f"\n  {analysis['eval']} Sobol' analysis:", file=sys.stderr)
    print(f"    Cells: {analysis['n_cells']}", file=sys.stderr)
    print(f"    Spread: {analysis['aggregate']['spread']}", file=sys.stderr)
    print(
        f"    Sum S1: {analysis['sobol_diagnostics']['sum_s1']}",
        file=sys.stderr,
    )
    print(
        f"    Sum ST: {analysis['sobol_diagnostics']['sum_st']}",
        file=sys.stderr,
    )
    if analysis["sobol_indices"]:
        dominant = analysis["sobol_indices"][0]
        print(
            f"    Dominant factor: {dominant['axis']} (ST={dominant['ST']:.4f})",
            file=sys.stderr,
        )
    print(f"    -> {args.output}", file=sys.stderr)


if __name__ == "__main__":
    main()
