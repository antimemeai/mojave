"""Repo-root utilities for locating Cargo binaries, audit sealing, and hashing."""

from __future__ import annotations

import hashlib
import json
import subprocess
import time
from pathlib import Path
from typing import Any


def repo_root() -> Path:
    """Walk up from this file to find the directory containing Cargo.toml."""
    p = Path(__file__).resolve().parent
    while p != p.parent:
        if (p / "Cargo.toml").exists():
            return p
        p = p.parent
    raise FileNotFoundError("Could not find repo root (no Cargo.toml found)")


def cargo_bin(name: str) -> str:
    """Return the absolute path to a Cargo release binary, or fall back to PATH."""
    candidate = repo_root() / "target" / "release" / name
    if candidate.exists():
        return str(candidate)
    return name


def compute_file_sha256(path: Path) -> str:
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(8192), b""):
            h.update(chunk)
    return h.hexdigest()


def audit_seal(
    run_id: str,
    eval_name: str,
    data_file: Path,
    model_name: str,
    model_provider: str,
    model_hash: str,
    model_hash_method: str = "StructuredDescriptor",
    actor: str = "generate_run_cards_v2.py",
) -> dict[str, Any] | None:
    if model_hash == "00" * 32 or len(model_hash) != 64:
        raise ValueError(
            f"model_hash must be a 64-char hex string (not zero-hash), got: {model_hash!r}"
        )
    data_sha256 = compute_file_sha256(data_file)
    seal_input = {
        "run_id": run_id,
        "eval_name": eval_name,
        "date_issued": time.strftime("%Y-%m-%d"),
        "data_file": str(data_file),
        "data_sha256": data_sha256,
        "actor": {"kind": "System", "id": actor},
        "model": {
            "name": model_name,
            "provider": model_provider,
            "hash_method": model_hash_method,
            "hash": model_hash,
        },
    }
    try:
        result = subprocess.run(
            [cargo_bin("mojave"), "audit", "seal"],
            input=json.dumps(seal_input),
            capture_output=True,
            text=True,
            timeout=30,
        )
        if result.returncode != 0:
            return None
        return json.loads(result.stdout)  # type: ignore[no-any-return]
    except (FileNotFoundError, subprocess.TimeoutExpired):
        return None
