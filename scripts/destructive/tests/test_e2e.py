"""End-to-end test: profile -> manifest -> command construction.

Does NOT call RunPod or run Inspect. Verifies the local toolchain
produces correct artifacts.
"""

import json
import subprocess
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent.parent))

from profiles import load_profile
from run_destructive import build_inspect_cmd


def test_profile_to_manifest_to_command(tmp_path: Path) -> None:
    profiles_dir = Path("scripts/destructive/profiles")
    profile = load_profile(profiles_dir / "qwen-7b-3090.yaml")

    manifest_path = tmp_path / "manifest.json"
    result = subprocess.run(
        [
            "python3",
            "scripts/destructive/gen_destructive_manifest.py",
            "inspect_evals/wmdp_chem",
            str(manifest_path),
            "--model",
            profile["model"],
        ],
        capture_output=True,
        text=True,
    )
    assert result.returncode == 0, result.stderr

    manifest = json.loads(manifest_path.read_text())
    assert manifest["model"] == "Qwen/Qwen2.5-7B-Instruct"
    assert manifest["task"] == "inspect_evals/wmdp_chem"

    variant = manifest["runs"][0]
    cmd = build_inspect_cmd(
        variant,
        manifest["task"],
        "https://fakepod-8000.proxy.runpod.net/v1",
        tmp_path / "logs",
        model=manifest["model"],
    )

    cmd_str = " ".join(cmd)
    assert "openai/Qwen/Qwen2.5-7B-Instruct" in cmd_str
    assert "inspect_evals/wmdp_chem" in cmd_str
    assert "fakepod" in cmd_str


def test_72b_profile_produces_correct_manifest(tmp_path: Path) -> None:
    profiles_dir = Path("scripts/destructive/profiles")
    profile = load_profile(profiles_dir / "qwen-72b-h100.yaml")

    manifest_path = tmp_path / "manifest.json"
    result = subprocess.run(
        [
            "python3",
            "scripts/destructive/gen_destructive_manifest.py",
            "inspect_evals/wmdp_bio",
            str(manifest_path),
            "--model",
            profile["model"],
        ],
        capture_output=True,
        text=True,
    )
    assert result.returncode == 0, result.stderr

    manifest = json.loads(manifest_path.read_text())
    assert manifest["model"] == "Qwen/Qwen2.5-72B-Instruct"
    assert manifest["task"] == "inspect_evals/wmdp_bio"
