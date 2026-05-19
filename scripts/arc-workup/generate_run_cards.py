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
}


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
        rf"\rcset{{perturb.design}}    {{Fully crossed block design: "
        rf"36 option-order seeds $\times$ 4 temperatures "
        rf"({temp_list}) + 1 deterministic baseline ($T{{=}}0$). "
        rf"{data['n_variants']} variants per item.}}"
    )
    lines.append(
        rf"\rcset{{inference.config}}  {{Per-variant: temperature $\in\{{{temp_list}\}}$; "
        rf"option-order seed $\in$ 36 pseudo-random seeds; "
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

    data_json = json.dumps(data, sort_keys=True)
    data_hash = hashlib.sha256(data_json.encode()).hexdigest()[:16]
    lines.append(rf"\rcset{{companion.hash}}    {{\texttt{{sha256:{data_hash}}}}}")
    lines.append(
        r"\rcset{companion.contents}{Per-variant accuracy, per-item response matrix, "
        r"perturbation stability analysis, sequential stopping results, "
        r"and the variant manifest.}"
    )
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
        r"\rcset{eval.selection}    {6 benchmarks from UK AISI Inspect Evals: "
        r"ARC Challenge, CyberMetric-2000, HellaSwag, MMLU (0-shot), "
        r"TruthfulQA, GSM8K. Selected to span accuracy levels and "
        r"metrological quality tiers.}"
    )
    lines.append(r"\rcset{evaluator.org}     {antimeme.ai}")
    lines.append(r"\rcset{evaluator.tool}    {UK AISI Inspect AI}")
    lines.append("")

    lines.append(r"% -------------------------------------------------- PERTURBATION DESIGN ----")
    lines.append(
        r"\rcset{perturb.design}    {Fully crossed: 36 option-order seeds "
        r"$\times$ 4 temperatures (0.3, 0.5, 0.7, 1.0) + 1 deterministic "
        r"baseline ($T{=}0$). 145 variants per item per eval "
        r"(ARC: 178 variants with extended temperature range).}"
    )
    lines.append(
        r"\rcset{inference.config}  {Per-variant: temperature "
        r"$\in\{0.0, 0.3, 0.5, 0.7, 1.0\}$; option-order seed "
        r"$\in$ 36 pseudo-random seeds; top\_p $=1.0$; greedy at $T{=}0$.}"
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
    summary_json = json.dumps(summary, sort_keys=True)
    summary_hash = hashlib.sha256(summary_json.encode()).hexdigest()[:16]
    lines.append(rf"\rcset{{companion.hash}}    {{\texttt{{sha256:{summary_hash}}}}}")
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
