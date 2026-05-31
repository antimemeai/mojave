#!/usr/bin/env python3
"""Create vLLM eval pods on RunPod from a deployment profile.

Uses the vllm/vllm-openai image so pods come up serving automatically.
Model/GPU config loaded from a YAML profile.

Usage:
    python create_pods.py profiles/qwen-7b-3090.yaml
    python create_pods.py profiles/qwen-72b-h100.yaml
    python create_pods.py profiles/qwen-7b-3090.yaml --pods 4
"""

import argparse
import json
import subprocess
import sys
import time
from pathlib import Path

import runpod

sys.path.insert(0, str(Path(__file__).parent.parent))

from audit_emit import emit as audit
from profiles import load_profile

PODS_FILE = Path("data/destructive/pods.json")
ENDPOINTS_FILE = Path("data/destructive/endpoints.json")
META_FILE = Path("data/destructive/meta.json")


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


def create_batch(profile: dict, start_index: int, count: int) -> list[dict]:
    created = []
    for i in range(count):
        idx = start_index + i
        name = f"mojave-{idx:02d}"
        print(f"  Creating {name}...", file=sys.stderr)
        try:
            pod = runpod.create_pod(
                name=name,
                image_name="vllm/vllm-openai:latest",
                gpu_type_id=profile["gpu_type"],
                gpu_count=profile["gpu_count"],
                volume_in_gb=profile["volume_gb"],
                container_disk_in_gb=profile["container_disk_gb"],
                ports="8000/http",
                cloud_type="ALL",
                docker_args=profile["docker_args"],
                env=profile["env"],
            )
            print(f"  {name}: {pod['id']}", file=sys.stderr)
            created.append({"id": pod["id"], "name": name})
            audit(
                "pod.created",
                resource_kind="pod",
                resource_id=pod["id"],
                detail={
                    "name": name,
                    "gpu_type": profile["gpu_type"],
                    "gpu_count": profile["gpu_count"],
                    "model": profile["model"],
                },
            )
        except Exception as e:
            print(f"  {name}: FAILED — {e}", file=sys.stderr)
    return created


def main() -> None:
    parser = argparse.ArgumentParser(description="Create vLLM eval pods from a profile")
    parser.add_argument("profile", type=Path, help="Path to profile YAML")
    parser.add_argument("--pods", type=int, default=None, help="Override total pod count")
    args = parser.parse_args()

    profile = load_profile(args.profile)
    total_pods = args.pods or profile["total_pods"]
    batch_size: int = profile["batch_size"]
    cost_per_pod: float = profile["hourly_cost_per_pod"]

    runpod.check_credentials()
    existing = load_pods()
    have = len(existing)
    need = total_pods - have

    if need <= 0:
        print(f"Already have {have} pods.", file=sys.stderr)
        return

    total_cost = total_pods * cost_per_pod
    print(f"Profile: {args.profile.name}", file=sys.stderr)
    print(f"Model: {profile['model']}", file=sys.stderr)
    print(f"GPU: {profile['gpu_type']} x{profile['gpu_count']}", file=sys.stderr)
    print(f"Creating {need} pods ({batch_size} at a time)...", file=sys.stderr)
    cost_msg = f"${total_cost:.2f}/hr ({total_pods} pods x ${cost_per_pod}/hr)"
    print(f"Estimated cost: {cost_msg}", file=sys.stderr)

    all_pods = list(existing)
    created_count = 0

    while created_count < need:
        this_batch = min(batch_size, need - created_count)
        start_idx = have + created_count
        print(f"\n--- Batch {created_count // batch_size + 1} ---", file=sys.stderr)

        batch = create_batch(profile, start_idx, this_batch)
        if not batch:
            print("Batch failed entirely. Stopping.", file=sys.stderr)
            break

        all_pods.extend(batch)
        save_pods(all_pods)
        created_count += len(batch)
        print(f"Progress: {len(all_pods)}/{total_pods}", file=sys.stderr)
        time.sleep(5)

    print("\nAll pods created. Polling for vLLM readiness...", file=sys.stderr)
    pod_ids = [p["id"] for p in all_pods]
    ready: set[str] = set()
    for _attempt in range(90):
        for pid in pod_ids:
            if pid not in ready and check_endpoint(pid):
                ready.add(pid)
                print(f"  {pid}: READY ({len(ready)}/{len(pod_ids)})", file=sys.stderr)
                audit(
                    "pod.ready",
                    resource_kind="pod",
                    resource_id=pid,
                    detail={"model": profile["model"]},
                )
        if len(ready) == len(pod_ids):
            break
        time.sleep(10)

    endpoints = [f"https://{pid}-8000.proxy.runpod.net/v1" for pid in ready]
    ENDPOINTS_FILE.parent.mkdir(parents=True, exist_ok=True)
    ENDPOINTS_FILE.write_text(json.dumps(endpoints, indent=2) + "\n")

    meta = {
        "profile": str(args.profile),
        "model": profile["model"],
        "hourly_cost_per_pod": cost_per_pod,
        "created_at": time.strftime("%Y-%m-%dT%H:%M:%SZ"),
    }
    META_FILE.write_text(json.dumps(meta, indent=2) + "\n")

    print(f"\n{len(endpoints)}/{len(pod_ids)} endpoints ready -> {ENDPOINTS_FILE}", file=sys.stderr)

    if len(ready) < len(pod_ids):
        not_ready = [pid for pid in pod_ids if pid not in ready]
        print(f"NOT READY: {not_ready}", file=sys.stderr)


if __name__ == "__main__":
    main()
