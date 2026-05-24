#!/usr/bin/env python3
"""Run destructive perturbation variants across multiple vLLM endpoints.

Same architecture as run_eval.py but passes all perturbation dimensions
to the destructive_task wrapper via -T flags.

Usage:
    python run_destructive.py [--limit N] <manifest.json> <output_dir> <endpoint1> [endpoint2] ...
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
    variant: dict,
    base_task: str,
    base_url: str,
    output_dir: Path,
    limit: int | None = None,
) -> list[str]:
    vid = variant["variant_id"]

    cmd = [
        "inspect",
        "eval",
        "scripts/destructive/destructive_task.py@destructive",
        "--model",
        "openai/Qwen/Qwen2.5-7B-Instruct",
        "--model-base-url",
        base_url,
        "-T",
        f"base_task={base_task}",
        "-T",
        f"shuffle_seed={variant['order_seed']}",
        "-T",
        f"system_prompt={variant['system_prompt']}",
        "-T",
        f"prompt_template={variant['prompt_template']}",
        "-T",
        f"few_shot={variant['few_shot']}",
        "-T",
        f"label_format={variant['label_format']}",
        "--temperature",
        str(variant["temperature"]),
        "--log-dir",
        str(output_dir / vid),
    ]

    if limit is not None:
        cmd.extend(["--limit", str(limit)])

    return cmd


def run_variant(
    variant: dict,
    base_task: str,
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

    cmd = build_inspect_cmd(variant, base_task, base_url, output_dir, limit=limit)
    env = os.environ.copy()
    env["OPENAI_BASE_URL"] = base_url
    env["OPENAI_API_KEY"] = "EMPTY"

    result = subprocess.run(cmd, capture_output=True, text=True, env=env)
    ok = result.returncode == 0
    err = result.stderr[:300] if not ok else ""
    status = "OK" if ok else f"FAILED: {err}"
    print(f"[{index}/{total}] {vid} ({variant['description']}) -> {status}", file=sys.stderr)
    return vid, ok, err


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("manifest", help="Path to manifest JSON")
    parser.add_argument("output_dir", help="Output directory for eval logs")
    parser.add_argument("endpoints", nargs="*", help="vLLM endpoint URLs")
    parser.add_argument("--limit", type=int, default=None)
    parser.add_argument(
        "--endpoints-file", type=Path, default=Path("data/destructive/endpoints.json")
    )
    args = parser.parse_args()

    manifest_path = Path(args.manifest)
    output_dir = Path(args.output_dir)
    limit = args.limit

    endpoints = args.endpoints or json.loads(args.endpoints_file.read_text())
    if not endpoints:
        print("No endpoints provided and none in endpoints.json", file=sys.stderr)
        sys.exit(1)

    manifest = json.loads(manifest_path.read_text())
    base_task = manifest["task"]
    output_dir.mkdir(parents=True, exist_ok=True)

    variants = manifest["runs"]
    total = len(variants)
    n_workers = len(endpoints)

    already = sum(1 for v in variants if variant_complete(output_dir, v["variant_id"]))
    remaining = total - already

    limit_str = f", --limit {limit}" if limit else ""
    print(
        f"destructive: {base_task} | {already} done, {remaining} remaining, "
        f"{n_workers} endpoint(s){limit_str}",
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
                run_variant(variant, base_task, endpoint, output_dir, idx, total, limit=limit)
            )
        return results

    with ThreadPoolExecutor(max_workers=n_workers) as pool:
        futures = [pool.submit(run_batch, batch) for batch in batches]
        for future in as_completed(futures):
            for vid, ok, _err in future.result():
                if not ok:
                    failed.append(vid)

    if failed:
        print(f"\n{base_task}: {len(failed)} variants failed: {failed}", file=sys.stderr)
        sys.exit(1)
    else:
        print(f"\n{base_task}: all {total} variants completed.", file=sys.stderr)


if __name__ == "__main__":
    main()
