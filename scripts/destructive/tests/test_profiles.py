import sys
from pathlib import Path

import pytest

# Add scripts/destructive to sys.path for imports
sys.path.insert(0, str(Path(__file__).parent.parent))

from profiles import ProfileError, load_profile

PROFILES_DIR = Path(__file__).parent.parent / "profiles"


def test_load_qwen_7b_profile():
    p = load_profile(PROFILES_DIR / "qwen-7b-3090.yaml")
    assert p["model"] == "Qwen/Qwen2.5-7B-Instruct"
    assert p["gpu_type"] == "NVIDIA GeForce RTX 3090"
    assert p["gpu_count"] == 1
    assert p["tensor_parallel_size"] == 1
    assert "--enforce-eager" in p["docker_args"]
    assert p["env"]["VLLM_USE_V1"] == "0"


def test_load_qwen_72b_profile():
    p = load_profile(PROFILES_DIR / "qwen-72b-h100.yaml")
    assert p["model"] == "Qwen/Qwen2.5-72B-Instruct"
    assert p["gpu_count"] == 4
    assert p["tensor_parallel_size"] == 4
    assert "--enforce-eager" not in p["docker_args"]


def test_missing_profile_raises():
    with pytest.raises(ProfileError, match="not found"):
        load_profile(Path("/nonexistent/profile.yaml"))


def test_profile_missing_required_field(tmp_path):
    bad = tmp_path / "bad.yaml"
    bad.write_text("model: Foo\n")
    with pytest.raises(ProfileError, match="gpu_type"):
        load_profile(bad)


def test_docker_args_built_from_fields():
    p = load_profile(PROFILES_DIR / "qwen-7b-3090.yaml")
    args = p["docker_args"]
    assert "--model Qwen/Qwen2.5-7B-Instruct" in args
    assert "--tensor-parallel-size 1" in args
    assert "--host 0.0.0.0" in args
    assert "--port 8000" in args
