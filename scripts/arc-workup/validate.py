#!/usr/bin/env python3
"""Post-ingest validation: verify all 1172 items x 217 runs are present.

Reads the mojave ingest JSON output and cross-references against the manifest.
"""

from __future__ import annotations

import json
import sys
from collections import Counter
from pathlib import Path

EXPECTED_ITEMS = 1172


def main() -> None:
    manifest_path = (
        Path(sys.argv[1]) if len(sys.argv) > 1 else Path("scripts/arc-workup/manifest.json")
    )
    records_path = (
        Path(sys.argv[2]) if len(sys.argv) > 2 else Path("data/arc-workup/trial_records.json")
    )

    manifest = json.loads(manifest_path.read_text())
    records = json.loads(records_path.read_text())

    expected_runs = manifest["total_variants"]
    run_ids = set()
    items_per_run: Counter[str] = Counter()

    for rec in records:
        rid = rec["run_id"]
        run_ids.add(rid)
        items_per_run[rid] += 1

    errors: list[str] = []

    if len(run_ids) != expected_runs:
        errors.append(f"Expected {expected_runs} runs, found {len(run_ids)}")

    for rid, count in items_per_run.items():
        if count != EXPECTED_ITEMS:
            errors.append(f"Run {rid}: expected {EXPECTED_ITEMS} items, found {count}")

    if errors:
        print("VALIDATION FAILED:", file=sys.stderr)
        for e in errors:
            print(f"  - {e}", file=sys.stderr)
        sys.exit(1)
    else:
        print(
            f"VALIDATION PASSED: {len(run_ids)} runs x {EXPECTED_ITEMS} items = "
            f"{len(records)} total records",
            file=sys.stderr,
        )


if __name__ == "__main__":
    main()
