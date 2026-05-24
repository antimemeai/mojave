#!/usr/bin/env python3
"""Generate populated runcard-config.tex files from analysis data.

Reads data/analysis/full_summary.json and creates one run card directory
per eval under data/run-cards/<eval-name>/, each with:
  - runcard-config.tex  (populated with real numbers)
  - symlinks to the engine files (runcard.tex, Makefile)

Re-run after adding new evals (e.g. GSM8K) to regenerate.

Usage:
    python scripts/arc-workup/generate_run_cards.py
"""

from __future__ import annotations

import hashlib
import json
import subprocess
from pathlib import Path

SUMMARY_PATH = Path("data/analysis/full_summary.json")
OUTPUT_BASE = Path("data/run-cards")
TEMPLATE_DIR = Path("templates/run-card/single-run-card")
CROSS_EVAL_TEMPLATE_DIR = Path("templates/run-card/cross-eval-summary")
CROSS_EVAL_OUTPUT = OUTPUT_BASE / "cross-eval-summary"

EVAL_META = {
    "arc_challenge": {
        "display_name": "ARC Challenge",
        "source": r"allenai/ai2\_arc (challenge split)",
        "id_suffix": "ARC",
    },
    "cybermetric_2000": {
        "display_name": "CyberMetric-2000",
        "source": r"cybermetric/CyberMetric (2000-question set)",
        "id_suffix": "CYBER",
    },
    "mmlu_0_shot": {
        "display_name": "MMLU (0-shot)",
        "source": r"cais/mmlu (0-shot, 500-item subset)",
        "id_suffix": "MMLU",
    },
    "hellaswag": {
        "display_name": "HellaSwag",
        "source": r"Rowan/hellaswag (500-item subset)",
        "id_suffix": "HSWAG",
    },
    "truthfulqa": {
        "display_name": "TruthfulQA",
        "source": r"truthfulqa/truthful\_qa (MC2, 500-item subset)",
        "id_suffix": "TFQA",
    },
    "gsm8k": {
        "display_name": "GSM8K",
        "source": r"openai/gsm8k (500-item pseudo-random subset, seed 20260519)",
        "id_suffix": "GSM8K",
    },
    "wmdp_chem": {
        "display_name": "WMDP-Chem",
        "source": r"cais/wmdp (chemistry split, 5-item smoketest subset)",
        "id_suffix": "WMDP",
    },
}


def compute_file_sha256(path: Path) -> str:
    """Compute full SHA-256 hex digest of a file."""
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(8192), b""):
            h.update(chunk)
    return h.hexdigest()


def audit_seal(run_id: str, eval_name: str, data_file: Path) -> dict | None:
    """Call mojave audit seal and return the output, or None if mojave is not available."""
    data_sha256 = compute_file_sha256(data_file)
    seal_input = {
        "run_id": run_id,
        "eval_name": eval_name,
        "date_issued": "2026-05-19",
        "data_file": str(data_file),
        "data_sha256": data_sha256,
        "actor": {"kind": "System", "id": "generate_run_cards.py"},
    }
    try:
        result = subprocess.run(
            ["mojave", "audit", "seal"],
            input=json.dumps(seal_input),
            capture_output=True,
            text=True,
            timeout=30,
        )
        if result.returncode != 0:
            print(f"  WARN: mojave audit seal failed: {result.stderr.strip()}")
            return None
        return json.loads(result.stdout)
    except FileNotFoundError:
        print("  WARN: mojave binary not found, skipping audit seal")
        return None
    except subprocess.TimeoutExpired:
        print("  WARN: mojave audit seal timed out")
        return None


def fmt(v: float, decimals: int = 4) -> str:
    return f"{v:.{decimals}f}"


