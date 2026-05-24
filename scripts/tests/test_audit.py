"""Tests for the Python audit chain writer.

Verifies canonical encoding, hash computation, chain linking,
and cross-language compatibility with `mojave audit verify`.
"""

import hashlib
import json
import shutil
import subprocess
import sys
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).parent.parent))

from audit import AuditChain, canonical_json


class TestCanonicalJson:
    def test_empty_object(self) -> None:
        assert canonical_json({}) == "{}"

    def test_sorted_keys(self) -> None:
        assert canonical_json({"b": 1, "a": 2, "c": 3}) == '{"a":2,"b":1,"c":3}'

    def test_nested_sorted_keys(self) -> None:
        assert canonical_json({"outer": {"z": 1, "a": 2}}) == '{"outer":{"a":2,"z":1}}'

    def test_no_whitespace(self) -> None:
        result = canonical_json({"a": [1, {"b": 2}], "c": "hello"})
        assert " " not in result
        assert "\n" not in result
        assert "\t" not in result

    def test_array_order_preserved(self) -> None:
        assert canonical_json([3, 1, 2]) == "[3,1,2]"

    def test_string_escaping(self) -> None:
        result = canonical_json({"s": 'a\\b"c'})
        assert "\\\\" in result
        assert '\\"' in result

    def test_control_chars(self) -> None:
        result = canonical_json({"s": "\x00\n"})
        assert "\\u0000" in result
        assert "\\n" in result

    def test_tab_escape(self) -> None:
        result = canonical_json({"s": "a\tb"})
        assert "\\t" in result

    def test_float_rejected(self) -> None:
        with pytest.raises(ValueError, match="Floats not allowed"):
            canonical_json({"n": 1.5})

    def test_null(self) -> None:
        assert canonical_json(None) == "null"

    def test_bool(self) -> None:
        assert canonical_json(True) == "true"
        assert canonical_json(False) == "false"

    def test_integer(self) -> None:
        assert canonical_json({"n": -42}) == '{"n":-42}'

    def test_unicode_passthrough(self) -> None:
        result = canonical_json({"emoji": "\U0001f525"})
        assert "\U0001f525" in result

    def test_deterministic(self) -> None:
        obj = {"z": 1, "a": 2, "m": [3, 4]}
        assert canonical_json(obj) == canonical_json(obj)


