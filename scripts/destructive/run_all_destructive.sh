#!/bin/bash
set -euo pipefail

# Run destructive perturbations across 15 L4 pods.
# IMPORTANT: $6/hr total at $0.40/pod. SHUT DOWN PODS WHEN DONE.
#
# Usage:
#   1. Spin up 15 L4 pods on RunPod with userdata.sh
#   2. Fill in ENDPOINTS below with the pod URLs
#   3. Run this script
#   4. SHUT DOWN THE PODS

cd "$(git rev-parse --show-toplevel)"

ENDPOINTS=(
  # Fill these in after pod creation — 15 L4 endpoints
  # https://XXXXXX-8000.proxy.runpod.net/v1
  PLACEHOLDER
)

if [[ "${ENDPOINTS[0]}" == "PLACEHOLDER" ]]; then
  echo "ERROR: Fill in ENDPOINTS with RunPod URLs first" >&2
  exit 1
fi

# Evals with full item sets
FULL_EVALS=(arc cybermetric)

# Evals with 500-item subsets
LIMITED_EVALS=(mmlu hellaswag truthfulqa gsm8k)

echo "=== Starting destructive perturbation runs ==="
echo "=== ${#ENDPOINTS[@]} endpoints, $(date) ==="
echo ""
echo "!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!"
echo "!! REMEMBER TO SHUT DOWN PODS WHEN DONE    !!"
echo "!! 15 × \$0.40/hr = \$6.00/hr               !!"
echo "!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!"
echo ""

for eval in "${FULL_EVALS[@]}"; do
  echo "=== $eval (full) ===" >&2
  python3 scripts/destructive/run_destructive.py \
    "data/destructive/$eval/manifest.json" \
    "data/destructive/$eval/logs" \
    "${ENDPOINTS[@]}"
done

for eval in "${LIMITED_EVALS[@]}"; do
  echo "=== $eval (--limit 500) ===" >&2
  python3 scripts/destructive/run_destructive.py \
    --limit 500 \
    "data/destructive/$eval/manifest.json" \
    "data/destructive/$eval/logs" \
    "${ENDPOINTS[@]}"
done

echo ""
echo "=== All destructive runs complete ==="
echo ""
echo "!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!"
echo "!! NOW SHUT DOWN YOUR PODS                 !!"
echo "!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!"
