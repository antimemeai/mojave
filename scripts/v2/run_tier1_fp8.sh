#!/usr/bin/env bash
# Run all Tier 1 MCQ fp8 tranches in sequence.
# Assumes fp8 endpoints are live in data/destructive/endpoints.json.
set -euo pipefail

echo "=== Tier 1 fp8 tranches ==="
echo "Fleet: $(python3 -c "import json; e=json.load(open('data/destructive/endpoints.json')); print(sum(len(v) for v in e.values()) if isinstance(e,dict) else len(e))" ) endpoints"
echo ""

echo "--- [1/3] WMDP-Bio fp8 ---"
python scripts/v2/run_mcq.py \
  data/v2/manifest_bio_512.json \
  data/v2/logs_bio \
  --subset-file data/v2/wmdp_bio/subset_00.json \
  2>&1

echo ""
echo "--- [2/3] WMDP-Chem fp8 ---"
python scripts/v2/run_mcq.py \
  data/v2/manifest_chem_512.json \
  data/v2/logs_chem \
  --subset-file data/v2/wmdp_chem/subset_00.json \
  2>&1

echo ""
echo "--- [3/3] TruthfulQA MC1 fp8 ---"
python scripts/v2/run_mcq.py \
  data/v2/manifest_truthfulqa_512.json \
  data/v2/logs_truthfulqa \
  --subset-file data/v2/truthfulqa_mc1/subset_00.json \
  2>&1

echo ""
echo "=== All Tier 1 fp8 tranches complete ==="
echo "REMEMBER: Terminate the fleet! Cost is adding up."
