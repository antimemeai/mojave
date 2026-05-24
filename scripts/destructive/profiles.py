"""Load and validate RunPod deployment profiles."""

from pathlib import Path

import yaml

REQUIRED_FIELDS = [
    "model",
    "gpu_type",
    "gpu_count",
    "tensor_parallel_size",
    "max_model_len",
    "enforce_eager",
    "gpu_memory_utilization",
    "container_disk_gb",
    "volume_gb",
    "total_pods",
    "batch_size",
    "hourly_cost_per_pod",
]


class ProfileError(Exception):
    pass


def load_profile(path: Path) -> dict:
    if not path.exists():
        raise ProfileError(f"Profile not found: {path}")

    with open(path) as f:
        raw = yaml.safe_load(f)

    if not isinstance(raw, dict):
        raise ProfileError(f"Profile must be a YAML mapping: {path}")

    for field in REQUIRED_FIELDS:
        if field not in raw:
            raise ProfileError(f"Missing required field '{field}' in {path}")

    raw["docker_args"] = _build_docker_args(raw)
    raw["env"] = _build_env(raw)
    return raw


def _build_docker_args(profile: dict) -> str:
    parts = [
        f"--model {profile['model']}",
        f"--max-model-len {profile['max_model_len']}",
        f"--gpu-memory-utilization {profile['gpu_memory_utilization']}",
        f"--tensor-parallel-size {profile['tensor_parallel_size']}",
        "--host 0.0.0.0",
        "--port 8000",
    ]
    if profile.get("enforce_eager"):
        parts.append("--enforce-eager")
    return " ".join(parts)


def _build_env(profile: dict) -> dict:
    env = {}
    use_v1 = profile.get("vllm_use_v1", True)
    if not use_v1:
        env["VLLM_USE_V1"] = "0"
    return env
