#!/usr/bin/env python3
"""Verify audit chain *structural* integrity and run card provenance.

IMPORTANT: This is a structural-only verifier. It checks JSON linkage
(parent_hash matches previous entry_hash, seq continuity, genesis presence)
but does NOT recompute SHA-256 entry hashes. For cryptographic hash
verification, use: mojave audit verify --chain <path>

Checks:
  1. Chain structure — genesis at index 0, parent_hash linkage, seq continuity
  2. Data hash verification — sha256 in audit events matches files on disk
  3. Lifecycle completeness — every eval.completed has a run_card.generated
  4. Run card ↔ chain tip — chain tips printed on cards exist in the chain

Usage:
    python verify_cards.py [--chain data/audit/chain.jsonl]
        [--cards-dir data/run-cards-v2] [--data-dir data/v2]
"""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import sys
from pathlib import Path


def sha256_file(path: Path) -> str:
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(8192), b""):
            h.update(chunk)
    return h.hexdigest()


def load_chain(path: Path) -> list[dict]:
    entries = []
    with open(path) as f:
        for line in f:
            line = line.strip()
            if line:
                entries.append(json.loads(line))
    return entries


def check_chain_integrity(entries: list[dict]) -> list[str]:
    """Verify hash chain linkage (DAG parent_hash -> entry_hash)."""
    issues: list[str] = []
    if not entries:
        issues.append("FAIL: empty chain (missing genesis)")
        return issues

    first = entries[0]
    if first.get("type") != "Genesis":
        issues.append(f"FAIL: first entry is {first.get('type', 'unknown')}, expected Genesis")

    seen_hashes: dict[str, int] = {}
    for i, entry in enumerate(entries):
        entry_type = entry.get("type", "unknown")
        base = entry.get("base", {})
        seq = base.get("seq", "?")

        if entry_type == "Genesis":
            if i != 0:
                issues.append(f"FAIL: duplicate Genesis at index {i}")
        elif entry_type == "Chained":
            parent_hash = entry.get("parent_hash")
            if parent_hash is not None:
                parent_key = str(parent_hash)
                if i > 0:
                    prev_hash = str(entries[i - 1].get("entry_hash"))
                    if parent_key != prev_hash:
                        issues.append(
                            f"FAIL: entry {i} (seq {seq}) parent_hash does not match "
                            f"previous entry_hash"
                        )
        else:
            issues.append(f"FAIL: unknown entry type {entry_type!r} at index {i}")

        entry_hash = str(entry.get("entry_hash"))
        if entry_hash in seen_hashes:
            issues.append(
                f"FAIL: duplicate entry_hash at index {i} (seq {seq}), "
                f"first seen at index {seen_hashes[entry_hash]}"
            )
        seen_hashes[entry_hash] = i

    for i in range(1, len(entries)):
        prev_seq = entries[i - 1].get("base", {}).get("seq", 0)
        curr_seq = entries[i].get("base", {}).get("seq", 0)
        if curr_seq != prev_seq + 1:
            issues.append(
                f"FAIL: seq discontinuity at index {i}: expected {prev_seq + 1}, got {curr_seq}"
            )

    if not issues:
        max_seq = max(e.get("base", {}).get("seq", 0) for e in entries)
        issues.append(f"OK: chain integrity verified ({len(entries)} entries, max seq {max_seq})")

    return issues


def check_data_hashes(entries: list[dict], data_dir: Path) -> list[str]:
    """Verify sha256 in audit events matches files on disk."""
    findings: list[str] = []
    file_hashes: dict[str, str] = {}

    for entry in entries:
        detail = entry["base"].get("detail", {})
        data_file = detail.get("data_file")
        data_sha256 = detail.get("data_sha256")
        if not data_file or not data_sha256:
            continue

        if data_file in file_hashes:
            if file_hashes[data_file] != data_sha256:
                findings.append(f"FAIL: inconsistent sha256 for {data_file} across audit events")
            continue

        file_hashes[data_file] = data_sha256

    for data_file, expected_hash in file_hashes.items():
        p = Path(data_file)
        if not p.exists():
            findings.append(f"WARN: data file not found: {data_file}")
            continue
        actual = sha256_file(p)
        if actual != expected_hash:
            findings.append(
                f"FAIL: hash mismatch for {data_file}: "
                f"audit={expected_hash[:16]}... "
                f"disk={actual[:16]}..."
            )

    return findings


