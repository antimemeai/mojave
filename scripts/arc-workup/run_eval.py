#!/usr/bin/env python3
"""Run any Inspect eval from a manifest across multiple vLLM endpoints.

Resumes automatically — skips variants with existing .eval files on disk.

Usage:
    python run_eval.py [--limit N] <manifest.json> <output_dir> <endpoint1> [endpoint2] ...

The --limit flag caps each variant at N items via Inspect's --limit flag.
See sampling-method.md in the output directory for documentation of item
selection when --limit is used.
"""

from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
from concurrent.futures import ThreadPoolExecutor, as_completed
from pathlib import Path


def variant_complete(output_dir: Path, vid: str) -> bool:
    log_dir = output_dir / vid
    if not log_dir.exists():
        return False
    return any(f.suffix == ".eval" for f in log_dir.iterdir())


def build_inspect_cmd(
    task: str,
    variant: dict,
    output_dir: Path,
    limit: int | None = None,
) -> list[str]:
    vid = variant["variant_id"]
    temp = variant["temperature"]
    order_seed = variant["order_seed"]

    cmd = [
        "inspect",
        "eval",
        task,
        "--model",
        "openai/Qwen/Qwen2.5-7B-Instruct",
        "-T",
        f"shuffle_seed={order_seed}",
        "--temperature",
        str(temp),
        "--log-dir",
        str(output_dir / vid),
    ]

    if limit is not None:
        cmd.extend(["--limit", str(limit)])

    if "prompt_template" in variant and variant["prompt_template"] != "default":
        cmd.extend(["-T", f"prompt_template={variant['prompt_template']}"])

    return cmd


def run_variant(
    task: str,
    variant: dict,
    base_url: str,
    output_dir: Path,
    index: int,
    total: int,
    limit: int | None = None,
) -> tuple[str, bool, str]:
    vid = variant["variant_id"]

    if variant_complete(output_dir, vid):
        print(f"[{index}/{total}] {vid} -> SKIP", file=sys.stderr)
        return vid, True, ""

    cmd = build_inspect_cmd(task, variant, output_dir, limit=limit)
    env = os.environ.copy()
    env["OPENAI_BASE_URL"] = base_url
    env["OPENAI_API_KEY"] = "EMPTY"

    result = subprocess.run(cmd, capture_output=True, text=True, env=env)
    ok = result.returncode == 0
    err = result.stderr[:200] if not ok else ""
    status = "OK" if ok else f"FAILED: {err}"
    print(f"[{index}/{total}] {vid} -> {status}", file=sys.stderr)
    return vid, ok, err


def write_sampling_method(output_dir: Path, task: str, limit: int, manifest_path: Path) -> None:
    """Write machine- and human-readable documentation of item selection method."""
    doc = {
        "task": task,
        "limit": limit,
        "method": "inspect_first_n",
        "description": (
            f"Each variant evaluates the first {limit} items returned by "
            f"Inspect AI's dataset loader for task '{task}'. Inspect loads "
            f"the dataset from HuggingFace Hub in its canonical split order "
            f"and applies --limit {limit} as a head-of-list truncation "
            f"(NOT random sampling). All variants evaluate the SAME {limit} "
            f"items — the perturbation dimensions are temperature and "
            f"sampling stochasticity, not item selection."
        ),
        "reproducibility": (
            "Deterministic given the same inspect-evals package version "
            "and HuggingFace dataset revision. The dataset order is fixed "
            "by the upstream dataset; Inspect does not shuffle before limiting."
        ),
        "manifest": str(manifest_path),
    }
    method_path = output_dir / "sampling-method.json"
    method_path.write_text(json.dumps(doc, indent=2) + "\n")
    print(f"Sampling method documented -> {method_path}", file=sys.stderr)


def main() -> None:
    parser = argparse.ArgumentParser(description="Run Inspect eval variants across vLLM endpoints")
    parser.add_argument("manifest", help="Path to manifest JSON")
    parser.add_argument("output_dir", help="Output directory for eval logs")
    parser.add_argument("endpoints", nargs="+", help="vLLM endpoint URLs")
    parser.add_argument(
        "--limit",
        type=int,
        default=None,
        help="Cap each variant at N items (Inspect --limit). Documents selection method.",
    )
    args = parser.parse_args()

    manifest_path = Path(args.manifest)
    output_dir = Path(args.output_dir)
    endpoints = args.endpoints
    limit = args.limit

    manifest = json.loads(manifest_path.read_text())
    task = manifest["task"]
    output_dir.mkdir(parents=True, exist_ok=True)

    if limit is not None:
        write_sampling_method(output_dir, task, limit, manifest_path)

    variants = manifest["runs"]
    total = len(variants)
    n_workers = len(endpoints)

    already = sum(1 for v in variants if variant_complete(output_dir, v["variant_id"]))
    remaining = total - already

    limit_str = f", --limit {limit}" if limit else ""
    print(
        f"{task}: {already} done, {remaining} remaining, {n_workers} endpoint(s){limit_str}",
        file=sys.stderr,
    )

    batches: list[list[tuple[int, dict, str]]] = [[] for _ in range(n_workers)]
    for i, variant in enumerate(variants):
        endpoint = endpoints[i % n_workers]
        batches[i % n_workers].append((i + 1, variant, endpoint))

    failed: list[str] = []

    def run_batch(batch: list[tuple[int, dict, str]]) -> list[tuple[str, bool, str]]:
        results = []
        for idx, variant, endpoint in batch:
            results.append(
                run_variant(task, variant, endpoint, output_dir, idx, total, limit=limit)
            )
        return results

    with ThreadPoolExecutor(max_workers=n_workers) as pool:
        futures = [pool.submit(run_batch, batch) for batch in batches]
        for future in as_completed(futures):
            for vid, ok, _err in future.result():
                if not ok:
                    failed.append(vid)

    if failed:
        print(f"\n{task}: {len(failed)} variants failed: {failed}", file=sys.stderr)
        sys.exit(1)
    else:
        print(f"\n{task}: all {total} variants completed.", file=sys.stderr)


if __name__ == "__main__":
    main()
