# RunPod Eval Pod Runbook

How to spin up cheap GPU pods for running Inspect evals with vLLM.
Written after burning through multiple rounds of pod creation getting the config right.

## TL;DR: The Right Way (2026-05-19)

Use the `vllm/vllm-openai` Docker image. Pass model config as `docker_args`.
No SSH. No manual setup. Pods come up serving.

```python
runpod.create_pod(
    name=f"mojave-{i:02d}",
    image_name="vllm/vllm-openai:latest",
    gpu_type_id="NVIDIA GeForce RTX 3090",  # or whatever's available
    gpu_count=1,
    volume_in_gb=50,
    container_disk_in_gb=50,
    ports="8000/http",
    cloud_type="ALL",
    docker_args=(
        "--model Qwen/Qwen2.5-7B-Instruct "
        "--max-model-len 4096 "
        "--enforce-eager "
        "--gpu-memory-utilization 0.9 "
        "--tensor-parallel-size 1 "
        "--host 0.0.0.0 "
        "--port 8000"
    ),
    env={"VLLM_USE_V1": "0"},
)
```

### Why this works

- `vllm/vllm-openai` image has vLLM + PyTorch + CUDA pre-installed. The
  entrypoint starts the OpenAI-compatible server automatically with whatever
  flags you pass via `docker_args`.
- No SSH needed = no PTY issues, no timeout failures, no manual setup.
- Model downloads on first boot from HuggingFace automatically.
- Pod is ready to serve when `GET /v1/models` returns 200.

### Critical flags

- **`--enforce-eager`**: Disables CUDA graphs. Required for consumer GPUs
  (RTX 3090, 4090). Without this, vLLM's V1 engine crashes with
  "Engine core initialization failed" on compute capability <8.9 GPUs.
- **`VLLM_USE_V1=0`**: Falls back to vLLM's legacy engine. Belt-and-suspenders
  with `--enforce-eager`. The V1 engine has known issues on consumer GPUs.
- **`--gpu-memory-utilization 0.9`**: Leave 10% headroom. Going higher risks
  OOM on GPUs with less VRAM.

## GPU availability & pricing (as of 2026-05-19)

GPU availability on RunPod is **spotty and changes hourly**. Don't assume
any type is available. Probe first:

```python
try:
    pod = runpod.create_pod(name="probe", **config)
    runpod.terminate_pod(pod["id"])
    print("available")
except:
    print("no capacity")
```

| GPU | VRAM | $/hr (community) | Notes |
|-----|------|-------------------|-------|
| L4 | 24GB | ~$0.39 | Best value. Often sold out. |
| RTX 3090 | 24GB | ~$0.22 | Good availability. Needs --enforce-eager. |
| RTX 4090 | 24GB | ~$0.49 | Sometimes available. Needs --enforce-eager. |
| A4000 | 16GB | ~$0.19 | Tight for 7B models. Use --max-model-len 2048. |
| A6000 | 48GB | ~$0.59 | Overkill for 7B, good for 13B+. |

**Strategy**: Try L4 first. Fall back to 3090. Grab whatever's available
in small batches — don't try to create 15 at once if capacity is limited.

## Pod creation parameters explained

### Why 50GB container disk

Qwen2.5-7B-Instruct is ~15GB. vLLM + torch + CUDA deps eat another ~15GB.
The HuggingFace download writes to a tmp dir before moving, so you need
headroom for the download + final copy. 20GB **will** fail with
`No space left on device` during model download. 50GB is safe.

The "Background writer channel closed" errors in HF logs are a red herring —
that's huggingface_hub's xet downloader dying because the disk filled up.

### Why cloud_type="ALL"

`SECURE` cloud sometimes runs out after 2-3 pods and throws
`This machine does not have the resources to deploy your pod`. `ALL` lets
RunPod place pods across secure and community datacenters.

### Why batch creation (3 at a time)

Creating 15 pods at once can exhaust a datacenter's capacity mid-batch,
leaving you with partial failures and orphan pods. Create 3 at a time,
verify each batch, continue.

## What NOT to do

### Don't use a base PyTorch image + SSH setup

The old approach was:
1. Create pod with `runpod/pytorch:2.4.0-py3.11-cuda12.4.1-devel-ubuntu22.04`
2. SSH in after creation
3. `pip install vllm`, download model, start server manually

This fails for multiple reasons:
- SSH setup commands via `docker_args` break GraphQL (special characters)
- SSH connections timeout on slow model downloads (600s not enough)
- PTY issues with RunPod's SSH proxy
- No way to recover if SSH fails — have to terminate and recreate
- Manual setup = manual errors at scale

### Don't skip --enforce-eager on consumer GPUs

vLLM's CUDA graph compilation crashes on RTX 3090/4090 (compute capability
8.6) with a cryptic "Engine core initialization failed" error. The error
message gives no useful root cause. We burned 19 pods discovering this.

### Don't forget to terminate pods

**15 × $0.39/hr = $5.85/hr. 15 × $0.22/hr = $3.30/hr.**

```bash
python3 scripts/destructive/teardown_pods.py
```

Check https://www.runpod.io/console/pods after teardown to verify.

## Verify endpoint is live

```bash
curl -s https://{pod_id}-8000.proxy.runpod.net/v1/models | python3 -m json.tool
```

Returns:
```json
{"object": "list", "data": [{"id": "Qwen/Qwen2.5-7B-Instruct", ...}]}
```

## Running evals

```bash
python3 scripts/destructive/run_destructive.py \
  data/destructive/arc/manifest.json \
  data/destructive/arc/logs \
  $(python3 -c "import json; [print(e) for e in json.load(open('data/destructive/endpoints.json'))]")
```

Or use `run_all_destructive.sh` which reads from `endpoints.json`.

## Teardown

```bash
python3 scripts/destructive/teardown_pods.py
```

Or manually:
```python
import runpod, json
pods = json.load(open("data/destructive/pods.json"))
for p in pods:
    runpod.terminate_pod(p["id"])
```

## Credentials

RunPod API key stored in `~/.runpod/` (managed by `runpod` Python SDK).
`runpod.check_credentials()` finds it automatically.

## Scripts reference

| Script | Purpose |
|--------|---------|
| `create_pods.py` | Create pods in batches, poll until serving |
| `setup_pods.py` | (DEPRECATED) SSH-based setup. Use vllm image instead. |
| `teardown_pods.py` | Terminate all pods from pods.json |
| `run_destructive.py` | Run eval variants across endpoints |
| `run_all_destructive.sh` | Orchestrate all 6 evals end-to-end |
| `gen_destructive_manifest.py` | Generate 106-variant manifest per eval |
| `destructive_task.py` | Inspect task wrapper for perturbation params |
