#!/usr/bin/env python3
"""Stand up a mixed-GPU fleet from multiple profiles.

Creates pods from each profile, polls for readiness, and writes a
quantization-keyed endpoints file that run_mcq.py can consume directly.

Usage:
    python create_fleet.py profiles/qwen-7b-3090.yaml profiles/qwen-7b-4090.yaml
    python create_fleet.py profiles/qwen-7b-3090.yaml --pods-per-profile 4
    python create_fleet.py profiles/qwen-7b-*.yaml
"""

from __future__ import annotations

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

FLEET_FILE = Path("data/destructive/fleet.json")
ENDPOINTS_FILE = Path("data/destructive/endpoints.json")


def load_fleet() -> list[dict]:
    if FLEET_FILE.exists():
        data = json.loads(FLEET_FILE.read_text())
        if isinstance(data, list):
            return data
    return []


def save_fleet(fleet: list[dict]) -> None:
    FLEET_FILE.parent.mkdir(parents=True, exist_ok=True)
    FLEET_FILE.write_text(json.dumps(fleet, indent=2) + "\n")


def check_endpoint(pod_id: str) -> bool:
    url = f"https://{pod_id}-8000.proxy.runpod.net/v1/models"
    try:
        r = subprocess.run(
            [
                "curl",
                "-s",
                "-o",
                "/dev/null",
                "-w",
                "%{http_code}",
                "--max-time",
                "5",
                url,
            ],
            capture_output=True,
            text=True,
            timeout=10,
        )
        return r.stdout.strip() == "200"
    except Exception:
        return False


def create_pod_for_profile(
    profile: dict,
    name: str,
    profile_name: str,
) -> dict | None:
    print(f"  Creating {name} ({profile_name})...", file=sys.stderr)
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
        quant = profile.get("quantization", "bf16")
        entry = {
            "id": pod["id"],
            "name": name,
            "gpu_type": profile["gpu_type"],
            "quantization": quant,
            "profile": profile_name,
        }
        print(f"  {name}: {pod['id']} ({quant})", file=sys.stderr)
        audit(
            "pod.created",
            resource_kind="pod",
            resource_id=pod["id"],
            detail={
                "name": name,
                "gpu_type": profile["gpu_type"],
                "gpu_count": profile["gpu_count"],
                "model": profile["model"],
                "quantization": quant,
            },
        )
        return entry
    except Exception as e:
        print(f"  {name}: FAILED -- {e}", file=sys.stderr)
        return None


def write_endpoints(fleet: list[dict], ready_ids: set[str]) -> None:
    endpoints: dict[str, list[str]] = {"bf16": [], "fp8": []}
    for pod in fleet:
        if pod["id"] in ready_ids:
            url = f"https://{pod['id']}-8000.proxy.runpod.net/v1"
            quant = pod.get("quantization", "bf16")
            endpoints.setdefault(quant, []).append(url)
    ENDPOINTS_FILE.parent.mkdir(parents=True, exist_ok=True)
    ENDPOINTS_FILE.write_text(json.dumps(endpoints, indent=2) + "\n")


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "profiles",
        nargs="+",
        type=Path,
        help="Profile YAML files",
    )
    parser.add_argument(
        "--pods-per-profile",
        type=int,
        default=None,
        help="Override pod count per profile",
    )
    args = parser.parse_args()

    runpod.check_credentials()
    fleet = load_fleet()
    existing_count = len(fleet)

    total_hourly = 0.0
    plan: list[tuple[dict, str, int]] = []

    for profile_path in args.profiles:
        profile = load_profile(profile_path)
        n_pods = args.pods_per_profile or profile["total_pods"]
        already = sum(1 for p in fleet if p["profile"] == profile_path.name)
        need = max(0, n_pods - already)
        if need > 0:
            plan.append((profile, profile_path.name, need))
            total_hourly += need * profile["hourly_cost_per_pod"]

    if not plan:
        print(f"Fleet already at target ({existing_count} pods).", file=sys.stderr)
        return

    total_new = sum(n for _, _, n in plan)
    print(f"Fleet plan: {total_new} new pods across {len(plan)} profiles", file=sys.stderr)
    print(f"Estimated cost: ${total_hourly:.2f}/hr", file=sys.stderr)
    for profile, name, n in plan:
        quant = profile.get("quantization", "bf16")
        print(f"  {name}: {n} x {profile['gpu_type']} ({quant})", file=sys.stderr)
    print(file=sys.stderr)

    pod_idx = existing_count
    for profile, profile_name, n_pods in plan:
        print(f"--- {profile_name} ({n_pods} pods) ---", file=sys.stderr)
        for _ in range(n_pods):
            name = f"mojave-{pod_idx:02d}"
            entry = create_pod_for_profile(profile, name, profile_name)
            if entry:
                fleet.append(entry)
                save_fleet(fleet)
                pod_idx += 1
            time.sleep(2)

    print(
        f"\n{len(fleet)} total pods. Polling for readiness...",
        file=sys.stderr,
    )
    all_ids = [p["id"] for p in fleet]
    ready: set[str] = set()
    for _attempt in range(90):
        for pid in all_ids:
            if pid not in ready and check_endpoint(pid):
                ready.add(pid)
                pod_entry = next(p for p in fleet if p["id"] == pid)
                print(
                    f"  {pid} ({pod_entry['gpu_type']}): READY ({len(ready)}/{len(all_ids)})",
                    file=sys.stderr,
                )
                audit(
                    "pod.ready",
                    resource_kind="pod",
                    resource_id=pid,
                    detail={"gpu_type": pod_entry["gpu_type"]},
                )
        if len(ready) == len(all_ids):
            break
        time.sleep(10)

    write_endpoints(fleet, ready)

    bf16_count = sum(
        1 for p in fleet if p["id"] in ready and p.get("quantization", "bf16") == "bf16"
    )
    fp8_count = sum(1 for p in fleet if p["id"] in ready and p.get("quantization") == "fp8")
    print(
        f"\nEndpoints: {bf16_count} bf16 + {fp8_count} fp8 -> {ENDPOINTS_FILE}",
        file=sys.stderr,
    )

    if len(ready) < len(all_ids):
        not_ready = [pid for pid in all_ids if pid not in ready]
        print(f"NOT READY: {not_ready}", file=sys.stderr)


if __name__ == "__main__":
    main()
