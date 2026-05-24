#!/usr/bin/env bash
# Run all Tier 1 MCQ bf16 tranches in sequence on the current fleet.
# Assumes bf16 endpoints are live in data/destructive/endpoints.json.
# Each run is idempotent — safe to re-run if interrupted.
set -euo pipefail

echo "=== Tier 1 bf16 tranches ==="
echo "Fleet: $(python3 -c "import json; e=json.load(open('data/destructive/endpoints.json')); print(sum(len(v) for v in e.values()) if isinstance(e,dict) else len(e))" ) endpoints"
echo ""

# 1. WMDP-Bio (may already be done or in progress)
echo "--- [1/3] WMDP-Bio bf16 ---"
python scripts/v2/run_mcq.py \
  data/v2/manifest_bio_512.json \
  data/v2/logs_bio \
  --subset-file data/v2/wmdp_bio/subset_00.json \
  2>&1

echo ""
echo "--- [2/3] WMDP-Chem bf16 ---"
python scripts/v2/run_mcq.py \
  data/v2/manifest_chem_512.json \
  data/v2/logs_chem \
  --subset-file data/v2/wmdp_chem/subset_00.json \
  2>&1

echo ""
echo "--- [3/3] TruthfulQA MC1 bf16 ---"
python scripts/v2/run_mcq.py \
  data/v2/manifest_truthfulqa_512.json \
  data/v2/logs_truthfulqa \
  --subset-file data/v2/truthfulqa_mc1/subset_00.json \
  2>&1

echo ""
echo "=== All Tier 1 bf16 tranches complete ==="
echo "Next: swap fleet to fp8, then run scripts/v2/run_tier1_fp8.sh"