def check_lifecycle(entries: list[dict]) -> list[str]:
    """Check eval.started → eval.completed → run_card.generated chain."""
    findings: list[str] = []

    started: dict[str, dict[str, int]] = {}
    completed: dict[str, dict[str, int]] = {}
    card_cells: dict[str, set[str]] = {}

    for entry in entries:
        b = entry["base"]
        ev = b["event"]
        detail = b.get("detail", {})

        if ev == "eval.started":
            cid = b["resource"]["id"]
            task = detail.get("task", "?")
            started.setdefault(task, {})[cid] = started.get(task, {}).get(cid, 0) + 1

        elif ev == "eval.completed":
            cid = b["resource"]["id"]
            task = detail.get("task", "?")
            completed.setdefault(task, {})[cid] = completed.get(task, {}).get(cid, 0) + 1

        elif ev == "run_card.generated":
            rid = detail.get("run_id", "")
            parts = rid.split("-")
            if len(parts) >= 4 and parts[-1].startswith("C") and len(parts[-1]) == 6:
                cid = parts[-1].lower()
                eval_parts = parts[2:-1]
                eval_name = "_".join(eval_parts).lower()
                card_cells.setdefault(eval_name, set()).add(cid)

    for task in sorted(set(list(started.keys()) + list(completed.keys()))):
        s = started.get(task, {})
        c = completed.get(task, {})
        cards = card_cells.get(task, set())

        started_only = set(s.keys()) - set(c.keys())
        completed_only = set(c.keys()) - cards

        n_retries = sum(v for v in s.values() if v > 1)

        findings.append(
            f"INFO: {task}: started={len(s)} completed={len(c)} "
            f"cards={len(cards)} retries={n_retries} "
            f"started_no_complete={len(started_only)} "
            f"complete_no_card={len(completed_only)}"
        )

        if completed_only:
            findings.append(
                f"WARN: {task}: {len(completed_only)} completed cells "
                f"without cards: {sorted(completed_only)[:5]}..."
            )

    return findings


def check_card_chain_tips(entries: list[dict], cards_dir: Path) -> list[str]:
    """Verify chain tips printed on run cards exist in the chain."""
    findings: list[str] = []

    entry_hashes: set[str] = set()
    for entry in entries:
        eh = bytes(entry["entry_hash"]).hex()
        entry_hashes.add(eh)

    tip_pattern = re.compile(
        r"\\rcset\{audit\.chain\.tip\}\s*\{\\texttt\{([0-9a-f]+)"
        r"\\allowbreak\s+([0-9a-f]+)\}\}"
    )

    configs = list(cards_dir.rglob("runcard-config.tex"))
    checked = 0
    missing = 0

    for config in configs:
        text = config.read_text()
        m = tip_pattern.search(text)
        if not m:
            continue
        tip_hash = m.group(1) + m.group(2)
        checked += 1
        if tip_hash not in entry_hashes:
            rel = config.relative_to(cards_dir)
            findings.append(f"FAIL: chain tip {tip_hash[:16]}... on card {rel} not found in chain")
            missing += 1

    findings.append(f"INFO: checked {checked} card chain tips, {missing} missing from chain")

    return findings


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Verify audit chain integrity and run card provenance"
    )
    parser.add_argument(
        "--chain",
        type=Path,
        default=Path("data/audit/chain.jsonl"),
    )
    parser.add_argument(
        "--cards-dir",
        type=Path,
        default=Path("data/run-cards-v2"),
    )
    parser.add_argument(
        "--data-dir",
        type=Path,
        default=Path("data/v2"),
    )
    parser.add_argument(
        "--skip-cards",
        action="store_true",
        help="Skip card chain tip verification (slow on 12k+ cards)",
    )
    args = parser.parse_args()

    if not args.chain.exists():
        print(f"FAIL: chain file not found: {args.chain}", file=sys.stderr)
        sys.exit(1)

    print(f"Loading chain from {args.chain}...")
    entries = load_chain(args.chain)
    print(f"  {len(entries)} entries loaded")
    print()

    all_findings: list[str] = []
    fail_count = 0

    print("--- Chain Integrity ---")
    findings = check_chain_integrity(entries)
    all_findings.extend(findings)
    for f in findings:
        print(f"  {f}")
    if not findings:
        print("  OK: chain integrity verified")
    print()

    print("--- Data Hash Verification ---")
    findings = check_data_hashes(entries, args.data_dir)
    all_findings.extend(findings)
    for f in findings:
        print(f"  {f}")
    if not any(f.startswith("FAIL") for f in findings):
        print("  OK: all data hashes match")
    print()

    print("--- Lifecycle Completeness ---")
    findings = check_lifecycle(entries)
    all_findings.extend(findings)
    for f in findings:
        print(f"  {f}")
    print()

    if not args.skip_cards and args.cards_dir.exists():
        print("--- Card Chain Tips ---")
        findings = check_card_chain_tips(entries, args.cards_dir)
        all_findings.extend(findings)
        for f in findings:
            print(f"  {f}")
        print()

    fail_count = sum(1 for f in all_findings if f.startswith("FAIL"))
    warn_count = sum(1 for f in all_findings if f.startswith("WARN"))

    print("=" * 60)
    if fail_count:
        print(f"RESULT: {fail_count} FAILURES, {warn_count} warnings")
        sys.exit(1)
    elif warn_count:
        print(f"RESULT: PASS with {warn_count} warnings")
    else:
        print("RESULT: PASS — all checks clean")


if __name__ == "__main__":
    main()
