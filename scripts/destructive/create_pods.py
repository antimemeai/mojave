#!/usr/bin/env python3
"""Create vLLM eval pods on RunPod.

Uses the vllm/vllm-openai image so pods come up serving automatically —
no SSH setup needed. Model flags go via docker_args, env vars via env.

See RUNPOD_RUNBOOK.md for lessons learned.
"""

from __future__ import annotations

import json
import subprocess
import sys
import time
from pathlib import Path

import runpod

TOTAL_PODS = 8
BATCH_SIZE = 3
PODS_FILE = Path("data/destructive/pods.json")
ENDPOINTS_FILE = Path("data/destructive/endpoints.json")

GPU_TYPE = "NVIDIA GeForce RTX 3090"

DOCKER_ARGS = (
    "--model Qwen/Qwen2.5-7B-Instruct "
    "--max-model-len 4096 "
    "--enforce-eager "
    "--gpu-memory-utilization 0.9 "
    "--tensor-parallel-size 1 "
    "--host 0.0.0.0 "
    "--port 8000"
)

POD_ENV = {
    "VLLM_USE_V1": "0",
}

POD_CONFIG = dict(
    image_name="vllm/vllm-openai:latest",
    gpu_type_id=GPU_TYPE,
    gpu_count=1,
    volume_in_gb=50,
    container_disk_in_gb=50,
    ports="8000/http",
    cloud_type="ALL",
    docker_args=DOCKER_ARGS,
    env=POD_ENV,
)


def load_pods() -> list[dict]:
    if PODS_FILE.exists():
        data = json.loads(PODS_FILE.read_text())
        if isinstance(data, list):
            return data
    return []


def save_pods(pods: list[dict]) -> None:
    PODS_FILE.parent.mkdir(parents=True, exist_ok=True)
    PODS_FILE.write_text(json.dumps(pods, indent=2) + "\n")


def check_endpoint(pod_id: str) -> bool:
    url = f"https://{pod_id}-8000.proxy.runpod.net/v1/models"
    try:
        r = subprocess.run(
            ["curl", "-s", "-o", "/dev/null", "-w", "%{http_code}", "--max-time", "5", url],
            capture_output=True,
            text=True,
            timeout=10,
        )
        return r.stdout.strip() == "200"
    except Exception:
        return False


def create_batch(start_index: int, count: int) -> list[dict]:
    created = []
    for i in range(count):
        idx = start_index + i
        name = f"mojave-{idx:02d}"
        print(f"  Creating {name}...", file=sys.stderr)
        try:
            pod = runpod.create_pod(name=name, **POD_CONFIG)
            print(f"  {name}: {pod['id']}", file=sys.stderr)
            created.append({"id": pod["id"], "name": name})
        except Exception as e:
            print(f"  {name}: FAILED — {e}", file=sys.stderr)
    return created


def main() -> None:
    runpod.check_credentials()
    existing = load_pods()
    have = len(existing)
    need = TOTAL_PODS - have

    if need <= 0:
        print(f"Already have {have} pods.", file=sys.stderr)
        return

    print(f"Creating {need} pods ({BATCH_SIZE} at a time)...", file=sys.stderr)
    print(f"Image: {POD_CONFIG['image_name']}", file=sys.stderr)
    print(f"GPU: {GPU_TYPE}", file=sys.stderr)
    print(f"Args: {DOCKER_ARGS}", file=sys.stderr)
    print(f"Env: {POD_ENV}", file=sys.stderr)

    all_pods = list(existing)
    created_count = 0

    while created_count < need:
        batch_size = min(BATCH_SIZE, need - created_count)
        start_idx = have + created_count
        print(f"\n--- Batch {created_count // BATCH_SIZE + 1} ---", file=sys.stderr)

        batch = create_batch(start_idx, batch_size)
        if not batch:
            print("Batch failed entirely. Stopping.", file=sys.stderr)
            break

        all_pods.extend(batch)
        save_pods(all_pods)
        created_count += len(batch)
        print(f"Progress: {len(all_pods)}/{TOTAL_PODS}", file=sys.stderr)
        time.sleep(5)

    # Poll for vLLM readiness
    print("\nAll pods created. Polling for vLLM readiness...", file=sys.stderr)
    pod_ids = [p["id"] for p in all_pods]
    ready = set()
    for _attempt in range(90):
        for pid in pod_ids:
            if pid not in ready and check_endpoint(pid):
                ready.add(pid)
                print(f"  {pid}: READY ({len(ready)}/{len(pod_ids)})", file=sys.stderr)
        if len(ready) == len(pod_ids):
            break
        time.sleep(10)

    endpoints = [f"https://{pid}-8000.proxy.runpod.net/v1" for pid in ready]
    ENDPOINTS_FILE.write_text(json.dumps(endpoints, indent=2) + "\n")
    print(f"\n{len(endpoints)}/{len(pod_ids)} endpoints ready -> {ENDPOINTS_FILE}", file=sys.stderr)

    if len(ready) < len(pod_ids):
        not_ready = [pid for pid in pod_ids if pid not in ready]
        print(f"NOT READY: {not_ready}", file=sys.stderr)


if __name__ == "__main__":
    main()
