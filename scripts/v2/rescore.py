#!/usr/bin/env python3
"""Post-hoc re-scorer for MCQ eval cells.

Reads .eval logs, applies permissive answer parsing (handles bare letters,
"ANSWER: X", "the answer is X", etc.), and outputs corrected accuracy
alongside native Inspect scores.

Usage:
    python rescore.py data/v2/logs_bio --manifest data/v2/manifest_bio_512.json
    python rescore.py data/v2/logs_bio --manifest data/v2/manifest_bio_512.json --details
"""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
from collections import defaultdict
from pathlib import Path


def parse_answer_permissive(text: str, n_choices: int = 4) -> str | None:
    """Extract an answer letter from model output, permissively."""
    text = text.strip()
    valid = set(chr(ord("A") + i) for i in range(n_choices))

    # 1. ANSWER: X (standard Inspect format)
    m = re.search(r"(?i)ANSWER\s*:\s*([A-Za-z])", text)
    if m and m.group(1).upper() in valid:
        return m.group(1).upper()

    # 2. Bare single letter (possibly with period/paren)
    bare = text.strip().rstrip(".)")
    if len(bare) == 1 and bare.upper() in valid:
        return bare.upper()

    # 3. "The answer is X" / "I would choose X"
    m = re.search(r"(?i)(?:answer|choose|select|pick)\s+(?:is\s+)?([A-Za-z])\b", text)
    if m and m.group(1).upper() in valid:
        return m.group(1).upper()

    # 4. Last single letter on its own line
    for line in reversed(text.split("\n")):
        line = line.strip().rstrip(".)")
        if len(line) == 1 and line.upper() in valid:
            return line.upper()

    # 5. Last letter after "ANSWER:" anywhere
    matches = re.findall(r"(?i)ANSWER\s*:\s*([A-Za-z])", text)
    if matches and matches[-1].upper() in valid:
        return matches[-1].upper()

    return None


def read_full_log(eval_path: str) -> dict | None:
    r = subprocess.run(
        ["inspect", "log", "dump", eval_path],
        capture_output=True,
        text=True,
        timeout=60,
    )
    if r.returncode != 0:
        return None
    return json.loads(r.stdout)


def rescore_cell(eval_path: str) -> dict | None:
    log = read_full_log(eval_path)
    if not log:
        return None

    samples = log.get("samples", [])
    if not samples:
        return None

    inspect_correct = 0
    rescore_correct = 0
    total = len(samples)
    mismatches = []

    for s in samples:
        target = s.get("target", "")
        score_data = s.get("scores", {}).get("choice", {})
        inspect_value = score_data.get("value", "I")
        inspect_ok = inspect_value == "C"

        model_output = ""
        for m in s.get("messages", []):
            if m.get("role") == "assistant":
                model_output = m.get("content", "")
                if isinstance(model_output, list):
                    model_output = " ".join(
                        p.get("text", "") for p in model_output if isinstance(p, dict)
                    )

        n_choices = len(s.get("choices", ["A", "B", "C", "D"]))
        parsed = parse_answer_permissive(model_output, n_choices)
        rescore_ok = parsed is not None and parsed == target

        if inspect_ok:
            inspect_correct += 1
        if rescore_ok:
            rescore_correct += 1
        if rescore_ok != inspect_ok:
            mismatches.append(
                {
                    "id": s.get("id", "?"),
                    "target": target,
                    "model_output": model_output[:100],
                    "parsed": parsed,
                    "inspect": inspect_value,
                }
            )

    return {
        "total": total,
        "inspect_correct": inspect_correct,
        "rescore_correct": rescore_correct,
        "inspect_acc": inspect_correct / total if total else 0,
        "rescore_acc": rescore_correct / total if total else 0,
        "mismatches": mismatches,
    }


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("log_dir", help="Directory with cell subdirectories")
    parser.add_argument("--manifest", required=True, help="Saltelli manifest JSON")
    parser.add_argument("--details", action="store_true", help="Show per-cell mismatches")
    parser.add_argument("--template", type=str, default=None, help="Filter to one template")
    args = parser.parse_args()

    manifest = json.loads(Path(args.manifest).read_text())
    cell_map = {str(c["cell_id"]): c for c in manifest["cells"]}

    log_dir = Path(args.log_dir)
    cell_dirs = sorted(log_dir.iterdir())

    by_template: dict[str, list[dict]] = defaultdict(list)
    errors = 0
    processed = 0

    for cd in cell_dirs:
        if not cd.is_dir():
            continue
        cell_id = cd.name
        cell = cell_map.get(cell_id)
        if not cell:
            continue

        template = str(cell["prompt_template"])
        if args.template and template != args.template:
            continue

        evals = list(cd.glob("*.eval"))
        if not evals:
            continue

        result = rescore_cell(str(evals[0]))
        if result is None:
            errors += 1
            continue

        result["cell_id"] = cell_id
        result["template"] = template
        result["system_prompt"] = str(cell["system_prompt"])
        result["choice_order"] = str(cell["choice_order"])
        result["n_shot_frac"] = str(cell["n_shot_frac"])
        result["decoding"] = str(cell["decoding"])
        by_template[template].append(result)
        processed += 1

        if processed % 50 == 0:
            print(f"  processed {processed} cells...", file=sys.stderr)

    print(f"\nProcessed: {processed}  Errors: {errors}\n")
    print(f"{'Template':<25s} {'N':>5s} {'Inspect':>9s} {'Rescore':>9s} {'Delta':>7s}")
    print("-" * 60)

    all_inspect: list[float] = []
    all_rescore: list[float] = []
    for tmpl in sorted(by_template):
        cells = by_template[tmpl]
        n = len(cells)
        insp = sum(c["inspect_acc"] for c in cells) / n
        resc = sum(c["rescore_acc"] for c in cells) / n
        delta = resc - insp
        all_inspect.extend(c["inspect_acc"] for c in cells)
        all_rescore.extend(c["rescore_acc"] for c in cells)
        print(f"{tmpl:<25s} {n:5d} {insp:9.3f} {resc:9.3f} {delta:+7.3f}")

    print("-" * 60)
    n = len(all_inspect)
    insp_mean = sum(all_inspect) / n
    resc_mean = sum(all_rescore) / n
    print(f"{'ALL':<25s} {n:5d} {insp_mean:9.3f} {resc_mean:9.3f} {resc_mean - insp_mean:+7.3f}")

    if args.details:
        print("\n=== Mismatches (rescore != inspect) ===")
        for tmpl in sorted(by_template):
            for cell in by_template[tmpl]:
                if cell["mismatches"]:
                    print(f"\n{cell['cell_id']} ({tmpl}):")
                    for mm in cell["mismatches"][:5]:
                        print(
                            f"  {mm['id']}: target={mm['target']} parsed={mm['parsed']} "
                            f"inspect={mm['inspect']} output={mm['model_output'][:60]!r}"
                        )


if __name__ == "__main__":
    main()
