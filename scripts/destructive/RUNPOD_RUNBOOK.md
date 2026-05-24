# RunPod Eval Pod Runbook

How to spin up cheap GPU pods for running Inspect evals with vLLM.
Written after burning through multiple rounds of pod creation getting the config right.

## TL;DR: The Right Way (2026-05-24)

Use deployment profiles. No hardcoded models or GPUs.

```bash
# 1. Create pods from a profile
python3 scripts/destructive/create_pods.py scripts/destructive/profiles/qwen-7b-3090.yaml

# 2. Generate manifest for the eval you want
python3 scripts/destructive/gen_destructive_manifest.py inspect_evals/wmdp_chem \
    data/destructive/wmdp_chem/manifest.json

# 3. Run it (endpoints auto-discovered from endpoints.json)
python3 scripts/destructive/run_destructive.py \
    data/destructive/wmdp_chem/manifest.json \
    data/destructive/wmdp_chem/logs

# 4. Tear down (shows cost, requires confirmation)
python3 scripts/destructive/teardown_pods.py
```

For 72B on H100s:
```bash
python3 scripts/destructive/create_pods.py scripts/destructive/profiles/qwen-72b-h100.yaml
python3 scripts/destructive/gen_destructive_manifest.py inspect_evals/wmdp_chem \
    data/destructive/wmdp_chem/manifest.json --model Qwen/Qwen2.5-72B-Instruct
```

## Deployment Profiles

Profiles live in `scripts/destructive/profiles/`. Each YAML file specifies
model, GPU type, pod count, cost, and vLLM flags. Create new profiles for
new model/GPU combinations — don't edit `create_pods.py`.

| Profile | Model | GPU | Pods | $/hr |
|---------|-------|-----|------|------|
| `qwen-7b-3090.yaml` | Qwen 2.5-7B-Instruct | RTX 3090 x1 | 8 | $0.22/pod |
| `qwen-72b-h100.yaml` | Qwen 2.5-72B-Instruct | H100 80GB x4 | 4 | $3.89/pod |

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
# Endpoints are auto-discovered from data/destructive/endpoints.json
python3 scripts/destructive/run_destructive.py \
    data/destructive/wmdp_chem/manifest.json \
    data/destructive/wmdp_chem/logs

# With options:
python3 scripts/destructive/run_destructive.py \
    data/destructive/wmdp_chem/manifest.json \
    data/destructive/wmdp_chem/logs \
    --limit 50 --timeout 1800 --retries 2
```

Features:
- **Resume**: skips variants with existing `.eval` files
- **Timeout**: kills hung variants after `--timeout` seconds (default 1800)
- **Retry**: retries failed variants up to `--retries` times (default 2)
- **Health check**: pings all endpoints before starting, drops unhealthy ones

## Teardown

```bash
python3 scripts/destructive/teardown_pods.py
```

Shows pod count, model, cost estimate, and creation time. Requires typing
`yes` to confirm. Use `--force` to skip confirmation (for scripted pipelines).
Cleans up `pods.json`, `endpoints.json`, and `meta.json` after teardown.

## Credentials

RunPod API key stored in `~/.runpod/` (managed by `runpod` Python SDK).
`runpod.check_credentials()` finds it automatically.

## Scripts reference

| Script | Purpose |
|--------|---------|
| `create_pods.py` | Create pods from a profile, poll until serving |
| `teardown_pods.py` | Terminate all pods (with confirmation and cost estimate) |
| `run_destructive.py` | Run eval variants across endpoints with timeout/retry |
| `gen_destructive_manifest.py` | Generate variant manifest for a single eval task |
| `destructive_task.py` | Inspect task wrapper for perturbation params |
