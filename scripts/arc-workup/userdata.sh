#!/bin/bash
set -euxo pipefail

exec > /var/log/arc-workup-setup.log 2>&1

export DEBIAN_FRONTEND=noninteractive

# Install pip and venv
apt-get update -qq
apt-get install -y -qq python3-pip python3-venv

# Create virtualenv
python3 -m venv /opt/arc-workup
source /opt/arc-workup/bin/activate

# Install vLLM (includes PyTorch + CUDA)
pip install --upgrade pip
pip install vllm

# Install Inspect AI + evals
pip install inspect-ai inspect-evals

# Download model ahead of serving
python3 -c "from huggingface_hub import snapshot_download; snapshot_download('Qwen/Qwen2.5-7B-Instruct')"

# Start vLLM as OpenAI-compatible server
nohup python3 -m vllm.entrypoints.openai.api_server \
  --model Qwen/Qwen2.5-7B-Instruct \
  --max-model-len 4096 \
  --gpu-memory-utilization 0.9 \
  --tensor-parallel-size 1 \
  --host 0.0.0.0 \
  --port 8000 \
  > /var/log/vllm-server.log 2>&1 &

echo "SETUP COMPLETE $(date)" > /var/log/arc-workup-ready