def generate_config(name: str, data: dict) -> str:
    meta = EVAL_META[name]
    agg = data["aggregate"]
    stab = data["perturbation_stability"]
    seq = data.get("sequential_stopping") or {}

    run_id = f"MOJAVE-2026-0519-{meta['id_suffix']}"
    wilson_width = agg["wilson_ci_95"][1] - agg["wilson_ci_95"][0]

    hist_str = ",".join(str(x) for x in stab["stability_histogram"])

    temps = sorted(data["by_temperature"].keys(), key=float)
    temp_list = ", ".join(temps)

    baseline_acc = None
    for t in data["by_temperature"].values():
        if t["n_variants"] == 1:
            baseline_acc = t["mean_accuracy"]
            break

    lines = []
    lines.append(r"% " + "=" * 76)
    lines.append(f"% MOJAVE RUN CARD — {meta['display_name']}")
    lines.append("% Auto-generated from data/analysis/full_summary.json")
    lines.append(r"% " + "=" * 76)
    lines.append("")

    lines.append(r"% ---------------------------------------------------------------- HEADER ----")
    lines.append(rf"\rcset{{run.id}}            {{{run_id}}}")
    lines.append(r"\rcset{date.issued}       {2026-05-19}")
    lines.append(rf"\rcset{{benchmark.name}}    {{{meta['display_name']}}}")
    lines.append(r"\rcset{benchmark.version} {UK AISI Inspect Evals (2026-05)}")
    lines.append(rf"\rcset{{benchmark.source}}  {{\texttt{{{meta['source']}}}}}")
    lines.append(r"\rcset{model.name}        {Qwen/Qwen2.5-7B-Instruct}")
    lines.append(r"\rcset{model.revision}    {\texttt{HuggingFace (default revision)}}")
    lines.append(r"\rcset{model.quant}       {bf16 (no quantization)}")
    lines.append(r"\rcset{model.serving}     {vLLM 0.8.5.post1, L4 24GB, max\_tokens=4096}")
    lines.append(r"\rcset{evaluator.org}     {antimeme.ai}")
    lines.append(r"\rcset{evaluator.tool}    {UK AISI Inspect AI}")
    lines.append("")

    lines.append(r"% ---------------------------------------------------------------- DESIGN ----")
    lines.append(rf"\rcset{{n.items}}           {{{data['n_items']}}}")
    lines.append(rf"\rcset{{n.variants}}        {{{data['n_variants']}}}")
    lines.append(
        rf"\rcset{{perturb.design}}    {{Destructive perturbation workup: "
        rf"5 prompt templates $\times$ 5 system prompts $\times$ "
        rf"4 few-shot levels $\times$ 5 label formats $\times$ "
        rf"4 temperatures ({temp_list}) + 1 deterministic baseline ($T{{=}}0$). "
        rf"{data['n_variants']} variants per item.}}"
    )
    lines.append(
        rf"\rcset{{inference.config}}  {{Per-variant: temperature $\in\{{{temp_list}\}}$; "
        rf"prompt template $\in$ 5 styles; system prompt $\in$ 5 styles; "
        rf"few-shot $\in \{{0,1,3,5\}}$; label format $\in$ 5 styles; "
        rf"top\_p $=1.0$; greedy decode at $T{{=}}0$.}}"
    )
    lines.append("")

    lines.append(r"% ------------------------------------------------------ AGGREGATE RESULTS ----")
    lines.append(rf"\rcset{{acc.overall}}       {{{fmt(agg['pooled_accuracy'])}}}")
    lines.append(rf"\rcset{{wilson.lower}}      {{{fmt(agg['wilson_ci_95'][0])}}}")
    lines.append(rf"\rcset{{wilson.upper}}      {{{fmt(agg['wilson_ci_95'][1])}}}")
    lines.append(rf"\rcset{{wilson.width}}      {{{fmt(wilson_width)}}}")
    if baseline_acc is not None:
        lines.append(
            rf"\rcset{{acc.baseline}}      {{{fmt(baseline_acc)} "
            rf"\textnormal{{(deterministic baseline: }}$T{{=}}0$\textnormal{{)}}}}"
        )
    else:
        lines.append(r"\rcset{acc.baseline}      {}")
    lines.append("")

    lines.append(r"% ----------------------------------------------- INSTRUMENT QUALITY (IRT) ----")
    lines.append(r"% IRT analysis not performed for this run. All fields left empty -> em-dash.")
    lines.append(r"\rcset{irt.model}         {}")
    lines.append(r"\rcset{irt.diff.mean}     {}")
    lines.append(r"\rcset{irt.diff.sd}       {}")
    lines.append(r"\rcset{irt.diff.min}      {}")
    lines.append(r"\rcset{irt.diff.max}      {}")
    lines.append(r"\rcset{irt.diff.hist}     {}")
    lines.append(r"\rcset{irt.disc.mean}     {}")
    lines.append(r"\rcset{irt.disc.sd}       {}")
    lines.append(r"\rcset{irt.disc.min}      {}")
    lines.append(r"\rcset{irt.disc.max}      {}")
    lines.append(r"\rcset{irt.disc.hist}     {}")
    lines.append(r"\rcset{irt.disc.caveat}   {}")
    lines.append(rf"\rcset{{irt.floor.items}}   {{{stab['n_floor']}}}")
    lines.append(rf"\rcset{{irt.ceil.items}}    {{{stab['n_ceiling']}}}")
    lines.append(r"\rcset{irt.nearzero.disc} {}")
    lines.append(r"\rcset{q3.summary}        {}")
    lines.append(r"\rcset{q3.count.gt2}      {}")
    lines.append(r"\rcset{q3.note}           {}")
    lines.append("")

    lines.append(r"% -------------------------------------------- PERTURBATION SENSITIVITY ----")
    lines.append(rf"\rcset{{stab.hist}}         {{{hist_str}}}")
    lines.append(rf"\rcset{{stab.stable}}       {{{stab['n_stable']}}}")
    lines.append(rf"\rcset{{stab.sensitive}}    {{{stab['n_sensitive']}}}")
    lines.append(r"\rcset{dim.order}         {}")
    lines.append(rf"\rcset{{dim.temp}}          {{{stab['n_temp_sensitive']}}}")
    lines.append(r"\rcset{dim.prompt}        {}")
    lines.append(r"\rcset{dim.mixed}         {}")
    lines.append(r"\rcset{spearman.diff.rho} {\mbox{--}}")
    lines.append(r"\rcset{spearman.diff.lo}  {}")
    lines.append(r"\rcset{spearman.diff.hi}  {}")
    lines.append(r"\rcset{spearman.disc.rho} {\mbox{--}}")
    lines.append(r"\rcset{spearman.disc.lo}  {}")
    lines.append(r"\rcset{spearman.disc.hi}  {}")
    lines.append("")

    lines.append(r"% ----------------------------------- SEQUENTIAL STOPPING (RETROSPECTIVE) ----")
    if seq:
        lines.append(r"\rcset{sprt.method}       {Bernoulli mSPRT}")
        lines.append(
            r"\rcset{sprt.beta}         {Beta mixing prior $\mathrm{Beta}(\alpha{=}1,\beta{=}1)$}"
        )
        lines.append(rf"\rcset{{sprt.alpha}}        {{{seq['alpha']}}}")
        lines.append(rf"\rcset{{sprt.nperm}}        {{{seq['n_permutations']}}}")
        lines.append(rf"\rcset{{sprt.median.stop}}  {{{seq['median_stop']}}}")
        lines.append(rf"\rcset{{sprt.iqr.lo}}       {{{seq['iqr'][0]}}}")
        lines.append(rf"\rcset{{sprt.iqr.hi}}       {{{seq['iqr'][1]}}}")
        lines.append(rf"\rcset{{sprt.frac.q4}}      {{{seq['frac_stopped_by_quarter']}}}")
        lines.append(rf"\rcset{{sprt.frac.half}}    {{{seq['frac_stopped_by_half']}}}")
        lines.append(rf"\rcset{{sprt.frac.full}}    {{{seq['frac_stopped_by_end']}}}")
    else:
        for key in [
            "sprt.method",
            "sprt.beta",
            "sprt.alpha",
            "sprt.nperm",
            "sprt.median.stop",
            "sprt.iqr.lo",
            "sprt.iqr.hi",
            "sprt.frac.q4",
            "sprt.frac.half",
            "sprt.frac.full",
        ]:
            lines.append(rf"\rcset{{{key}}}        {{}}")
    lines.append("")

    lines.append(r"% ----------------------------------------------- RAW DATA REFERENCE ----")
    name_escaped = name.replace("_", r"\_")
    lines.append(
        rf"\rcset{{companion.file}}    {{\texttt{{data/analysis/{name_escaped}\_analysis.json}}}}"
    )

    data_file = Path(f"data/analysis/{name}_analysis.json")
    if data_file.exists():
        data_hash = compute_file_sha256(data_file)
    else:
        data_json = json.dumps(data, sort_keys=True)
        data_hash = hashlib.sha256(data_json.encode()).hexdigest()
    mid = len(data_hash) // 2
    h1, h2 = data_hash[:mid], data_hash[mid:]
    lines.append(rf"\rcset{{companion.hash}}    {{\texttt{{sha256:{h1}\allowbreak {h2}}}}}")
    lines.append(
        r"\rcset{companion.contents}{Per-variant accuracy, per-item response matrix, "
        r"perturbation stability analysis, sequential stopping results, "
        r"and the variant manifest.}"
    )
    lines.append("")

    lines.append(r"% ---------------------------------------------------------- AUDIT TRAIL ----")
    lines.append(r"\rcset{audit.chain.tip}   {}")
    lines.append(r"\rcset{audit.chain.seq}   {}")
    lines.append(r"\rcset{audit.signed}      {}")
    lines.append("")

    lines.append(r"% ---------------------------------------------------------- DISCLAIMERS ----")
    lines.append(
        r"\rcset{disc.2pl}{"
        r"\textbf{IRT not fitted.} Item Response Theory parameters were not estimated "
        r"for this run. Floor/ceiling counts are from raw response fractions only.}"
    )
    lines.append(
        r"\rcset{disc.unidim}{\textbf{Single model only.} This run card reports "
        r"perturbation stability for one model (Qwen/Qwen2.5-7B-Instruct). "
        r"Item behavior may differ substantially across models.}"
    )

    return "\n".join(lines) + "\n"


