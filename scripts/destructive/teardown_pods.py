#!/usr/bin/env python3
"""Terminate all destructive perturbation pods.

Shows cost estimate and requires confirmation before terminating.
Cleans up pods.json, endpoints.json, and meta.json after teardown.
"""

import json
import sys
from pathlib import Path

import runpod

sys.path.insert(0, str(Path(__file__).parent.parent))

from audit_emit import emit as audit

PODS_FILE = Path("data/destructive/pods.json")
ENDPOINTS_FILE = Path("data/destructive/endpoints.json")
META_FILE = Path("data/destructive/meta.json")


def main() -> None:
    force = "--force" in sys.argv

    if not PODS_FILE.exists():
        print("No pods.json found — nothing to tear down.")
        return

    pods = json.loads(PODS_FILE.read_text())
    if not pods:
        print("pods.json is empty — nothing to tear down.")
        return

    meta: dict = {}
    if META_FILE.exists():
        meta = json.loads(META_FILE.read_text())

    cost_per_pod = meta.get("hourly_cost_per_pod", 0)
    created_at = meta.get("created_at", "unknown")
    model = meta.get("model", "unknown")

    print(f"Pods: {len(pods)}")
    print(f"Model: {model}")
    print(f"Created: {created_at}")
    if cost_per_pod > 0:
        total_hourly = len(pods) * cost_per_pod
        print(f"Cost: ${total_hourly:.2f}/hr (${cost_per_pod}/pod)")

    print()
    for p in pods:
        print(f"  {p['name']}: {p['id']}")

    if not force:
        print(f"\nAbout to terminate {len(pods)} pods. This cannot be undone.")
        confirm = input("Type 'yes' to confirm: ")
        if confirm.strip().lower() != "yes":
            print("Aborted.")
            return

    print(f"\nTerminating {len(pods)} pods...")
    runpod.check_credentials()

    terminated = 0
    for p in pods:
        pid = p["id"]
        try:
            runpod.terminate_pod(pid)
            print(f"  {p['name']} ({pid}): terminated")
            terminated += 1
            audit(
                "pod.terminated",
                resource_kind="pod",
                resource_id=pid,
                detail={"name": p["name"], "model": model},
            )
        except Exception as e:
            print(f"  {p['name']} ({pid}): failed — {e}")

    for f in [PODS_FILE, ENDPOINTS_FILE, META_FILE]:
        if f.exists():
            f.unlink()

    print(f"\nDone. {terminated}/{len(pods)} pods terminated.")
    print("State files cleaned up.")
    print("Verify at https://www.runpod.io/console/pods")


if __name__ == "__main__":
    main()
