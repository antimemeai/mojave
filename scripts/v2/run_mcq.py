#!/usr/bin/env python3
"""Run v2 MCQ perturbation cells across vLLM endpoints.

Supports quantization-aware endpoint routing: if the endpoints file is a
dict keyed by quantization level, cells are routed to the matching pool.
If it's a flat list, all cells go to all endpoints (for testing).

Usage:
    python run_mcq.py <manifest.json> <output_dir> --endpoints-file <endpoints.json>
    python run_mcq.py <manifest.json> <output_dir> --endpoints http://host:8000/v1
"""

from __future__ import annotations

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

CELL_TIMEOUT = 1800
MAX_RETRIES = 2

DECODING_MAP: dict[str, dict[str, float]] = {
    "greedy": {"temperature": 0.0},
    "T=0.7": {"temperature": 0.7},
    "T=1.0": {"temperature": 1.0},
}


def cell_complete(output_dir: Path, cell_id: str) -> bool:
    log_dir = output_dir / cell_id
    if not log_dir.exists():
        return False
    return any(f.suffix == ".eval" for f in log_dir.iterdir())


def check_endpoint_health(url: str) -> bool:
    models_url = url.rstrip("/") + "/models"
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
                models_url,
            ],
            capture_output=True,
            text=True,
            timeout=10,
        )
        return r.stdout.strip() == "200"
    except Exception:
        return False


def load_endpoints(
    cli_endpoints: list[str] | None,
    endpoints_file: Path,
) -> dict[str, list[str]] | list[str]:
    """Load endpoints. Returns dict (keyed by quant) or list (flat)."""
    if cli_endpoints:
        return cli_endpoints
    raw = json.loads(endpoints_file.read_text())
    return raw  # type: ignore[no-any-return]


def get_endpoints_for_cell(
    endpoints: dict[str, list[str]] | list[str],
    quantization: str,
) -> list[str]:
    """Get the endpoint pool for a cell's quantization level."""
    if isinstance(endpoints, dict):
        pool = endpoints.get(quantization, [])
        if not pool:
            return []
        return pool
    return endpoints


def build_inspect_cmd(
    cell: dict[str, object],
    base_task: str,
    base_url: str,
    output_dir: Path,
    model: str,
    limit: int | None,
    subset_file: str | None = None,
) -> list[str]:
    cell_id = str(cell["cell_id"])
    decoding = DECODING_MAP[str(cell["decoding"])]
    shuffle = "true" if cell["choice_order"] == "shuffled" else "false"

    cmd = [
        "inspect",
        "eval",
        "scripts/v2/mcq_task.py@mcq_v2",
        "--model",
        f"openai/{model}",
        "--model-base-url",
        base_url,
    ]

    if subset_file:
        cmd.extend(["-T", f"subset_file={subset_file}"])
    else:
        cmd.extend(["-T", f"base_task={base_task}"])

    cmd.extend(
        [
            "-T",
            f"prompt_template={cell['prompt_template']}",
            "-T",
            f"system_prompt={cell['system_prompt']}",
            "-T",
            f"n_shot_frac={cell['n_shot_frac']}",
            "-T",
            f"shuffle={shuffle}",
            "--temperature",
            str(decoding["temperature"]),
            "--log-dir",
            str(output_dir / cell_id),
        ]
    )

    if limit is not None:
        cmd.extend(["--limit", str(limit)])

    return cmd