EVAL_ORDER = [
    "arc_challenge",
    "cybermetric_2000",
    "hellaswag",
    "mmlu_0_shot",
    "truthfulqa",
    "gsm8k",
    "wmdp_chem",
]


def generate_cross_eval_config(summary: dict) -> str:
    evals = [(name, summary[name]) for name in EVAL_ORDER if name in summary]
    total_obs = sum(d["aggregate"]["pooled_n"] for _, d in evals)

    accs = [(EVAL_META[n]["display_name"], d["aggregate"]["pooled_accuracy"]) for n, d in evals]
    accs_sorted = sorted(accs, key=lambda x: x[1], reverse=True)
    spreads = [(EVAL_META[n]["display_name"], d["aggregate"]["variant_spread"]) for n, d in evals]
    spreads_sorted = sorted(spreads, key=lambda x: x[1])

    lines = []
    lines.append(r"% " + "=" * 76)
    lines.append("% CROSS-EVAL METROLOGICAL SUMMARY")
    lines.append("% Auto-generated from data/analysis/full_summary.json")
    lines.append(r"% " + "=" * 76)
    lines.append("")

    lines.append(r"% ---------------------------------------------------------------- HEADER ----")
    lines.append(r"\rcset{summary.id}        {MOJAVE-XEVAL-2026-0519}")
    lines.append(r"\rcset{date.issued}       {2026-05-19}")
    lines.append(r"\rcset{model.short}       {Qwen2.5-7B}")
    lines.append(r"\rcset{model.name}        {Qwen/Qwen2.5-7B-Instruct}")
    lines.append(r"\rcset{model.revision}    {\texttt{HuggingFace (default revision)}}")
    lines.append(r"\rcset{model.quant}       {bf16 (no quantization)}")
    lines.append(r"\rcset{model.serving}     {vLLM 0.8.5.post1, L4 24GB, max\_tokens=4096}")
    lines.append(rf"\rcset{{n.evals}}          {{{len(evals)}}}")
    lines.append(
        r"\rcset{eval.selection}    {5 benchmarks from UK AISI Inspect Evals: "
        r"ARC Challenge, CyberMetric-2000, HellaSwag, MMLU (0-shot), "
        r"TruthfulQA. Selected to span accuracy levels and "
        r"metrological quality tiers. GSM8K excluded (run failed).}"
    )
    lines.append(r"\rcset{evaluator.org}     {antimeme.ai}")
    lines.append(r"\rcset{evaluator.tool}    {UK AISI Inspect AI}")
    lines.append("")

    lines.append(r"% -------------------------------------------------- PERTURBATION DESIGN ----")
    lines.append(
        r"\rcset{perturb.design}    {Destructive perturbation workup: "
        r"5 prompt templates $\times$ 5 system prompts $\times$ "
        r"4 few-shot levels $\times$ 5 label formats $\times$ "
        r"4 temperatures (0.3, 0.7, 1.0) + 1 deterministic "
        r"baseline ($T{=}0$). 106 variants per item per eval.}"
    )
    lines.append(
        r"\rcset{inference.config}  {Per-variant: temperature "
        r"$\in\{0.0, 0.3, 0.7, 1.0\}$; prompt template $\in$ 5 styles; "
        r"system prompt $\in$ 5 styles; few-shot $\in \{0,1,3,5\}$; "
        r"label format $\in$ 5 styles; top\_p $=1.0$; greedy at $T{=}0$.}"
    )
    obs_str = f"{total_obs:,}".replace(",", "{,}")
    lines.append(rf"\rcset{{n.total.obs}}      {{{obs_str}}}")
    lines.append("")

    lines.append(
        r"% ----------------------------------------------------- CROSS-EVAL ACCURACY ----"
    )
    lines.append(rf"\rcset{{acc.best.name}}     {{{accs_sorted[0][0]}}}")
    lines.append(rf"\rcset{{acc.best.val}}      {{{fmt(accs_sorted[0][1])}}}")
    lines.append(rf"\rcset{{acc.worst.name}}    {{{accs_sorted[-1][0]}}}")
    lines.append(rf"\rcset{{acc.worst.val}}     {{{fmt(accs_sorted[-1][1])}}}")
    lines.append(rf"\rcset{{spread.best.name}}  {{{spreads_sorted[0][0]}}}")
    lines.append(rf"\rcset{{spread.best.val}}   {{{fmt(spreads_sorted[0][1], 3)}pp}}")
    lines.append(rf"\rcset{{spread.worst.name}} {{{spreads_sorted[-1][0]}}}")
    lines.append(rf"\rcset{{spread.worst.val}}  {{{fmt(spreads_sorted[-1][1], 3)}pp}}")
    lines.append("")

    lines.append(r"% --------------------------------------------- METROLOGICAL QUALITY ----")
    for i, (name, data) in enumerate(evals, 1):
        meta = EVAL_META[name]
        stab = data["perturbation_stability"]
        hist_str = ",".join(str(x) for x in stab["stability_histogram"])
        lines.append(rf"\rcset{{eval.{i}.name}}      {{{meta['display_name']}}}")
        lines.append(rf"\rcset{{eval.{i}.hist}}      {{{hist_str}}}")
        lines.append(rf"\rcset{{eval.{i}.stable}}    {{{stab['n_stable']}}}")
        lines.append(rf"\rcset{{eval.{i}.sensitive}} {{{stab['n_sensitive']}}}")
        lines.append(rf"\rcset{{eval.{i}.pct}}       {{{stab['pct_sensitive']}}}")
    lines.append("")

    lines.append(r"% ------------------------------------------------ TIER CLASSIFICATION ----")
    strong = []
    middling = []
    fragile = []
    for name, data in evals:
        pct = data["perturbation_stability"]["pct_sensitive"]
        display = EVAL_META[name]["display_name"]
        spread = data["aggregate"]["variant_spread"]
        entry = rf"{display} ({pct}\% sensitive, {fmt(spread, 3)}pp spread)"
        if pct < 2.0:
            strong.append(entry)
        elif pct < 10.0:
            middling.append(entry)
        else:
            fragile.append(entry)

    lines.append(
        r"\rcset{tier.method}       {Sensitive items as fraction of total: "
        r"$<2\%$ strong, $2$--$10\%$ middling, $>10\%$ fragile. "
        r"Thresholds are descriptive, not normative.}"
    )
    lines.append(rf"\rcset{{tier.strong}}      {{{'; '.join(strong) if strong else '(none)'}}}")
    lines.append(
        rf"\rcset{{tier.middling}}     {{{'; '.join(middling) if middling else '(none)'}}}"
    )
    lines.append(rf"\rcset{{tier.fragile}}     {{{'; '.join(fragile) if fragile else '(none)'}}}")
    lines.append(
        r"\rcset{tier.interpretation}{Tier assignment is descriptive and "
        r"model-specific. It measures instrument reliability under "
        r"perturbation, not construct validity. A fragile eval may still "
        r"measure something important --- it just does so noisily.}"
    )
    lines.append("")

    lines.append(r"% ------------------------------------ SEQUENTIAL STOPPING ----")
    lines.append(
        r"\rcset{stop.method}       {Bernoulli mSPRT with "
        r"$\mathrm{Beta}(1,1)$ mixing, $\alpha{=}0.05$, "
        r"1000 permutations per eval.}"
    )
    stop_parts = []
    for name, data in evals:
        seq = data.get("sequential_stopping")
        if seq:
            display = EVAL_META[name]["display_name"]
            stop_parts.append(
                rf"{display}: {seq['median_stop']}/{seq['n_items']} "
                rf"(IQR [{seq['iqr'][0]}, {seq['iqr'][1]}])"
            )
    lines.append(rf"\rcset{{stop.summary}}     {{{'; '.join(stop_parts)}}}")
    lines.append(
        r"\rcset{stop.interpretation}{All evals reach the mSPRT boundary "
        r"well before exhausting the item pool ($p_0{=}0.5$). "
        r"Stopping efficiency varies with accuracy: high-accuracy evals "
        r"(ARC, CyberMetric) stop in $\sim$12 items; moderate-accuracy "
        r"evals (TruthfulQA) need $\sim$71.}"
    )
    lines.append("")

    lines.append(r"% ----------------------------------------------- RAW DATA REFERENCE ----")
    lines.append(r"\rcset{companion.file}    {\texttt{data/analysis/full\_summary.json}}")
    summary_file = SUMMARY_PATH
    if summary_file.exists():
        summary_hash = compute_file_sha256(summary_file)
    else:
        summary_json = json.dumps(summary, sort_keys=True)
        summary_hash = hashlib.sha256(summary_json.encode()).hexdigest()
    mid = len(summary_hash) // 2
    h1, h2 = summary_hash[:mid], summary_hash[mid:]
    lines.append(rf"\rcset{{companion.hash}}    {{\texttt{{sha256:{h1}\allowbreak {h2}}}}}")
    lines.append(
        r"\rcset{companion.contents}{Per-eval analysis summaries, "
        r"perturbation stability metrics, sequential stopping results, "
        r"and per-temperature breakdowns for all 6 benchmarks.}"
    )

    return "\n".join(lines) + "\n"


