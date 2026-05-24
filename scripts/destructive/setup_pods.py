#!/usr/bin/env python3
"""SSH into each RunPod and install vLLM + start serving.

Reads pod IDs from data/destructive/pods.json, SSHes in parallel,
runs setup commands, then polls until vLLM is serving on port 8000.
"""

from __future__ import annotations

import json
import subprocess
import sys
import time
from concurrent.futures import ThreadPoolExecutor, as_completed
from pathlib import Path

import runpod

SETUP_COMMANDS_24GB = """
set -euxo pipefail
pip install --quiet vllm inspect-ai inspect-evals 2>&1 | tail -5
python3 -c "from huggingface_hub import snapshot_download; \
snapshot_download('Qwen/Qwen2.5-7B-Instruct')"
nohup python3 -m vllm.entrypoints.openai.api_server \
  --model Qwen/Qwen2.5-7B-Instruct \
  --max-model-len 4096 \
  --gpu-memory-utilization 0.9 \
  --tensor-parallel-size 1 \
  --host 0.0.0.0 \
  --port 8000 \
  > /var/log/vllm-server.log 2>&1 &
echo "vLLM started in background"
""".strip()

SETUP_COMMANDS_16GB = """
set -euxo pipefail
pip install --quiet vllm inspect-ai inspect-evals 2>&1 | tail -5
python3 -c "from huggingface_hub import snapshot_download; \
snapshot_download('Qwen/Qwen2.5-7B-Instruct')"
nohup python3 -m vllm.entrypoints.openai.api_server \
  --model Qwen/Qwen2.5-7B-Instruct \
  --max-model-len 2048 \
  --gpu-memory-utilization 0.95 \
  --tensor-parallel-size 1 \
  --host 0.0.0.0 \
  --port 8000 \
  > /var/log/vllm-server.log 2>&1 &
echo "vLLM started in background"
""".strip()

SETUP_COMMANDS = SETUP_COMMANDS_24GB


def get_ssh_command(pod_id: str) -> list[str] | None:
    info = runpod.get_pod(pod_id)
    runtime = info.get("runtime", {}) or {}
    ports = runtime.get("ports", []) or []
    for port in ports:
        if port.get("privatePort") == 22 and port.get("ip"):
            return [
                "ssh",
                "-o",
                "StrictHostKeyChecking=no",
                "-o",
                "ConnectTimeout=10",
                "-p",
                str(port["publicPort"]),
                f"root@{port['ip']}",
            ]
    return None


def setup_pod(pod_id: str, gpu: str, index: int, total: int) -> tuple[str, bool, str]:
    cmds = SETUP_COMMANDS_16GB if gpu == "a4000" else SETUP_COMMANDS_24GB
    print(f"[{index}/{total}] {pod_id} ({gpu}): getting SSH info...", file=sys.stderr)
    ssh_cmd = get_ssh_command(pod_id)
    if not ssh_cmd:
        return pod_id, False, "no SSH port found"

    print(f"[{index}/{total}] {pod_id} ({gpu}): running setup via SSH...", file=sys.stderr)
    result = subprocess.run(
        [*ssh_cmd, cmds],
        capture_output=True,
        text=True,
        timeout=600,
    )
    if result.returncode != 0:
        return pod_id, False, result.stderr[:300]

    print(f"[{index}/{total}] {pod_id} ({gpu}): setup complete", file=sys.stderr)
    return pod_id, True, ""


def check_endpoint(pod_id: str) -> bool:
    """Check if vLLM is serving on the pod's proxy."""
    url = f"https://{pod_id}-8000.proxy.runpod.net/v1/models"
    try:
        result = subprocess.run(
            ["curl", "-s", "-o", "/dev/null", "-w", "%{http_code}", url],
            capture_output=True,
            text=True,
            timeout=10,
        )
        return result.stdout.strip() == "200"
    except Exception:
        return False


def main() -> None:
    pods_file = Path("data/destructive/pods.json")
    pods = json.loads(pods_file.read_text())
    total = len(pods)

    print(f"Setting up {total} pods...", file=sys.stderr)

    with ThreadPoolExecutor(max_workers=5) as pool:
        futures = {
            pool.submit(setup_pod, p["id"], p.get("gpu", "3090"), i + 1, total): p["id"]
            for i, p in enumerate(pods)
        }
        results = {}
        for future in as_completed(futures):
            pid, ok, err = future.result()
            results[pid] = (ok, err)
            if not ok:
                print(f"  FAILED {pid}: {err}", file=sys.stderr)

    succeeded = [pid for pid, (ok, _) in results.items() if ok]
    failed = [pid for pid, (ok, _) in results.items() if not ok]
    print(f"\nSetup: {len(succeeded)} ok, {len(failed)} failed", file=sys.stderr)

    if failed:
        print(f"Failed pods: {failed}", file=sys.stderr)

    # Poll for vLLM readiness
    print("\nWaiting for vLLM to start serving...", file=sys.stderr)
    ready = set()
    for _attempt in range(60):
        for pid in succeeded:
            if pid not in ready and check_endpoint(pid):
                ready.add(pid)
                print(f"  {pid}: READY ({len(ready)}/{len(succeeded)})", file=sys.stderr)
        if len(ready) == len(succeeded):
            break
        time.sleep(10)

    endpoints = [f"https://{pid}-8000.proxy.runpod.net/v1" for pid in ready]

    endpoints_file = Path("data/destructive/endpoints.json")
    endpoints_file.write_text(json.dumps(endpoints, indent=2) + "\n")
    print(f"\n{len(endpoints)} endpoints ready -> {endpoints_file}", file=sys.stderr)

    if len(ready) < len(succeeded):
        not_ready = set(succeeded) - ready
        print(f"NOT READY: {not_ready}", file=sys.stderr)


if __name__ == "__main__":
    main()
