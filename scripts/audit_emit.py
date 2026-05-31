"""Audit event emission via mojave CLI subprocess.

Replaces the former Python audit chain writer (audit.py) with subprocess
calls to `mojave audit emit`. The Rust binary is the single source of
truth for chain format, hash computation, and canonical encoding.

Usage:
    from audit_emit import emit

    emit("eval.started", resource_kind="eval", resource_id="cell-42",
         detail={"trial": 1})

If the mojave binary is not found or the chain does not exist, the call
is a silent no-op. Audit emission must never block eval execution.
"""

from __future__ import annotations

import json
import shutil
import subprocess
import sys
from datetime import UTC, datetime
from pathlib import Path

DEFAULT_AUDIT_DIR = Path("data/audit")

# Event kind mapping: Python dotted strings -> Rust EventKind enum values.
# Events not in this map are silently dropped (the Rust side would reject them).
_VALID_EVENT_KINDS = {
    "eval.started",
    "eval.completed",
    "eval.failed",
    "pod.created",
    "pod.ready",
    "pod.terminated",
    "endpoint.verified",
    "dataset.loaded",
    "dataset.cached",
    "model.loaded",
    "scoring.completed",
    "run_card.generated",
    "run_card.sealed",
    "key.generated",
    "key.loaded",
    "chain.verified",
    "chain.genesis",
    "attestation.created",
    "config.changed",
    "circuit_breaker.tripped",
    "circuit_breaker.reset",
}


def _find_mojave_bin() -> str | None:
    """Locate the mojave binary: release build, debug build, or PATH."""
    # Try release build first
    candidate = Path("target/release/mojave")
    if candidate.exists():
        return str(candidate)
    # Try debug build
    candidate = Path("target/debug/mojave")
    if candidate.exists():
        return str(candidate)
    # Fall back to PATH
    return shutil.which("mojave")


def emit(
    event_kind: str,
    *,
    resource_kind: str = "system",
    resource_id: str = "unknown",
    actor_kind: str = "System",
    actor_id: str = "mojave-python",
    authorization: str = "Allowed",
    outcome: str = "Succeeded",
    tags: dict[str, str] | None = None,
    detail: dict[str, object] | None = None,
    audit_dir: Path = DEFAULT_AUDIT_DIR,
) -> bool:
    """Emit an audit event via mojave CLI subprocess.

    Returns True if the event was emitted successfully, False otherwise.
    Never raises — audit emission must not block eval execution.
    """
    if event_kind not in _VALID_EVENT_KINDS:
        print(
            f"[audit] skipping unknown event kind: {event_kind}",
            file=sys.stderr,
        )
        return False

    mojave_bin = _find_mojave_bin()
    if mojave_bin is None:
        return False

    chain_file = audit_dir / "chain.jsonl"
    if not chain_file.exists():
        return False

    # Build the AuditEvent JSON that mojave audit emit expects.
    # Must match crate audit_events::types::AuditEvent.
    now = datetime.now(UTC).replace(microsecond=0)

    # Map string outcome to Rust enum format
    if outcome == "Succeeded":
        outcome_value: object = "Succeeded"
    elif outcome == "Failed":
        error_msg = ""
        if detail and "error" in detail:
            error_msg = str(detail["error"])
        outcome_value = {"Failed": {"error": error_msg}}
    else:
        outcome_value = "Observed"

    # Clean detail: ensure no floats (canonical encoding rejects them).
    # Convert float values to strings to avoid rejection.
    clean_detail = _sanitize_detail(detail) if detail else None

    event = {
        "envelope_version": 1,
        "at": now.strftime("%Y-%m-%dT%H:%M:%SZ"),
        "actor": {"kind": actor_kind, "id": actor_id},
        "event": event_kind,
        "resource": {"kind": resource_kind, "id": resource_id},
        "authorization": authorization,
        "outcome": outcome_value,
    }

    if tags:
        event["tags"] = dict(sorted(tags.items()))

    if clean_detail is not None:
        event["detail"] = clean_detail

    try:
        result = subprocess.run(
            [mojave_bin, "audit", "emit", "--audit-dir", str(audit_dir)],
            input=json.dumps(event),
            capture_output=True,
            text=True,
            timeout=10,
        )
        return result.returncode == 0
    except (subprocess.TimeoutExpired, OSError):
        return False


def _sanitize_detail(detail: dict[str, object] | None) -> dict[str, object] | None:
    """Convert float values to strings to satisfy canonical encoding."""
    if detail is None:
        return None
    return {k: _sanitize_value(v) for k, v in detail.items()}


def _sanitize_value(value: object) -> object:
    """Recursively convert floats to strings."""
    if isinstance(value, float):
        # Convert to int if it's a whole number, otherwise to string
        if value == int(value):
            return int(value)
        return str(value)
    if isinstance(value, dict):
        return {k: _sanitize_value(v) for k, v in value.items()}
    if isinstance(value, list):
        return [_sanitize_value(item) for item in value]
    return value
