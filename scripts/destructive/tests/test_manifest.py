import json
import subprocess
from pathlib import Path


def test_gen_manifest_produces_valid_json(tmp_path: Path) -> None:
    out = tmp_path / "manifest.json"
    result = subprocess.run(
        [
            "python3",
            "scripts/destructive/gen_destructive_manifest.py",
            "inspect_evals/wmdp_chem",
            str(out),
        ],
        capture_output=True,
        text=True,
    )
    assert result.returncode == 0, result.stderr
    manifest = json.loads(out.read_text())
    assert manifest["task"] == "inspect_evals/wmdp_chem"
    assert manifest["model"] == "Qwen/Qwen2.5-7B-Instruct"
    assert manifest["total_variants"] == len(manifest["runs"])
    assert manifest["total_variants"] > 0


def test_gen_manifest_accepts_model_override(tmp_path: Path) -> None:
    out = tmp_path / "manifest.json"
    result = subprocess.run(
        [
            "python3",
            "scripts/destructive/gen_destructive_manifest.py",
            "inspect_evals/wmdp_chem",
            str(out),
            "--model",
            "Qwen/Qwen2.5-72B-Instruct",
        ],
        capture_output=True,
        text=True,
    )
    assert result.returncode == 0, result.stderr
    manifest = json.loads(out.read_text())
    assert manifest["model"] == "Qwen/Qwen2.5-72B-Instruct"


def test_no_dead_fields_in_variants(tmp_path: Path) -> None:
    out = tmp_path / "manifest.json"
    subprocess.run(
        [
            "python3",
            "scripts/destructive/gen_destructive_manifest.py",
            "inspect_evals/arc_challenge",
            str(out),
        ],
        capture_output=True,
        text=True,
    )
    manifest = json.loads(out.read_text())
    for v in manifest["runs"]:
        assert "few_shot" not in v, f"Dead field 'few_shot' in {v['variant_id']}"
        assert "label_format" not in v, f"Dead field 'label_format' in {v['variant_id']}"


def test_all_variants_have_required_fields(tmp_path: Path) -> None:
    out = tmp_path / "manifest.json"
    subprocess.run(
        [
            "python3",
            "scripts/destructive/gen_destructive_manifest.py",
            "inspect_evals/arc_challenge",
            str(out),
        ],
        capture_output=True,
        text=True,
    )
    manifest = json.loads(out.read_text())
    required = {
        "variant_id",
        "block",
        "description",
        "temperature",
        "order_seed",
        "prompt_template",
        "system_prompt",
    }
    for v in manifest["runs"]:
        missing = required - v.keys()
        assert not missing, f"Missing fields in {v['variant_id']}: {missing}"
