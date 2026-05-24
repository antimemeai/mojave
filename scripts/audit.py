"""Thin Python audit chain writer, compatible with `mojave audit verify`.

Produces chain.jsonl entries with the same hash computation as the Rust
audit-chain crate: SHA256(domain_tag || canonical_json || parent_hash).

Usage:
    from scripts.audit import emit

    emit("pod.created", resource_kind="pod", resource_id=pod_id,
         detail={"gpu_type": "RTX 3090"})
"""

import hashlib
import json
import threading
from datetime import UTC, datetime
from pathlib import Path

DOMAIN_TAG = b"mojave-audit-v1\x00"
GENESIS_SENTINEL = bytes(32)
DEFAULT_AUDIT_DIR = Path("data/audit")


class AuditChain:
    """Append-only audit chain writer producing Rust-verifiable entries."""

    def __init__(self, audit_dir: Path = DEFAULT_AUDIT_DIR) -> None:
        self.audit_dir = audit_dir
        self.chain_file = audit_dir / "chain.jsonl"
        self.head_file = audit_dir / "chain-head.json"
        self.tip_hash: bytes | None = None
        self.next_seq: int = 0
        self._lock = threading.Lock()
        self._load_head()

    def _load_head(self) -> None:
        self.audit_dir.mkdir(parents=True, exist_ok=True)

        if self.head_file.exists():
            head = json.loads(self.head_file.read_text())
            self.next_seq = head["next_seq"]
            tip_hex = head.get("tip_hash")
            self.tip_hash = bytes.fromhex(tip_hex) if tip_hex else None
        elif self.chain_file.exists():
            for line in self.chain_file.read_text().splitlines():
                line = line.strip()
                if not line:
                    continue
                try:
                    entry = json.loads(line)
                    self.tip_hash = bytes(entry["entry_hash"])
                    self.next_seq = entry["base"]["seq"] + 1
                except (json.JSONDecodeError, KeyError):
                    pass

    def _save_head(self) -> None:
        head: dict[str, object] = {"next_seq": self.next_seq}
        if self.tip_hash:
            head["tip_hash"] = self.tip_hash.hex()
        self.head_file.write_text(json.dumps(head, indent=2) + "\n")

    def emit(
        self,
        event_kind: str,
        *,
        resource_kind: str | None = None,
        resource_id: str | None = None,
        actor_kind: str = "System",
        actor_id: str = "mojave-python",
        authorization: str = "Allowed",
        outcome: str = "Succeeded",
        tags: dict[str, str] | None = None,
        detail: dict[str, object] | None = None,
    ) -> int:
        with self._lock:
            base = self._build_entry(
                event_kind=event_kind,
                resource_kind=resource_kind,
                resource_id=resource_id,
                actor_kind=actor_kind,
                actor_id=actor_id,
                authorization=authorization,
                outcome=outcome,
                tags=tags,
                detail=detail,
            )

            canonical = canonical_json(base)
            parent_bytes = self.tip_hash if self.tip_hash else GENESIS_SENTINEL

            hasher = hashlib.sha256()
            hasher.update(DOMAIN_TAG)
            hasher.update(canonical.encode("utf-8"))
            hasher.update(parent_bytes)
            entry_hash = hasher.digest()

            sealed = {
                "base": base,
                "parent_hash": list(self.tip_hash) if self.tip_hash else None,
                "entry_hash": list(entry_hash),
            }

            with open(self.chain_file, "a") as f:
                f.write(json.dumps(sealed, separators=(",", ":")) + "\n")

            seq = self.next_seq
            self.tip_hash = entry_hash
            self.next_seq += 1
            self._save_head()

            return seq

    def _build_entry(
        self,
        *,
        event_kind: str,
        resource_kind: str | None,
        resource_id: str | None,
        actor_kind: str,
        actor_id: str,
        authorization: str,
        outcome: str,
        tags: dict[str, str] | None,
        detail: dict[str, object] | None,
    ) -> dict[str, object]:
        now = datetime.now(UTC).replace(microsecond=0)
        entry: dict[str, object] = {
            "seq": self.next_seq,
            "envelope_version": 1,
            "at": now.strftime("%Y-%m-%dT%H:%M:%SZ"),
            "actor": {"kind": actor_kind, "id": actor_id},
            "event": event_kind,
            "authorization": authorization,
            "outcome": outcome,
            "detail": detail if detail is not None else {},
        }
        if resource_kind is not None and resource_id is not None:
            entry["resource"] = {"kind": resource_kind, "id": resource_id}
        if tags:
            entry["tags"] = dict(sorted(tags.items()))
        return entry


def canonical_json(obj: object) -> str:
    """Canonical JSON encoding matching Rust audit-chain canonical.rs.

    Sorted keys, no whitespace, standard escapes, floats rejected.
    """
    if obj is None:
        return "null"
    if isinstance(obj, bool):
        return "true" if obj else "false"
    if isinstance(obj, int):
        return str(obj)
    if isinstance(obj, float):
        raise ValueError(f"Floats not allowed in canonical encoding: {obj}")
    if isinstance(obj, str):
        return _escape_string(obj)
    if isinstance(obj, list):
        return "[" + ",".join(canonical_json(item) for item in obj) + "]"
    if isinstance(obj, dict):
        keys = sorted(obj.keys())
        pairs = [_escape_string(k) + ":" + canonical_json(obj[k]) for k in keys]
        return "{" + ",".join(pairs) + "}"
    raise ValueError(f"Unsupported type in canonical encoding: {type(obj)}")


def _escape_string(s: str) -> str:
    buf = ['"']
    for ch in s:
        if ch == '"':
            buf.append('\\"')
        elif ch == "\\":
            buf.append("\\\\")
        elif ch == "\x08":
            buf.append("\\b")
        elif ch == "\x0c":
            buf.append("\\f")
        elif ch == "\n":
            buf.append("\\n")
        elif ch == "\r":
            buf.append("\\r")
        elif ch == "\t":
            buf.append("\\t")
        elif ord(ch) < 0x20:
            buf.append(f"\\u{ord(ch):04x}")
        else:
            buf.append(ch)
    buf.append('"')
    return "".join(buf)


_chain: AuditChain | None = None


def emit(
    event_kind: str,
    *,
    resource_kind: str | None = None,
    resource_id: str | None = None,
    actor_kind: str = "System",
    actor_id: str = "mojave-python",
    authorization: str = "Allowed",
    outcome: str = "Succeeded",
    tags: dict[str, str] | None = None,
    detail: dict[str, object] | None = None,
    audit_dir: Path = DEFAULT_AUDIT_DIR,
) -> int:
    """Module-level emit — initializes chain on first call."""
    global _chain
    if _chain is None or _chain.audit_dir != audit_dir:
        _chain = AuditChain(audit_dir)
    return _chain.emit(
        event_kind,
        resource_kind=resource_kind,
        resource_id=resource_id,
        actor_kind=actor_kind,
        actor_id=actor_id,
        authorization=authorization,
        outcome=outcome,
        tags=tags,
        detail=detail,
    )