def run_cell(
    cell: dict[str, object],
    base_task: str,
    base_url: str,
    output_dir: Path,
    model: str,
    index: int,
    total: int,
    limit: int | None = None,
    timeout: int = CELL_TIMEOUT,
    retries: int = MAX_RETRIES,
    subset_file: str | None = None,
) -> tuple[str, bool, str]:
    cell_id = str(cell["cell_id"])

    if cell_complete(output_dir, cell_id):
        print(f"[{index}/{total}] {cell_id} -> SKIP (done)", file=sys.stderr)
        return cell_id, True, ""

    env = os.environ.copy()
    env["OPENAI_BASE_URL"] = base_url
    env["OPENAI_API_KEY"] = "EMPTY"

    detail = {
        "cell_id": cell_id,
        "saltelli_index": str(cell["saltelli_index"]),
        "task": base_task,
        "model": model,
        "quantization": cell.get("quantization", "bf16"),
        "n_shot_frac": str(cell.get("n_shot_frac", 0.0)),
    }
    audit("eval.started", resource_kind="eval", resource_id=cell_id, detail=detail)

    err = ""
    for attempt in range(1, retries + 1):
        cmd = build_inspect_cmd(
            cell,
            base_task,
            base_url,
            output_dir,
            model,
            limit,
            subset_file=subset_file,
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
                print(f"[{index}/{total}] {cell_id} -> OK", file=sys.stderr)
                audit(
                    "eval.completed",
                    resource_kind="eval",
                    resource_id=cell_id,
                    detail=detail,
                )
                return cell_id, True, ""
            err = result.stderr[:300]
            print(
                f"[{index}/{total}] {cell_id} -> FAILED (attempt {attempt}/{retries}): {err}",
                file=sys.stderr,
            )
        except subprocess.TimeoutExpired:
            err = f"timeout after {timeout}s"
            print(
                f"[{index}/{total}] {cell_id} -> TIMEOUT (attempt {attempt}/{retries})",
                file=sys.stderr,
            )

        if attempt < retries:
            time.sleep(5)

    audit(
        "eval.failed",
        resource_kind="eval",
        resource_id=cell_id,
        outcome="Failed",
        detail={**detail, "error": err[:200]},
    )
    return cell_id, False, err


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("manifest", help="Path to Saltelli manifest JSON")
    parser.add_argument("output_dir", help="Output directory for eval logs")
    parser.add_argument("endpoints", nargs="*", help="vLLM endpoint URLs")
    parser.add_argument("--timeout", type=int, default=CELL_TIMEOUT)
    parser.add_argument("--retries", type=int, default=MAX_RETRIES)
    parser.add_argument(
        "--limit",
        type=int,
        default=None,
        help="Limit items per cell (for smoketest)",
    )
    parser.add_argument(
        "--endpoints-file",
        type=Path,
        default=Path("data/destructive/endpoints.json"),
    )
    parser.add_argument(
        "--subset-file",
        type=str,
        default=None,
        help="Path to pre-sampled item subset JSON (overrides base_task dataset)",
    )
    args = parser.parse_args()

    manifest = json.loads(Path(args.manifest).read_text())
    output_dir = Path(args.output_dir)
    output_dir.mkdir(parents=True, exist_ok=True)

    base_task = manifest["task"]
    model = manifest["model"]
    subset_file: str | None = str(Path(args.subset_file).resolve()) if args.subset_file else None

    endpoints = load_endpoints(args.endpoints or None, args.endpoints_file)

    cells = manifest["cells"]
    total = len(cells)
    already = sum(1 for c in cells if cell_complete(output_dir, c["cell_id"]))
    remaining = total - already

    print(
        f"task: {base_task} | model: {model} | {already} done, {remaining} remaining",
        file=sys.stderr,
    )

    quant_groups: dict[str, list[tuple[int, dict[str, object]]]] = {}
    for i, cell in enumerate(cells):
        if cell_complete(output_dir, str(cell["cell_id"])):
            continue
        q = str(cell.get("quantization", "bf16"))
        quant_groups.setdefault(q, []).append((i + 1, cell))

    failed: list[str] = []

    for quant, quant_cells in quant_groups.items():
        pool = get_endpoints_for_cell(endpoints, quant)
        healthy = [ep for ep in pool if check_endpoint_health(ep)]
        if not healthy:
            print(
                f"No healthy endpoints for quantization={quant}. "
                f"Skipping {len(quant_cells)} cells.",
                file=sys.stderr,
            )
            failed.extend(str(c["cell_id"]) for _, c in quant_cells)
            continue

        n_workers = len(healthy)
        batches: list[list[tuple[int, dict[str, object], str]]] = [[] for _ in range(n_workers)]
        for idx, (cell_num, cell) in enumerate(quant_cells):
            ep = healthy[idx % n_workers]
            batches[idx % n_workers].append((cell_num, cell, ep))

        def run_batch(
            batch: list[tuple[int, dict[str, object], str]],
        ) -> list[tuple[str, bool, str]]:
            results = []
            for cell_num, cell, endpoint in batch:
                results.append(
                    run_cell(
                        cell,
                        base_task,
                        endpoint,
                        output_dir,
                        model,
                        cell_num,
                        total,
                        limit=args.limit,
                        timeout=args.timeout,
                        retries=args.retries,
                        subset_file=subset_file,
                    )
                )
            return results

        with ThreadPoolExecutor(max_workers=n_workers) as pool_exec:
            futures = [pool_exec.submit(run_batch, batch) for batch in batches if batch]
            for future in as_completed(futures):
                for cid, ok, _err in future.result():
                    if not ok:
                        failed.append(cid)

    if failed:
        print(
            f"\n{base_task}: {len(failed)}/{total} cells failed",
            file=sys.stderr,
        )
        sys.exit(1)
    else:
        print(f"\n{base_task}: all {total} cells completed.", file=sys.stderr)


if __name__ == "__main__":
    main()
