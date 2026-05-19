#!/bin/bash
set -euxo pipefail
export DEBIAN_FRONTEND=noninteractive
apt-get update -qq
apt-get install -y -qq python3-pip python3-venv
python3 -m venv /opt/vllm
source /opt/vllm/bin/activate
pip install -q --upgrade pip
pip install -q "vllm<1.0"
python3 -c "from huggingface_hub import snapshot_download; snapshot_download('Qwen/Qwen2.5-7B-Instruct')"
nohup python3 -m vllm.entrypoints.openai.api_server --model Qwen/Qwen2.5-7B-Instruct --max-model-len 4096 --gpu-memory-utilization 0.9 --host 0.0.0.0 --port 8000 > /var/log/vllm.log 2>&1 &
echo "DONE - vLLM launching, check in 60s: curl localhost:8000/v1/models"
