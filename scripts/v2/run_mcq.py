#!/usr/bin/env python3
"""Run v2 MCQ perturbation cells across vLLM endpoints.

Work-stealing design: all pending cells go into a shared queue per
quantization level.  One worker thread per healthy endpoint pulls cells
from the queue as it becomes free.  Failed cells are re-enqueued (up to
max retries) so ANY healthy endpoint can pick them up — no manual
requeue, no stranded work on dead pods.

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
import queue
import subprocess
import sys
import threading
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent.parent))

from audit_emit import emit as audit

CELL_TIMEOUT = 1800
MAX_RETRIES = 3
HEALTH_CHECK_INTERVAL = 30
HEALTH_BACKOFF_MAX = 120

DECODING_MAP: dict[str, dict[str, float]] = {
    "greedy": {"temperature": 0.0},
    "T=0.7": {"temperature": 0.7},
    "T=1.0": {"temperature": 1.0},
}


class RunStats:
    """Thread-safe counters for progress reporting."""

    def __init__(self, total: int, already_done: int) -> None:
        self._lock = threading.Lock()
        self.total = total
        self.completed = already_done
        self.failed_final = 0
        self.requeued = 0

    def complete_one(self) -> int:
        with self._lock:
            self.completed += 1
            return self.completed

    def fail_one(self) -> None:
        with self._lock:
            self.failed_final += 1

    def requeue_one(self) -> None:
        with self._lock:
            self.requeued += 1

    def snapshot(self) -> tuple[int, int, int, int]:
        with self._lock:
            return self.total, self.completed, self.failed_final, self.requeued


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


def run_cell_once(
    cell: dict[str, object],
    base_task: str,
    base_url: str,
    output_dir: Path,
    model: str,
    limit: int | None = None,
    timeout: int = CELL_TIMEOUT,
    subset_file: str | None = None,
) -> tuple[bool, str]:
    """Execute a single cell on a single endpoint. Returns (success, error)."""
    cell_id = str(cell["cell_id"])

    if cell_complete(output_dir, cell_id):
        return True, ""

    env = os.environ.copy()
    env["OPENAI_BASE_URL"] = base_url
    env["OPENAI_API_KEY"] = "EMPTY"

    cmd = build_inspect_cmd(cell, base_task, base_url, output_dir, model, limit, subset_file)
    try:
        result = subprocess.run(
            cmd,
            capture_output=True,
            text=True,
            env=env,
            timeout=timeout,
        )
        if result.returncode == 0:
            return True, ""
        return False, result.stderr[:300]
    except subprocess.TimeoutExpired:
        return False, f"timeout after {timeout}s"


def worker(
    endpoint: str,
    work_queue: queue.Queue[tuple[dict[str, object], int]],
    base_task: str,
    output_dir: Path,
    model: str,
    stats: RunStats,
    limit: int | None,
    timeout: int,
    max_retries: int,
    subset_file: str | None,
) -> list[str]:
    """Pull cells from shared queue until empty. Returns list of finally-failed cell IDs."""
    failures: list[str] = []
    ep_short = endpoint.split("//")[-1].split(":")[0][:20]
    backoff = 0.0

    while True:
        try:
            cell, attempt = work_queue.get_nowait()
        except queue.Empty:
            break

        cell_id = str(cell["cell_id"])
        tot, _done, _fail, _req = stats.snapshot()

        if cell_complete(output_dir, cell_id):
            n = stats.complete_one()
            print(
                f"[{n}/{tot}] {cell_id} -> SKIP (done) [{ep_short}]",
                file=sys.stderr,
            )
            work_queue.task_done()
            continue

        if backoff > 0:
            if not check_endpoint_health(endpoint):
                backoff = min(backoff * 2, HEALTH_BACKOFF_MAX)
                print(
                    f"  [{ep_short}] unhealthy, re-enqueuing {cell_id}, backoff {backoff:.0f}s",
                    file=sys.stderr,
                )
                work_queue.put((cell, attempt))
                stats.requeue_one()
                work_queue.task_done()
                time.sleep(backoff)
                continue
            backoff = 0.0

        detail = {
            "cell_id": cell_id,
            "saltelli_index": str(cell["saltelli_index"]),
            "task": base_task,
            "model": model,
            "quantization": cell.get("quantization", "bf16"),
            "n_shot_frac": str(cell.get("n_shot_frac", 0.0)),
        }
        if attempt == 1:
            audit(
                "eval.started",
                resource_kind="eval",
                resource_id=cell_id,
                detail=detail,
            )

        ok, err = run_cell_once(
            cell,
            base_task,
            endpoint,
            output_dir,
            model,
            limit=limit,
            timeout=timeout,
            subset_file=subset_file,
        )

        if ok:
            n = stats.complete_one()
            print(
                f"[{n}/{tot}] {cell_id} -> OK [{ep_short}]",
                file=sys.stderr,
            )
            audit(
                "eval.completed",
                resource_kind="eval",
                resource_id=cell_id,
                detail=detail,
            )
            backoff = 0.0
        elif attempt < max_retries:
            print(
                f"  {cell_id} -> FAIL attempt {attempt}/{max_retries} [{ep_short}]: {err[:80]}",
                file=sys.stderr,
            )
            work_queue.put((cell, attempt + 1))
            stats.requeue_one()
            backoff = HEALTH_CHECK_INTERVAL
        else:
            print(
                f"  {cell_id} -> EXHAUSTED {max_retries} attempts [{ep_short}]",
                file=sys.stderr,
            )
            audit(
                "eval.failed",
                resource_kind="eval",
                resource_id=cell_id,
                outcome="Failed",
                detail={**detail, "error": err[:200]},
            )
            stats.fail_one()
            failures.append(cell_id)

        work_queue.task_done()

    return failures


def main() -> None:
    parser = argparse.ArgumentParser(description="Work-stealing MCQ runner for vLLM fleet")
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

    quant_groups: dict[str, list[dict[str, object]]] = {}
    for cell in cells:
        if cell_complete(output_dir, str(cell["cell_id"])):
            continue
        q = str(cell.get("quantization", "bf16"))
        quant_groups.setdefault(q, []).append(cell)

    all_failed: list[str] = []

    for quant, quant_cells in quant_groups.items():
        pool = get_endpoints_for_cell(endpoints, quant)
        healthy = [ep for ep in pool if check_endpoint_health(ep)]
        if not healthy:
            print(
                f"No healthy endpoints for quantization={quant}. "
                f"Skipping {len(quant_cells)} cells.",
                file=sys.stderr,
            )
            all_failed.extend(str(c["cell_id"]) for c in quant_cells)
            continue

        print(
            f"quantization={quant}: {len(quant_cells)} cells -> "
            f"{len(healthy)} healthy endpoints (work-stealing)",
            file=sys.stderr,
        )

        work_q: queue.Queue[tuple[dict[str, object], int]] = queue.Queue()
        for cell in quant_cells:
            work_q.put((cell, 1))

        stats = RunStats(total, already)

        threads: list[threading.Thread] = []
        thread_results: dict[str, list[str]] = {}

        def _run_worker(
            ep: str,
            wq: queue.Queue[tuple[dict[str, object], int]] = work_q,
            st: RunStats = stats,
            tr: dict[str, list[str]] = thread_results,
        ) -> None:
            tr[ep] = worker(
                endpoint=ep,
                work_queue=wq,
                base_task=base_task,
                output_dir=output_dir,
                model=model,
                stats=st,
                limit=args.limit,
                timeout=args.timeout,
                max_retries=args.retries,
                subset_file=subset_file,
            )

        for ep in healthy:
            t = threading.Thread(target=_run_worker, args=(ep,), daemon=True)
            threads.append(t)
            t.start()

        for t in threads:
            t.join()

        for _ep, fails in thread_results.items():
            all_failed.extend(fails)

        _, done_now, fail_now, req_now = stats.snapshot()
        print(
            f"quantization={quant} done: completed={done_now} failed={fail_now} requeues={req_now}",
            file=sys.stderr,
        )

    if all_failed:
        print(
            f"\n{base_task}: {len(all_failed)}/{total} cells failed",
            file=sys.stderr,
        )
        sys.exit(1)
    else:
        print(f"\n{base_task}: all {total} cells completed.", file=sys.stderr)


if __name__ == "__main__":
    main()
