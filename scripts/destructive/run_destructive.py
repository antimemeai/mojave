#!/usr/bin/env python3
"""Run destructive perturbation variants across multiple vLLM endpoints.

Usage:
    python run_destructive.py <manifest.json> <output_dir>
    python run_destructive.py <manifest.json> <output_dir> --limit 50 --timeout 1800 --retries 2

Endpoints auto-discovered from data/destructive/endpoints.json unless
passed as positional args.
"""

import argparse
import json
import os
import subprocess
import sys
import time
from concurrent.futures import ThreadPoolExecutor, as_completed
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent.parent))

from audit import emit as audit

VARIANT_TIMEOUT = 1800
MAX_RETRIES = 2


def variant_complete(output_dir: Path, vid: str) -> bool:
    log_dir = output_dir / vid
    if not log_dir.exists():
        return False
    return any(f.suffix == ".eval" for f in log_dir.iterdir())


def check_endpoint_health(pod_id: str) -> bool:
    url = f"https://{pod_id}-8000.proxy.runpod.net/v1/models"
    try:
        r = subprocess.run(
            [
                "curl",
                "-s",
                "-o",
                "/dev/null",
                "-w",
                "%{http_code}",
                "--max-time",
                "5",
                url,
            ],
            capture_output=True,
            text=True,
            timeout=10,
        )
        return r.stdout.strip() == "200"
    except Exception:
        return False


def extract_pod_id(endpoint_url: str) -> str:
    return endpoint_url.split("//")[-1].split("-8000")[0]


