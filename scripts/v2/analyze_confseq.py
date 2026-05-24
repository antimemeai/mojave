#!/usr/bin/env python3
"""Retrospective confseq CI-width stopping analysis wrapper.

Calls mojave-gsa confseq (Rust binary) to run permutation-based
retrospective stopping analysis using anytime-valid confidence sequences.

Usage:
    python analyze_confseq.py <results.json> <output.json>
"""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
from pathlib import Path
from typing import Any


def run_mojave_gsa_confseq(
    results_path: Path,
    output_path: Path,
    mojave_gsa_bin: str = "mojave-gsa",
    half_width_threshold: float = 0.02,
    alpha: float = 0.05,
    n_permutations: int = 1000,
    seed: int = 42,
) -> dict[str, Any]:
    """Call mojave-gsa confseq and return the analysis JSON."""
    cmd = [
        mojave_gsa_bin,
        "confseq",
        "--results",
        str(results_path),
        "--output",
        str(output_path),
        "--half-width-threshold",
        str(half_width_threshold),
        "--alpha",
        str(alpha),
        "--n-permutations",
        str(n_permutations),
        "--seed",
        str(seed),
    ]

    result = subprocess.run(
        cmd,
        capture_output=True,
        text=True,
        timeout=600,
    )

    if result.returncode != 0:
        print(f"mojave-gsa confseq failed:\n{result.stderr}", file=sys.stderr)
        msg = f"mojave-gsa confseq exited with code {result.returncode}"
        raise RuntimeError(msg)

    if result.stderr:
        print(result.stderr, file=sys.stderr, end="")

    return json.loads(output_path.read_text())  # type: ignore[no-any-return]


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("results", type=Path, help="Results JSON with item_matrix")
    parser.add_argument("output", type=Path, help="Output JSON path")
    parser.add_argument(
        "--mojave-gsa-bin",
        default="mojave-gsa",
    )
    parser.add_argument("--half-width-threshold", type=float, default=0.02)
    parser.add_argument("--alpha", type=float, default=0.05)
    parser.add_argument("--n-permutations", type=int, default=1000)
    parser.add_argument("--seed", type=int, default=42)
    args = parser.parse_args()

    args.output.parent.mkdir(parents=True, exist_ok=True)

    analysis = run_mojave_gsa_confseq(
        args.results,
        args.output,
        mojave_gsa_bin=args.mojave_gsa_bin,
        half_width_threshold=args.half_width_threshold,
        alpha=args.alpha,
        n_permutations=args.n_permutations,
        seed=args.seed,
    )

    agg = analysis["aggregate"]
    print(f"\n  Confseq stopping analysis: {analysis['eval']}", file=sys.stderr)
    print(f"    Cells: {analysis['n_cells']}", file=sys.stderr)
    print(
        f"    Early stop: {agg['cells_with_early_stop']}/{agg['total_cells']}",
        file=sys.stderr,
    )
    print(
        f"    Median stopping N: {agg['median_stopping_n']:.0f}"
        f" [{agg['iqr_low']:.0f}, {agg['iqr_high']:.0f}]",
        file=sys.stderr,
    )
    print(
        f"    Frac stopped at half: {agg['frac_stopped_half']:.3f}",
        file=sys.stderr,
    )
    print(f"    -> {args.output}", file=sys.stderr)


if __name__ == "__main__":
    main()
