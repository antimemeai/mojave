#!/bin/bash
set -euo pipefail

ENDPOINTS=(
  https://03bypbibsxwqi5-8000.proxy.runpod.net/v1
  https://am1c33rfqyi8r0-8000.proxy.runpod.net/v1
  https://q2sjbqn65ap20q-8000.proxy.runpod.net/v1
  https://rlwexsv56n0556-8000.proxy.runpod.net/v1
  https://uvvzdoaoo4cxmb-8000.proxy.runpod.net/v1
  https://4dzb421b66ymsn-8000.proxy.runpod.net/v1
  https://jt5yg9c9qs9vvd-8000.proxy.runpod.net/v1
  https://mdiwb5f1bxftvf-8000.proxy.runpod.net/v1
  https://4oy898287mvm1l-8000.proxy.runpod.net/v1
  https://c3lx7ce2vqvpjg-8000.proxy.runpod.net/v1
  https://qao0c9f56cmr28-8000.proxy.runpod.net/v1
  https://w3vpod2grn2d67-8000.proxy.runpod.net/v1
)

EVALS=(cybermetric mmlu hellaswag truthfulqa)

for eval in "${EVALS[@]}"; do
  echo "=== Starting $eval ===" >&2
  python3 scripts/arc-workup/run_eval.py \
    "data/$eval/manifest.json" \
    "data/$eval/logs" \
    "${ENDPOINTS[@]}"
  echo "=== Finished $eval ===" >&2
done

echo "=== All evals complete ===" >&2