def generate_evals_csv(summary: dict) -> str:
    rows = ["eval,accuracy,ci_low,ci_high,spread,sensitive,n_items,n_variants"]
    for name in EVAL_ORDER:
        if name not in summary:
            continue
        data = summary[name]
        meta = EVAL_META[name]
        agg = data["aggregate"]
        stab = data["perturbation_stability"]
        rows.append(
            f"{meta['display_name']},"
            f"{fmt(agg['pooled_accuracy'])},"
            f"{fmt(agg['wilson_ci_95'][0])},"
            f"{fmt(agg['wilson_ci_95'][1])},"
            f"{fmt(agg['variant_spread'])},"
            f"{stab['pct_sensitive']},"
            f"{data['n_items']},"
            f"{data['n_variants']}"
        )
    return "\n".join(rows) + "\n"


def main() -> None:
    summary = json.loads(SUMMARY_PATH.read_text())

    for name, data in summary.items():
        if name not in EVAL_META:
            print(f"SKIP {name}: no metadata defined")
            continue

        out_dir = OUTPUT_BASE / name.replace("_", "-")
        out_dir.mkdir(parents=True, exist_ok=True)

        config_content = generate_config(name, data)
        config_path = out_dir / "runcard-config.tex"
        config_path.write_text(config_content)
        print(f"  {config_path}")

        # Audit seal
        data_file = Path(f"data/analysis/{name}_analysis.json")
        if data_file.exists():
            seal_result = audit_seal(
                run_id=f"MOJAVE-2026-0519-{EVAL_META[name]['id_suffix']}",
                eval_name=name,
                data_file=data_file,
            )
            if seal_result:
                config_lines = config_content.rstrip().split("\n")
                patched = []
                for line in config_lines:
                    if r"\rcset{audit.chain.tip}" in line:
                        tip = seal_result["chain_tip_hash"]
                        mid = len(tip) // 2
                        h1, h2 = tip[:mid], tip[mid:]
                        line = rf"\rcset{{audit.chain.tip}}   {{\texttt{{{h1}\allowbreak {h2}}}}}"
                    elif r"\rcset{audit.chain.seq}" in line:
                        line = rf"\rcset{{audit.chain.seq}}   {{{seal_result['chain_tip_seq']}}}"
                    elif r"\rcset{audit.signed}" in line:
                        if seal_result.get("attestation_cbor_b64"):
                            line = r"\rcset{audit.signed}      {Yes --- Ed25519 COSE\_Sign1}"
                        else:
                            line = r"\rcset{audit.signed}      {No --- chain only (unsigned)}"
                    patched.append(line)
                config_content = "\n".join(patched) + "\n"
                config_path.write_text(config_content)
                print(
                    f"    audit: seq={seal_result['chain_tip_seq']} "
                    f"tip={seal_result['chain_tip_hash'][:16]}..."
                )

        engine_src = Path("../../..") / TEMPLATE_DIR / "runcard.tex"
        engine_dst = out_dir / "runcard.tex"
        if engine_dst.exists() or engine_dst.is_symlink():
            engine_dst.unlink()
        engine_dst.symlink_to(engine_src)

        makefile_path = out_dir / "Makefile"
        makefile_path.write_text(
            "TEX = pdflatex -interaction=nonstopmode -halt-on-error\n\n"
            "runcard.pdf: runcard.tex runcard-config.tex\n"
            "\t$(TEX) runcard.tex\n"
            "\t$(TEX) runcard.tex\n\n"
            ".PHONY: clean veryclean\n"
            "clean:\n"
            "\trm -f *.aux *.log *.out *.toc\n"
            "veryclean: clean\n"
            "\trm -f runcard.pdf\n"
        )

    # --- Cross-eval summary ---
    CROSS_EVAL_OUTPUT.mkdir(parents=True, exist_ok=True)

    cross_config = generate_cross_eval_config(summary)
    cross_config_path = CROSS_EVAL_OUTPUT / "cross-eval-config.tex"
    cross_config_path.write_text(cross_config)
    print(f"  {cross_config_path}")

    evals_csv = generate_evals_csv(summary)
    csv_path = CROSS_EVAL_OUTPUT / "evals.csv"
    csv_path.write_text(evals_csv)
    print(f"  {csv_path}")

    engine_src = Path("../../..") / CROSS_EVAL_TEMPLATE_DIR / "cross-eval.tex"
    engine_dst = CROSS_EVAL_OUTPUT / "cross-eval.tex"
    if engine_dst.exists() or engine_dst.is_symlink():
        engine_dst.unlink()
    engine_dst.symlink_to(engine_src)

    makefile_path = CROSS_EVAL_OUTPUT / "Makefile"
    makefile_path.write_text(
        "TEX = pdflatex -interaction=nonstopmode -halt-on-error\n\n"
        "cross-eval.pdf: cross-eval.tex cross-eval-config.tex evals.csv\n"
        "\t$(TEX) cross-eval.tex\n"
        "\t$(TEX) cross-eval.tex\n\n"
        ".PHONY: clean veryclean\n"
        "clean:\n"
        "\trm -f *.aux *.log *.out *.toc\n"
        "veryclean: clean\n"
        "\trm -f cross-eval.pdf\n"
    )

    # --- Top-level Makefile ---
    top_makefile = OUTPUT_BASE / "Makefile"
    subdirs = sorted(
        d.name for d in OUTPUT_BASE.iterdir() if d.is_dir() and (d / "Makefile").exists()
    )
    top_makefile.write_text(
        ".PHONY: all clean\n"
        f"SUBDIRS = {' '.join(subdirs)}\n\n"
        "all:\n"
        "\t$(foreach d,$(SUBDIRS),$(MAKE) -C $(d) &&) true\n\n"
        "clean:\n"
        "\t$(foreach d,$(SUBDIRS),$(MAKE) -C $(d) clean &&) true\n"
    )
    print(f"\n  Top-level Makefile: {top_makefile}")
    print(f"  {len(subdirs)} directories generated")


if __name__ == "__main__":
    main()
