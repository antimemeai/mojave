#!/usr/bin/env python3
"""Orchestrate Inspect AI runs for each variant in the manifest.

Reads manifest.json, runs `inspect eval` for each variant with the
correct temperature, shuffle seed, and prompt template. Collects
.eval log files into an output directory.

Requires: inspect-ai, inspect-evals, vLLM running as OpenAI-compatible server.
"""

from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path


def build_inspect_cmd(
    variant: dict,
    model_url: str,
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
        model_url,
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


def main() -> None:
    manifest_path = (
        Path(sys.argv[1]) if len(sys.argv) > 1 else Path("scripts/arc-workup/manifest.json")
    )
    output_dir = Path(sys.argv[2]) if len(sys.argv) > 2 else Path("data/arc-workup/logs")
    model_url = sys.argv[3] if len(sys.argv) > 3 else "openai/Qwen2.5-7B-Instruct"

    manifest = json.loads(manifest_path.read_text())
    output_dir.mkdir(parents=True, exist_ok=True)

    total = manifest["total_variants"]
    failed: list[str] = []

    for i, variant in enumerate(manifest["runs"]):
        vid = variant["variant_id"]
        print(
            f"[{i + 1}/{total}] Running {vid} (temp={variant['temperature']}, "
            f"order={variant['order_permutation_index']}, prompt={variant['prompt_template']})",
            file=sys.stderr,
        )

        cmd = build_inspect_cmd(variant, model_url, output_dir)
        result = subprocess.run(cmd, capture_output=True, text=True)

        if result.returncode != 0:
            print(f"  FAILED: {result.stderr[:200]}", file=sys.stderr)
            failed.append(vid)
        else:
            print("  OK", file=sys.stderr)

    if failed:
        print(f"\n{len(failed)} variants failed: {failed}", file=sys.stderr)
        sys.exit(1)
    else:
        print(f"\nAll {total} variants completed.", file=sys.stderr)


if __name__ == "__main__":
    main()
