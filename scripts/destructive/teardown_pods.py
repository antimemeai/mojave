#!/usr/bin/env python3
"""Terminate all destructive perturbation pods.

RUN THIS WHEN THE EVALS ARE DONE. 15 x $0.40/hr = $6.00/hr.
"""

from __future__ import annotations

import json
from pathlib import Path

import runpod


def main() -> None:
    pods_file = Path("data/destructive/pods.json")
    if not pods_file.exists():
        print("No pods.json found — nothing to tear down.")
        return

    pods = json.loads(pods_file.read_text())
    print(f"Terminating {len(pods)} pods...")

    for p in pods:
        pid = p["id"]
        try:
            runpod.terminate_pod(pid)
            print(f"  {pid}: terminated")
        except Exception as e:
            print(f"  {pid}: failed — {e}")

    print(f"\nDone. {len(pods)} pods terminated.")
    print("Verify at https://www.runpod.io/console/pods")


if __name__ == "__main__":
    main()
