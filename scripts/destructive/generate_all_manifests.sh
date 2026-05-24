#!/bin/bash
set -euo pipefail

# Generate destructive perturbation manifests for all 6 evals
cd "$(git rev-parse --show-toplevel)"

declare -A TASKS=(
  [arc]="inspect_evals/arc_challenge"
  [cybermetric]="inspect_evals/cybermetric_2000"
  [mmlu]="inspect_evals/mmlu_0_shot"
  [hellaswag]="inspect_evals/hellaswag"
  [truthfulqa]="inspect_evals/truthfulqa"
  [gsm8k]="inspect_evals/gsm8k"
)

for eval in "${!TASKS[@]}"; do
  python3 scripts/destructive/gen_destructive_manifest.py \
    "${TASKS[$eval]}" \
    "data/destructive/$eval/manifest.json"
done

echo ""
echo "=== All manifests generated ==="
total=$(python3 -c "
import json, glob
n = sum(json.load(open(f))['total_variants'] for f in glob.glob('data/destructive/*/manifest.json'))
print(f'{n} total variants across 6 evals')
")
echo "$total"
