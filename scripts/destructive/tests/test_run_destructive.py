import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent.parent))

from run_destructive import build_inspect_cmd, check_endpoint_health, variant_complete


def test_variant_complete_detects_eval_file(tmp_path: Path) -> None:
    assert not variant_complete(tmp_path, "v000")

    vdir = tmp_path / "v000"
    vdir.mkdir()
    assert not variant_complete(tmp_path, "v000")

    (vdir / "log.eval").touch()
    assert variant_complete(tmp_path, "v000")


def test_build_inspect_cmd_uses_model_from_manifest() -> None:
    variant = {
        "variant_id": "v000",
        "order_seed": 0,
        "system_prompt": "none",
        "prompt_template": "default",
        "temperature": 0.0,
    }
    cmd = build_inspect_cmd(
        variant,
        "inspect_evals/wmdp_chem",
        "https://example.com/v1",
        Path("/tmp/out"),
        model="Qwen/Qwen2.5-72B-Instruct",
        limit=None,
    )
    assert "openai/Qwen/Qwen2.5-72B-Instruct" in cmd
    assert "inspect_evals/wmdp_chem" in " ".join(cmd)


def test_check_endpoint_health_returns_false_on_timeout() -> None:
    assert not check_endpoint_health("nonexistent-pod-id-12345")