class TestAuditChain:
    def test_genesis_entry_has_null_parent(self, tmp_path: Path) -> None:
        chain = AuditChain(tmp_path)
        chain.emit("eval.started")

        lines = (tmp_path / "chain.jsonl").read_text().strip().splitlines()
        entry = json.loads(lines[0])
        assert entry["parent_hash"] is None
        assert entry["base"]["seq"] == 0

    def test_second_entry_chains_to_first(self, tmp_path: Path) -> None:
        chain = AuditChain(tmp_path)
        chain.emit("eval.started")
        chain.emit("eval.completed")

        lines = (tmp_path / "chain.jsonl").read_text().strip().splitlines()
        first = json.loads(lines[0])
        second = json.loads(lines[1])
        assert second["parent_hash"] == first["entry_hash"]
        assert second["base"]["seq"] == 1

    def test_hash_is_deterministic(self, tmp_path: Path) -> None:
        chain = AuditChain(tmp_path)
        chain.emit("eval.started")

        lines = (tmp_path / "chain.jsonl").read_text().strip().splitlines()
        entry = json.loads(lines[0])
        entry_hash = bytes(entry["entry_hash"])

        canonical = canonical_json(entry["base"])
        hasher = hashlib.sha256()
        hasher.update(b"mojave-audit-v1\x00")
        hasher.update(canonical.encode("utf-8"))
        hasher.update(bytes(32))
        assert hasher.digest() == entry_hash

    def test_head_file_written(self, tmp_path: Path) -> None:
        chain = AuditChain(tmp_path)
        chain.emit("eval.started")

        head = json.loads((tmp_path / "chain-head.json").read_text())
        assert head["next_seq"] == 1
        assert "tip_hash" in head

    def test_resume_from_head(self, tmp_path: Path) -> None:
        chain = AuditChain(tmp_path)
        chain.emit("eval.started")
        chain.emit("eval.completed")
        tip = chain.tip_hash

        chain2 = AuditChain(tmp_path)
        assert chain2.next_seq == 2
        assert chain2.tip_hash == tip

    def test_resume_from_chain_without_head(self, tmp_path: Path) -> None:
        chain = AuditChain(tmp_path)
        chain.emit("eval.started")
        chain.emit("eval.completed")
        tip = chain.tip_hash

        (tmp_path / "chain-head.json").unlink()

        chain2 = AuditChain(tmp_path)
        assert chain2.next_seq == 2
        assert chain2.tip_hash == tip

    def test_resource_included_when_provided(self, tmp_path: Path) -> None:
        chain = AuditChain(tmp_path)
        chain.emit("pod.created", resource_kind="pod", resource_id="abc123")

        lines = (tmp_path / "chain.jsonl").read_text().strip().splitlines()
        entry = json.loads(lines[0])
        assert entry["base"]["resource"] == {"kind": "pod", "id": "abc123"}

    def test_resource_omitted_when_not_provided(self, tmp_path: Path) -> None:
        chain = AuditChain(tmp_path)
        chain.emit("eval.started")

        lines = (tmp_path / "chain.jsonl").read_text().strip().splitlines()
        entry = json.loads(lines[0])
        assert "resource" not in entry["base"]

    def test_tags_omitted_when_empty(self, tmp_path: Path) -> None:
        chain = AuditChain(tmp_path)
        chain.emit("eval.started")

        lines = (tmp_path / "chain.jsonl").read_text().strip().splitlines()
        entry = json.loads(lines[0])
        assert "tags" not in entry["base"]

    def test_tags_included_when_provided(self, tmp_path: Path) -> None:
        chain = AuditChain(tmp_path)
        chain.emit("eval.started", tags={"model": "qwen-7b"})

        lines = (tmp_path / "chain.jsonl").read_text().strip().splitlines()
        entry = json.loads(lines[0])
        assert entry["base"]["tags"] == {"model": "qwen-7b"}

    def test_detail_defaults_to_empty_object(self, tmp_path: Path) -> None:
        chain = AuditChain(tmp_path)
        chain.emit("eval.started")

        lines = (tmp_path / "chain.jsonl").read_text().strip().splitlines()
        entry = json.loads(lines[0])
        assert entry["base"]["detail"] == {}

    def test_five_entry_chain(self, tmp_path: Path) -> None:
        chain = AuditChain(tmp_path)
        for i in range(5):
            chain.emit(
                "eval.started",
                resource_kind="eval",
                resource_id=f"run-{i}",
                detail={"index": i},
            )

        lines = (tmp_path / "chain.jsonl").read_text().strip().splitlines()
        assert len(lines) == 5

        prev_hash = None
        for i, line in enumerate(lines):
            entry = json.loads(line)
            assert entry["base"]["seq"] == i
            if prev_hash is None:
                assert entry["parent_hash"] is None
            else:
                assert entry["parent_hash"] == prev_hash
            prev_hash = entry["entry_hash"]


class TestRustVerification:
    """Cross-validate Python chain with Rust `mojave audit verify`."""

    @pytest.fixture()
    def mojave_bin(self) -> Path | None:
        bin_path = shutil.which("mojave")
        if bin_path:
            return Path(bin_path)
        cargo_bin = Path("target/release/mojave")
        if cargo_bin.exists():
            return cargo_bin
        cargo_debug = Path("target/debug/mojave")
        if cargo_debug.exists():
            return cargo_debug
        return None

    def test_rust_verifier_accepts_python_chain(
        self, tmp_path: Path, mojave_bin: Path | None
    ) -> None:
        if mojave_bin is None:
            pytest.skip("mojave binary not built — run `cargo build`")

        chain = AuditChain(tmp_path)
        chain.emit("eval.started", detail={"trial": 1})
        chain.emit(
            "pod.created",
            resource_kind="pod",
            resource_id="test-pod-01",
            detail={"gpu": "RTX 3090"},
        )
        chain.emit(
            "eval.completed",
            resource_kind="eval",
            resource_id="wmdp_chem",
            outcome="Succeeded",
            tags={"model": "Qwen/Qwen2.5-7B-Instruct"},
            detail={"accuracy": 80, "samples": 5},
        )

        result = subprocess.run(
            [str(mojave_bin), "audit", "verify", "--chain", str(tmp_path / "chain.jsonl")],
            capture_output=True,
            text=True,
        )

        assert result.returncode == 0, f"verify failed: {result.stderr}\n{result.stdout}"

        output = json.loads(result.stdout)
        assert output["is_clean"] is True
        assert output["entries_verified"] == 3
