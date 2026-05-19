#!/usr/bin/env python3
"""Convert mojave TrialRecords JSON to py-irt JSONL format.

Each variant (run_id) becomes a subject. Each unique task item becomes an item.
Binary outcomes map to 0/1 responses.
"""

from __future__ import annotations

import json
import sys
from collections import defaultdict
from pathlib import Path


def main() -> None:
    records_path = (
        Path(sys.argv[1]) if len(sys.argv) > 1 else Path("data/arc-workup/trial_records.json")
    )
    output_path = (
        Path(sys.argv[2]) if len(sys.argv) > 2 else Path("data/arc-workup/irt_input.jsonl")
    )

    records = json.loads(records_path.read_text())

    # Group by run_id (each run = one subject)
    runs: dict[str, dict[str, int]] = defaultdict(dict)

    for rec in records:
        run_id = rec["run_id"]
        sample_id = rec.get("metadata", {}).get("sample_id", rec["trial_id"])
        outcome = rec["outcome"]

        if isinstance(outcome, dict) and "Binary" in outcome:
            response = 1 if outcome["Binary"] else 0
        elif isinstance(outcome, bool):
            response = 1 if outcome else 0
        else:
            continue

        runs[run_id][sample_id] = response

    output_path.parent.mkdir(parents=True, exist_ok=True)
    with output_path.open("w") as f:
        for run_id, responses in sorted(runs.items()):
            line = json.dumps({"subject_id": run_id, "responses": responses})
            f.write(line + "\n")

    print(
        f"Wrote {len(runs)} subjects x {len(next(iter(runs.values())))} items -> {output_path}",
        file=sys.stderr,
    )


if __name__ == "__main__":
    main()