def build_inspect_cmd(
    variant: dict,
    base_task: str,
    base_url: str,
    output_dir: Path,
    model: str,
    limit: int | None = None,
) -> list[str]:
    vid = variant["variant_id"]

    cmd = [
        "inspect",
        "eval",
        "scripts/destructive/destructive_task.py@destructive",
        "--model",
        f"openai/{model}",
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
    model: str,
    index: int,
    total: int,
    limit: int | None = None,
    timeout: int = VARIANT_TIMEOUT,
    retries: int = MAX_RETRIES,
) -> tuple[str, bool, str]:
    vid = variant["variant_id"]

    if variant_complete(output_dir, vid):
        print(f"[{index}/{total}] {vid} -> SKIP (done)", file=sys.stderr)
        return vid, True, ""

    env = os.environ.copy()
    env["OPENAI_BASE_URL"] = base_url
    env["OPENAI_API_KEY"] = "EMPTY"

    variant_detail = {
        "variant_id": vid,
        "block": variant.get("block"),
        "task": base_task,
        "model": model,
    }
    audit(
        "eval.started",
        resource_kind="eval",
        resource_id=vid,
        detail=variant_detail,
    )

    err = ""
    for attempt in range(1, retries + 1):
        cmd = build_inspect_cmd(
            variant,
            base_task,
            base_url,
            output_dir,
            model,
            limit=limit,
        )
        try:
            result = subprocess.run(
                cmd,
                capture_output=True,
                text=True,
                env=env,
                timeout=timeout,
            )
            if result.returncode == 0:
                print(f"[{index}/{total}] {vid} -> OK", file=sys.stderr)
                audit(
                    "eval.completed",
                    resource_kind="eval",
                    resource_id=vid,
                    detail=variant_detail,
                )
                return vid, True, ""
            err = result.stderr[:300]
            print(
                f"[{index}/{total}] {vid} -> FAILED (attempt {attempt}/{retries}): {err}",
                file=sys.stderr,
            )
        except subprocess.TimeoutExpired:
            err = f"timeout after {timeout}s"
            print(
                f"[{index}/{total}] {vid} -> TIMEOUT after {timeout}s "
                f"(attempt {attempt}/{retries})",
                file=sys.stderr,
            )

        if attempt < retries:
            time.sleep(5)

    audit(
        "eval.failed",
        resource_kind="eval",
        resource_id=vid,
        outcome="Failed",
        detail={**variant_detail, "error": err[:200]},
    )
    return vid, False, err


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("manifest", help="Path to manifest JSON")
    parser.add_argument("output_dir", help="Output directory for eval logs")
    parser.add_argument("endpoints", nargs="*", help="vLLM endpoint URLs")
    parser.add_argument("--limit", type=int, default=None)
    parser.add_argument(
        "--timeout",
        type=int,
        default=VARIANT_TIMEOUT,
        help=f"Per-variant timeout in seconds (default: {VARIANT_TIMEOUT})",
    )
    parser.add_argument(
        "--retries",
        type=int,
        default=MAX_RETRIES,
        help=f"Max retries per variant (default: {MAX_RETRIES})",
    )
    parser.add_argument(
        "--endpoints-file",
        type=Path,
        default=Path("data/destructive/endpoints.json"),
    )
    args = parser.parse_args()

    manifest_path = Path(args.manifest)
    output_dir = Path(args.output_dir)

    endpoints: list[str] = args.endpoints or json.loads(
        args.endpoints_file.read_text(),
    )
    if not endpoints:
        print(
            "No endpoints provided and none in endpoints.json",
            file=sys.stderr,
        )
        sys.exit(1)

    manifest = json.loads(manifest_path.read_text())
    base_task: str = manifest["task"]
    model: str = manifest["model"]
    output_dir.mkdir(parents=True, exist_ok=True)

    variants: list[dict] = manifest["runs"]
    total = len(variants)

    already = sum(1 for v in variants if variant_complete(output_dir, v["variant_id"]))
    remaining = total - already

    print(
        f"task: {base_task} | model: {model} | "
        f"{already} done, {remaining} remaining, "
        f"{len(endpoints)} endpoint(s) | "
        f"timeout={args.timeout}s, retries={args.retries}",
        file=sys.stderr,
    )

    healthy: list[str] = []
    for ep in endpoints:
        pid = extract_pod_id(ep)
        if check_endpoint_health(pid):
            healthy.append(ep)
            print(f"  {pid}: healthy", file=sys.stderr)
        else:
            print(f"  {pid}: UNHEALTHY — skipping", file=sys.stderr)

    if not healthy:
        print("No healthy endpoints. Aborting.", file=sys.stderr)
        sys.exit(1)

    if len(healthy) < len(endpoints):
        dropped = len(endpoints) - len(healthy)
        print(
            f"Warning: {dropped} unhealthy endpoints dropped",
            file=sys.stderr,
        )

    n_workers = len(healthy)
    batches: list[list[tuple[int, dict, str]]] = [[] for _ in range(n_workers)]
    pending_idx = 0
    for i, variant in enumerate(variants):
        if variant_complete(output_dir, variant["variant_id"]):
            continue
        endpoint = healthy[pending_idx % n_workers]
        batches[pending_idx % n_workers].append((i + 1, variant, endpoint))
        pending_idx += 1

    failed: list[str] = []

    def run_batch(
        batch: list[tuple[int, dict, str]],
    ) -> list[tuple[str, bool, str]]:
        results = []
        for idx, variant, endpoint in batch:
            results.append(
                run_variant(
                    variant,
                    base_task,
                    endpoint,
                    output_dir,
                    model,
                    idx,
                    total,
                    limit=args.limit,
                    timeout=args.timeout,
                    retries=args.retries,
                ),
            )
        return results

    with ThreadPoolExecutor(max_workers=n_workers) as pool:
        futures = [pool.submit(run_batch, batch) for batch in batches if batch]
        for future in as_completed(futures):
            for vid, ok, _err in future.result():
                if not ok:
                    failed.append(vid)

    if failed:
        print(
            f"\n{base_task}: {len(failed)}/{total} variants failed: {failed}",
            file=sys.stderr,
        )
        sys.exit(1)
    else:
        print(
            f"\n{base_task}: all {total} variants completed.",
            file=sys.stderr,
        )


if __name__ == "__main__":
    main()
