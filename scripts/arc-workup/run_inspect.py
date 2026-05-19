#!/usr/bin/env python3
"""Orchestrate Inspect AI runs for each variant in the manifest.

Distributes variants round-robin across multiple vLLM endpoints for
parallel execution. Each endpoint runs variants sequentially via
subprocess, but multiple endpoints run concurrently via threads.

Requires: inspect-ai, inspect-evals, vLLM running as OpenAI-compatible server.
"""

from __future__ import annotations

import json
import os
import subprocess
import sys
from concurrent.futures import ThreadPoolExecutor, as_completed
from pathlib import Path


def build_inspect_cmd(
    variant: dict,
    output_dir: Path,
) -> list[str]:
    vid = variant["variant_id"]
    temp = variant["temperature"]
    order_seed = variant["order_seed"]
    prompt = variant["prompt_template"]

    cmd = [
        "inspect",
        "eval",
        "inspect_evals/arc_challenge",
        "--model",
        "openai/Qwen2.5-7B-Instruct",
        "-T",
        f"shuffle_seed={order_seed}",
        "--temperature",
        str(temp),
        "--log-dir",
        str(output_dir / vid),
        "--no-sandbox",
    ]

    if prompt != "default":
        cmd.extend(["-T", f"prompt_template={prompt}"])

    return cmd


def run_variant(
    variant: dict,
    base_url: str,
    output_dir: Path,
    index: int,
    total: int,
) -> tuple[str, bool, str]:
    vid = variant["variant_id"]
    cmd = build_inspect_cmd(variant, output_dir)
    env = os.environ.copy()
    env["OPENAI_BASE_URL"] = base_url
    env["OPENAI_API_KEY"] = "EMPTY"

    result = subprocess.run(cmd, capture_output=True, text=True, env=env)
    ok = result.returncode == 0
    err = result.stderr[:200] if not ok else ""
    status = "OK" if ok else f"FAILED: {err}"
    print(
        f"[{index}/{total}] {vid} -> {status}",
        file=sys.stderr,
    )
    return vid, ok, err


def main() -> None:
    manifest_path = (
        Path(sys.argv[1]) if len(sys.argv) > 1 else Path("scripts/arc-workup/manifest.json")
    )
    output_dir = Path(sys.argv[2]) if len(sys.argv) > 2 else Path("data/arc-workup/logs")

    # Remaining args are vLLM base URLs
    endpoints = sys.argv[3:] if len(sys.argv) > 3 else ["http://localhost:8000/v1"]

    manifest = json.loads(manifest_path.read_text())
    output_dir.mkdir(parents=True, exist_ok=True)

    variants = manifest["runs"]
    total = len(variants)
    n_workers = len(endpoints)

    print(
        f"Running {total} variants across {n_workers} endpoint(s)",
        file=sys.stderr,
    )

    # Distribute variants round-robin across endpoints
    batches: list[list[tuple[int, dict, str]]] = [[] for _ in range(n_workers)]
    for i, variant in enumerate(variants):
        endpoint = endpoints[i % n_workers]
        batches[i % n_workers].append((i + 1, variant, endpoint))

    failed: list[str] = []

    def run_batch(batch: list[tuple[int, dict, str]]) -> list[tuple[str, bool, str]]:
        results = []
        for idx, variant, endpoint in batch:
            results.append(run_variant(variant, endpoint, output_dir, idx, total))
        return results

    with ThreadPoolExecutor(max_workers=n_workers) as pool:
        futures = [pool.submit(run_batch, batch) for batch in batches]
        for future in as_completed(futures):
            for vid, ok, _err in future.result():
                if not ok:
                    failed.append(vid)

    if failed:
        print(f"\n{len(failed)} variants failed: {failed}", file=sys.stderr)
        sys.exit(1)
    else:
        print(f"\nAll {total} variants completed.", file=sys.stderr)


if __name__ == "__main__":
    main()
