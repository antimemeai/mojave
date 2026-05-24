#!/usr/bin/env python3
"""Fast post-hoc re-scorer for MCQ eval cells.

Reads .eval files directly (zstd-compressed zip) without shelling out to
`inspect log dump`. Orders of magnitude faster than rescore.py.

Usage:
    python rescore_fast.py data/v2/logs_bio --manifest data/v2/manifest_bio_512.json
    python rescore_fast.py data/v2/logs_bio --manifest data/v2/manifest_bio_512.json --by-axis
    python rescore_fast.py data/v2/logs_bio \
        --manifest data/v2/manifest_bio_512.json --json results.json
"""

from __future__ import annotations

import argparse
import json
import re
import sys
import time
import zipfile
from collections import defaultdict
from pathlib import Path

import zipfile_zstd  # noqa: F401 — patches zipfile for zstd


def parse_answer_permissive(text: str, n_choices: int = 4) -> str | None:
    text = text.strip()
    valid = set(chr(ord("A") + i) for i in range(n_choices))

    m = re.search(r"(?i)ANSWER\s*:\s*([A-Za-z])", text)
    if m and m.group(1).upper() in valid:
        return m.group(1).upper()

    bare = text.strip().rstrip(".)")
    if len(bare) == 1 and bare.upper() in valid:
        return bare.upper()

    m = re.search(r"(?i)(?:answer|choose|select|pick)\s+(?:is\s+)?([A-Za-z])\b", text)
    if m and m.group(1).upper() in valid:
        return m.group(1).upper()

    for line in reversed(text.split("\n")):
        line = line.strip().rstrip(".)")
        if len(line) == 1 and line.upper() in valid:
            return line.upper()

    return None


def read_eval_direct(eval_path: Path) -> list[dict] | None:
    try:
        with zipfile.ZipFile(str(eval_path)) as zf:
            sample_files = [f for f in zf.namelist() if f.startswith("samples/")]
            results = []
            for sf in sample_files:
                data = json.loads(zf.read(sf))
                target = data.get("target", "")
                choice_score = data.get("scores", {}).get("choice", {})
                inspect_value = choice_score.get("value", "I")

                model_output = ""
                for m in data.get("messages", []):
                    if m.get("role") == "assistant":
                        content = m.get("content", "")
                        if isinstance(content, list):
                            content = " ".join(
                                p.get("text", "") for p in content if isinstance(p, dict)
                            )
                        model_output = content
                        break

                n_choices = len(data.get("choices", ["A", "B", "C", "D"]))
                parsed = parse_answer_permissive(model_output, n_choices)

                results.append(
                    {
                        "id": data.get("id", "?"),
                        "target": target,
                        "inspect_correct": inspect_value == "C",
                        "rescore_correct": parsed is not None and parsed == target,
                        "parsed": parsed,
                        "model_output_head": model_output[:80],
                    }
                )
            return results
    except Exception as e:
        print(f"  error reading {eval_path}: {e}", file=sys.stderr)
        return None


def process_log_dir(log_dir: Path, cell_map: dict) -> list[dict]:
    cells = []
    cell_dirs = sorted(d for d in log_dir.iterdir() if d.is_dir())

    t0 = time.time()
    for i, cd in enumerate(cell_dirs):
        cell_id = cd.name
        cell = cell_map.get(cell_id)
        if not cell:
            continue

        evals = list(cd.glob("*.eval"))
        if not evals:
            continue

        samples = read_eval_direct(evals[0])
        if samples is None:
            continue

        total = len(samples)
        inspect_correct = sum(1 for s in samples if s["inspect_correct"])
        rescore_correct = sum(1 for s in samples if s["rescore_correct"])

        cells.append(
            {
                "cell_id": cell_id,
                "prompt_template": str(cell["prompt_template"]),
                "system_prompt": str(cell["system_prompt"]),
                "n_shot_frac": str(cell["n_shot_frac"]),
                "choice_order": str(cell["choice_order"]),
                "decoding": str(cell["decoding"]),
                "quantization": str(cell.get("quantization", "bf16")),
                "total": total,
                "inspect_correct": inspect_correct,
                "rescore_correct": rescore_correct,
                "inspect_acc": inspect_correct / total if total else 0,
                "rescore_acc": rescore_correct / total if total else 0,
            }
        )

        if (i + 1) % 200 == 0:
            elapsed = time.time() - t0
            rate = (i + 1) / elapsed
            print(f"  {i + 1}/{len(cell_dirs)} cells ({rate:.0f}/s)", file=sys.stderr)

    elapsed = time.time() - t0
    print(f"  Done: {len(cells)} cells in {elapsed:.1f}s", file=sys.stderr)
    return cells


def show_by_field(cells: list[dict], field: str, label: str) -> None:
    groups: dict[str, list[dict]] = defaultdict(list)
    for c in cells:
        groups[c[field]].append(c)

    print(f"\n{label}:")
    print(f"  {'Level':<25s} {'N':>5s} {'Inspect':>9s} {'Rescore':>9s} {'Delta':>7s}")
    print(f"  {'-' * 56}")
    for k in sorted(groups):
        g = groups[k]
        n = len(g)
        insp = sum(c["inspect_acc"] for c in g) / n
        resc = sum(c["rescore_acc"] for c in g) / n
        print(f"  {k:<25s} {n:5d} {insp:9.3f} {resc:9.3f} {resc - insp:+7.3f}")


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("log_dir", type=Path)
    parser.add_argument("--manifest", required=True, type=Path)
    parser.add_argument("--by-axis", action="store_true", help="Break down by all axes")
    parser.add_argument("--json", type=Path, default=None, help="Write per-cell results to JSON")
    args = parser.parse_args()

    manifest = json.loads(args.manifest.read_text())
    cell_map = {str(c["cell_id"]): c for c in manifest["cells"]}

    cells = process_log_dir(args.log_dir, cell_map)

    show_by_field(cells, "prompt_template", "Prompt Template")

    if args.by_axis:
        show_by_field(cells, "system_prompt", "System Prompt")
        show_by_field(cells, "n_shot_frac", "N-Shot Frac")
        show_by_field(cells, "choice_order", "Choice Order")
        show_by_field(cells, "decoding", "Decoding")

    n = len(cells)
    if n:
        insp = sum(c["inspect_acc"] for c in cells) / n
        resc = sum(c["rescore_acc"] for c in cells) / n
        print(f"\n  {'ALL':<25s} {n:5d} {insp:9.3f} {resc:9.3f} {resc - insp:+7.3f}")
        print(
            f"\n  Max-min spread (rescore): {max(c['rescore_acc'] for c in cells):.3f} - "
            f"{min(c['rescore_acc'] for c in cells):.3f} = "
            f"{max(c['rescore_acc'] for c in cells) - min(c['rescore_acc'] for c in cells):.3f}"
        )

    if args.json:
        args.json.write_text(json.dumps(cells, indent=2))
        print(f"\nWrote {len(cells)} cell results to {args.json}", file=sys.stderr)


if __name__ == "__main__":
    main()
