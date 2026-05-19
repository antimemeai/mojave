#!/usr/bin/env bash
set -euo pipefail

# ARC Challenge Measurement Workup — Full Pipeline
#
# Prerequisites:
#   - vLLM server(s) running with Qwen2.5-7B-Instruct
#   - inspect-ai and inspect-evals installed
#   - mojave CLI built (cargo build --release -p mojave-cli)
#   - mojave-calibrate installed (cd python && uv sync)
#
# Usage:
#   ./scripts/arc-workup/run_analysis.sh [VLLM_BASE_URL]

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
DATA_DIR="${SCRIPT_DIR}/../../data/arc-workup"
MOJAVE="cargo run --release -p mojave-cli --"
CALIBRATE="uv run --project python mojave-calibrate"
VLLM_URL="${1:-http://localhost:8000/v1}"

echo "=== Phase 1: Generate variant specs ==="
python3 "${SCRIPT_DIR}/variant_spec.py" "${DATA_DIR}/manifest.json"

echo "=== Phase 2: Run Inspect evaluations ==="
python3 "${SCRIPT_DIR}/run_inspect.py" \
    "${DATA_DIR}/manifest.json" \
    "${DATA_DIR}/logs" \
    "openai/Qwen2.5-7B-Instruct"

echo "=== Phase 3: Ingest with mojave ==="
${MOJAVE} ingest "${DATA_DIR}/logs"/*/*.eval \
    --format inspect \
    > "${DATA_DIR}/trial_records.json"

echo "=== Phase 4: Validate ingest ==="
python3 "${SCRIPT_DIR}/validate.py" \
    "${DATA_DIR}/manifest.json" \
    "${DATA_DIR}/trial_records.json"

echo "=== Phase 5: Run mojave analyze (full battery) ==="
${MOJAVE} analyze "${DATA_DIR}/logs"/*/*.eval \
    > "${DATA_DIR}/analysis_report.json"

echo "=== Phase 6: Prepare IRT input ==="
python3 "${SCRIPT_DIR}/prepare_irt.py" \
    "${DATA_DIR}/trial_records.json" \
    "${DATA_DIR}/irt_input.jsonl"

echo "=== Phase 7: Run IRT calibration (1PL Rasch — primary) ==="
${CALIBRATE} irt \
    --input "${DATA_DIR}/irt_input.jsonl" \
    --output "${DATA_DIR}/item_pool_1pl.json" \
    --model-type 1pl \
    --content-domain science \
    --device cpu

echo "=== Phase 8: Run IRT calibration (2PL — exploratory) ==="
${CALIBRATE} irt \
    --input "${DATA_DIR}/irt_input.jsonl" \
    --output "${DATA_DIR}/item_pool_2pl.json" \
    --model-type 2pl \
    --content-domain science \
    --device cpu

echo "=== Complete ==="
echo "Artifacts in ${DATA_DIR}:"
ls -la "${DATA_DIR}"/*.json "${DATA_DIR}"/*.jsonl 2>/dev/null || true
