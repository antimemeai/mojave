#!/usr/bin/env python3
"""Read Inspect .eval log files (zstd-compressed zip archives).

Usage:
    python read_eval.py <path.eval>                   # header + accuracy
    python read_eval.py <path.eval> --samples 5       # + first N samples
    python read_eval.py <log_dir> --summary           # summarize all cells
"""

from __future__ import annotations

import argparse
import glob
import json
import subprocess
import sys
from pathlib import Path


def read_header(eval_path: str) -> dict | None:
    result = subprocess.run(
        ["inspect", "log", "dump", eval_path, "--header-only"],
        capture_output=True,
        text=True,
        timeout=30,
    )
    if result.returncode != 0:
        return None
    return json.loads(result.stdout)


def read_full(eval_path: str) -> dict | None:
    result = subprocess.run(
        ["inspect", "log", "dump", eval_path],
        capture_output=True,
        text=True,
        timeout=60,
    )
    if result.returncode != 0:
        return None
    return json.loads(result.stdout)


def print_header(header: dict) -> None:
    eval_info = header.get("eval", {})
    results = header.get("results", {})
    scores = results.get("scores", [{}])
    metrics = scores[0].get("metrics", {}) if scores else {}
    acc = metrics.get("accuracy", {}).get("value", "?")
    task_args = eval_info.get("task_args", {})

    print(f"  Status:    {header.get('status')}")
    print(f"  Model:     {eval_info.get('model')}")
    print(f"  Samples:   {results.get('completed_samples')} / {results.get('total_samples')}")
    print(f"  Accuracy:  {acc}")
    print(f"  Template:  {task_args.get('prompt_template', '?')}")
    print(f"  System:    {task_args.get('system_prompt', '?')}")
    print(f"  N-shot:    {task_args.get('n_shot_frac', '?')}")
    print(f"  Shuffle:   {task_args.get('shuffle', '?')}")


def print_samples(log: dict, n: int) -> None:
    samples = log.get("samples", [])
    for s in samples[:n]:
        sid = s.get("id", "?")
        target = s.get("target", "?")
        score_data = s.get("scores", {})
        choice_score = score_data.get("choice", {})
        value = choice_score.get("value", "?")
        answer = choice_score.get("answer", "?")
        msgs = s.get("messages", [])
        model_output = ""
        for m in msgs:
            if m.get("role") == "assistant":
                model_output = m.get("content", "")[:200]
        print(f"\n  --- {sid} ---")
        print(f"  Target: {target}  Score: {value}  Answer: {answer}")
        print(f"  Output: {model_output}")


def summarize_dir(log_dir: str) -> None:
    all_logs = glob.glob(f"{log_dir}/c*/*.eval")
    if not all_logs:
        print(f"No .eval files found in {log_dir}")
        return

    by_template: dict[str, list[float]] = {}
    errors = 0

    for lpath in sorted(all_logs):
        header = read_header(lpath)
        if not header:
            errors += 1
            continue
        results = header.get("results", {})
        scores = results.get("scores", [{}])
        acc = scores[0].get("metrics", {}).get("accuracy", {}).get("value") if scores else None
        template = header.get("eval", {}).get("task_args", {}).get("prompt_template", "?")

        if acc is not None:
            by_template.setdefault(template, []).append(acc)

    total = sum(len(v) for v in by_template.values())
    print(f"Cells completed: {total}  (read errors: {errors})")
    print(f"\n{'Template':<25s} {'N':>5s} {'Mean':>7s} {'Min':>7s} {'Max':>7s} {'Zeros':>6s}")
    print("-" * 60)
    for tmpl in sorted(by_template):
        accs = by_template[tmpl]
        n = len(accs)
        mean = sum(accs) / n
        zeros = sum(1 for a in accs if a < 0.01)
        print(f"{tmpl:<25s} {n:5d} {mean:7.3f} {min(accs):7.3f} {max(accs):7.3f} {zeros:6d}")

    all_accs = [a for v in by_template.values() for a in v]
    print("-" * 60)
    print(
        f"{'ALL':<25s} {len(all_accs):5d} {sum(all_accs) / len(all_accs):7.3f}"
        f" {min(all_accs):7.3f} {max(all_accs):7.3f}"
        f" {sum(1 for a in all_accs if a < 0.01):6d}"
    )


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("path", help=".eval file or log directory")
    parser.add_argument("--samples", type=int, default=0, help="Show N samples")
    parser.add_argument("--summary", action="store_true", help="Summarize all cells in dir")
    args = parser.parse_args()

    if args.summary:
        summarize_dir(args.path)
        return

    p = Path(args.path)
    if p.is_dir():
        evals = list(p.glob("*.eval"))
        if not evals:
            print(f"No .eval files in {p}", file=sys.stderr)
            sys.exit(1)
        target = str(evals[0])
    else:
        target = str(p)

    print(f"Log: {target}")

    if args.samples > 0:
        log = read_full(target)
        if not log:
            print("Failed to read log", file=sys.stderr)
            sys.exit(1)
        print_header(log)
        print_samples(log, args.samples)
    else:
        header = read_header(target)
        if not header:
            print("Failed to read header", file=sys.stderr)
            sys.exit(1)
        print_header(header)


if __name__ == "__main__":
    main()
