#!/usr/bin/env python3
"""Generate v2 run cards with Sobol' indices from analysis data.

Reads a v2 analysis JSON (from analyze_sobol.py, which calls mojave-gsa
analyze) and a rescored JSON (from rescore_fast.py --json), then creates
a run card directory with runcard-config.tex + symlinks to the v2 template.

Usage:
    python generate_run_cards.py <analysis.json> --rescored <rescored.json> \
        [--output-dir data/run-cards-v2]
"""

from __future__ import annotations

import argparse
import json
import math
import time
from pathlib import Path
from typing import Any

from repo import audit_seal, compute_file_sha256


def fmt(v: float, decimals: int = 4) -> str:
    return f"{v:.{decimals}f}"


def tex_escape(s: str) -> str:
    return s.replace("_", r"\_").replace("&", r"\&").replace("%", r"\%")


def welch_t_test(
    cells: list[dict[str, Any]],
) -> dict[str, Any]:
    bf16 = [c["rescore_acc"] for c in cells if c["quantization"] == "bf16"]
    fp8 = [c["rescore_acc"] for c in cells if c["quantization"] == "fp8"]

    n1, n2 = len(bf16), len(fp8)
    m1 = sum(bf16) / n1
    m2 = sum(fp8) / n2
    s1 = math.sqrt(sum((x - m1) ** 2 for x in bf16) / (n1 - 1))
    s2 = math.sqrt(sum((x - m2) ** 2 for x in fp8) / (n2 - 1))

    se = math.sqrt(s1**2 / n1 + s2**2 / n2)
    t_stat = (m1 - m2) / se if se > 0 else 0.0

    # Welch-Satterthwaite df
    num = (s1**2 / n1 + s2**2 / n2) ** 2
    denom = (s1**2 / n1) ** 2 / (n1 - 1) + (s2**2 / n2) ** 2 / (n2 - 1)
    df = num / denom if denom > 0 else n1 + n2 - 2

    # two-tailed p via normal approx (good enough for df > 100)
    z = abs(t_stat)
    p = 2 * (1 - 0.5 * (1 + math.erf(z / math.sqrt(2))))

    return {
        "bf16_n": n1,
        "bf16_mean": m1,
        "bf16_sd": s1,
        "fp8_n": n2,
        "fp8_mean": m2,
        "fp8_sd": s2,
        "diff": m1 - m2,
        "t": t_stat,
        "df": df,
        "p": p,
    }


def make_accuracy_histogram(cells: list[dict[str, Any]], n_bins: int = 20) -> str:
    accs = [c["rescore_acc"] for c in cells]
    bin_width = 1.0 / n_bins
    counts = [0] * n_bins
    for a in accs:
        idx = min(int(a / bin_width), n_bins - 1)
        counts[idx] += 1
    return ",".join(str(c) for c in counts)


def generate_sobol_table_latex(indices: list[dict[str, Any]]) -> str:
    rows = []
    for idx in indices:
        axis = tex_escape(idx["axis"])
        rows.append(
            rf"    {axis} & {fmt(idx['S1'])} & "
            rf"[{fmt(idx['S1_ci_low'])},\,{fmt(idx['S1_ci_high'])}] & "
            rf"{fmt(idx['ST'])} & "
            rf"[{fmt(idx['ST_ci_low'])},\,{fmt(idx['ST_ci_high'])}] \\"
        )
    header = (
        r"  \begin{tabular}{@{}l r r r r@{}}"
        "\n"
        r"  \toprule"
        "\n"
        r"  Axis & $S_i$ & 95\% CI & $S_{T_i}$ & 95\% CI \\"
        "\n"
        r"  \midrule"
        "\n"
    )
    footer = r"  \bottomrule" "\n" r"  \end{tabular}"
    return header + "\n".join(rows) + "\n" + footer


def generate_borgonovo_table_latex(indices: list[dict[str, Any]]) -> str:
    rows = []
    for idx in indices:
        axis = tex_escape(idx["axis"])
        rows.append(rf"    {axis} & {fmt(idx['delta'])} \\")
    header = (
        r"  \begin{tabular}{@{}l r@{}}"
        "\n"
        r"  \toprule"
        "\n"
        r"  Axis & $\delta_i$ \\"
        "\n"
        r"  \midrule"
        "\n"
    )
    footer = r"  \bottomrule" "\n" r"  \end{tabular}"
    return header + "\n".join(rows) + "\n" + footer


def generate_config(
    analysis: dict[str, Any],
    analysis_path: Path,
    rescored: list[dict[str, Any]] | None = None,
) -> str:
    agg = analysis["aggregate"]
    design = analysis["design"]
    sobol = analysis["sobol_indices"]
    borgonovo = analysis["borgonovo_indices"]
    diag = analysis["sobol_diagnostics"]

    eval_name = analysis["eval"]
    model = analysis["model"]
    run_id = f"MOJAVE-V2-{eval_name.upper()}".replace("_", r"\_")

    dominant = sobol[0] if sobol else None

    axes_list = [tex_escape(s["axis"]) for s in sobol]
    axes_str = ", ".join(axes_list)

    lines: list[str] = []
    lines.append(r"% " + "=" * 76)
    lines.append(f"% MOJAVE V2 RUN CARD -- {eval_name}")
    lines.append(r"% " + "=" * 76)
    lines.append("")

    # --- Header ---
    lines.append(rf"\rcset{{run.id}}            {{{run_id}}}")
    lines.append(rf"\rcset{{date.issued}}       {{{time.strftime('%Y-%m-%d')}}}")
    name_escaped = tex_escape(eval_name)
    lines.append(rf"\rcset{{benchmark.name}}    {{{name_escaped}}}")
    lines.append(r"\rcset{benchmark.version} {inspect\_evals 0.12.0}")
    lines.append(r"\rcset{benchmark.source}  {\texttt{cais/wmdp}}")
    model_escaped = tex_escape(model)
    lines.append(rf"\rcset{{model.name}}        {{{model_escaped}}}")
    lines.append(r"\rcset{model.revision}    {\texttt{HuggingFace (default)}}")
    lines.append(r"\rcset{model.quant}       {bf16 + fp8 (perturbation axis)}")
    lines.append(r"\rcset{model.serving}     {vLLM on RunPod}")
    lines.append(r"\rcset{evaluator.org}     {antimeme.ai}")
    lines.append(r"\rcset{evaluator.tool}    {UK AISI Inspect AI}")
    lines.append("")

    # --- Design ---
    lines.append(
        rf"\rcset{{perturb.design}}    {{Saltelli radial: "
        rf"$N = {design['N_base']}$, $k = {design['k']}$, "
        rf"cells $= {analysis['n_cells']}$.}}"
    )
    lines.append(rf"\rcset{{sobol.N.base}}      {{{design['N_base']}}}")
    lines.append(rf"\rcset{{sobol.k}}           {{{design['k']}}}")
    lines.append(rf"\rcset{{sobol.n.cells}}     {{{analysis['n_cells']}}}")
    lines.append(rf"\rcset{{perturb.axes}}      {{{axes_str}}}")
    lines.append(
        r"\rcset{inference.config}  {Per-cell: temperature "
        r"$\in\{0.0, 0.7, 1.0\}$; prompt template "
        r"$\in\{$direct, cot, letter-only, repeat-stem$\}$; "
        r"system prompt $\in\{$helpful, domain-expert, none$\}$; "
        r"n-shot fraction $\in [0, 0.05]$; "
        r"choice order $\in\{$original, shuffled$\}$; "
        r"quantization $\in\{$bf16, fp8$\}$.}"
    )
    lines.append("")

    # --- Aggregate Results ---
    lines.append(rf"\rcset{{acc.overall}}       {{{fmt(agg['mean_accuracy'])}}}")
    lines.append(rf"\rcset{{acc.sd}}            {{{fmt(agg['sd'])}}}")
    lines.append(rf"\rcset{{acc.min}}           {{{fmt(agg['min_accuracy'])}}}")
    lines.append(rf"\rcset{{acc.max}}           {{{fmt(agg['max_accuracy'])}}}")
    lines.append(rf"\rcset{{acc.spread}}        {{{fmt(agg['spread'])}}}")
    lines.append("")

    # --- Sobol' ---
    lines.append(r"% ----------------------------------------------- SOBOL' ----")
    lines.append(r"\rcset{sobol.design}      {Saltelli radial (salib-rs 0.1.1)}")
    if dominant:
        dom_axis = tex_escape(dominant["axis"])
        lines.append(
            rf"\rcset{{sobol.dominant}}    {{{dom_axis} "
            rf"($S_{{T_i}} = {fmt(dominant['ST'])}$)}}"
        )
        lines.append(rf"\rcset{{sobol.dominant.name}}{{{dom_axis}}}")
    else:
        lines.append(r"\rcset{sobol.dominant}    {--}")
        lines.append(r"\rcset{sobol.dominant.name}{--}")
    lines.append(rf"\rcset{{sobol.sum.S1}}      {{{fmt(diag['sum_s1'], 3)}}}")
    lines.append(rf"\rcset{{sobol.sum.ST}}      {{{fmt(diag['sum_st'], 3)}}}")

    sobol_table = generate_sobol_table_latex(sobol)
    lines.append(rf"\rcset{{sobol.table}}       {{{sobol_table}}}")

    borgonovo_table = generate_borgonovo_table_latex(borgonovo)
    lines.append(rf"\rcset{{borgonovo.table}}   {{{borgonovo_table}}}")
    lines.append("")

    # --- Quantization comparison ---
    if rescored:
        qt = welch_t_test(rescored)
        lines.append(r"% ------------------------------------------ QUANT COMPARISON ----")
        lines.append(rf"\rcset{{quant.bf16.n}}      {{{qt['bf16_n']}}}")
        lines.append(rf"\rcset{{quant.bf16.mean}}   {{{fmt(qt['bf16_mean'])}}}")
        lines.append(rf"\rcset{{quant.bf16.sd}}     {{{fmt(qt['bf16_sd'])}}}")
        lines.append(rf"\rcset{{quant.fp8.n}}       {{{qt['fp8_n']}}}")
        lines.append(rf"\rcset{{quant.fp8.mean}}    {{{fmt(qt['fp8_mean'])}}}")
        lines.append(rf"\rcset{{quant.fp8.sd}}      {{{fmt(qt['fp8_sd'])}}}")
        lines.append(rf"\rcset{{quant.diff}}        {{{fmt(qt['diff'])}pp}}")
        lines.append(rf"\rcset{{quant.t}}           {{{fmt(qt['t'], 3)}}}")
        lines.append(rf"\rcset{{quant.p}}           {{{fmt(qt['p'], 4)}}}")
        lines.append(rf"\rcset{{quant.df}}          {{{fmt(qt['df'], 1)}}}")
        if qt["p"] < 0.05:
            lines.append(
                r"\rcset{quant.interp}      {Statistically significant "
                r"at $\alpha = 0.05$.}"
            )
        else:
            lines.append(
                r"\rcset{quant.interp}      {NOT statistically significant "
                r"at $\alpha = 0.05$. Quantization does not meaningfully "
                r"affect accuracy under this perturbation design.}"
            )
        lines.append("")

        # accuracy histogram across all cells
        hist = make_accuracy_histogram(rescored)
        lines.append(rf"\rcset{{acc.hist}}          {{{hist}}}")
        lines.append("")

    # --- Companion / audit ---
    name_file_escaped = eval_name.replace("_", r"\_")
    lines.append(
        rf"\rcset{{companion.file}}    "
        rf"{{\texttt{{data/v2/{name_file_escaped}\_sobol\_analysis.json}}}}"
    )
    data_hash = compute_file_sha256(analysis_path)
    mid = len(data_hash) // 2
    lines.append(
        rf"\rcset{{companion.hash}}    "
        rf"{{\texttt{{sha256:{data_hash[:mid]}"
        rf"\allowbreak {data_hash[mid:]}}}}}"
    )
    lines.append(
        r"\rcset{companion.contents}{Sobol' indices, Borgonovo delta, "
        r"per-cell accuracy, perturbation manifest.}"
    )
    lines.append("")

    lines.append(r"\rcset{audit.chain.tip}   {}")
    lines.append(r"\rcset{audit.chain.seq}   {}")
    lines.append(r"\rcset{audit.signed}      {}")

    return "\n".join(lines) + "\n"


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("analysis", type=Path, help="Sobol' analysis JSON")
    parser.add_argument(
        "--rescored",
        type=Path,
        default=None,
        help="Rescored JSON (from rescore_fast.py --json)",
    )
    parser.add_argument(
        "--output-dir",
        type=Path,
        default=Path("data/run-cards-v2"),
    )
    args = parser.parse_args()

    analysis: dict[str, Any] = json.loads(args.analysis.read_text())
    rescored_data: list[dict[str, Any]] | None = None
    if args.rescored:
        rescored_data = json.loads(args.rescored.read_text())
    eval_name = analysis["eval"]

    out_dir = args.output_dir / eval_name.replace("_", "-")
    out_dir.mkdir(parents=True, exist_ok=True)

    config_content = generate_config(analysis, args.analysis, rescored=rescored_data)
    config_path = out_dir / "runcard-config.tex"
    config_path.write_text(config_content)
    print(f"  {config_path}")

    seal_result = audit_seal(
        run_id=f"MOJAVE-V2-{eval_name.upper()}",
        eval_name=eval_name,
        data_file=args.analysis,
        model_name=analysis.get("model_name", "unknown"),
        model_provider=analysis.get("model_provider", "unknown"),
        model_hash=analysis.get("model_hash", "00" * 32),
        model_hash_method=analysis.get("model_hash_method", "StructuredDescriptor"),
    )
    if seal_result:
        config_lines = config_content.rstrip().split("\n")
        patched = []
        for line in config_lines:
            if r"\rcset{audit.chain.tip}" in line:
                tip = seal_result["chain_tip_hash"]
                tip_mid = len(tip) // 2
                line = (
                    rf"\rcset{{audit.chain.tip}}   "
                    rf"{{\texttt{{{tip[:tip_mid]}"
                    rf"\allowbreak {tip[tip_mid:]}}}}}"
                )
            elif r"\rcset{audit.chain.seq}" in line:
                line = (
                    rf"\rcset{{audit.chain.seq}}   "
                    rf"{{{seal_result['chain_tip_seq']}}}"
                )
            elif r"\rcset{audit.signed}" in line:
                if seal_result.get("attestation_cbor_b64"):
                    line = (
                        r"\rcset{audit.signed}      "
                        r"{Yes --- Ed25519 COSE\_Sign1}"
                    )
                else:
                    line = (
                        r"\rcset{audit.signed}      "
                        r"{No --- chain only (unsigned)}"
                    )
            patched.append(line)
        config_content = "\n".join(patched) + "\n"
        config_path.write_text(config_content)
        print(f"    audit: seq={seal_result['chain_tip_seq']}")

    # Symlink v2 template engine
    engine_src = Path("../../../templates/run-card/single-run-card/runcard-v2.tex")
    engine_dst = out_dir / "runcard-v2.tex"
    if engine_dst.exists() or engine_dst.is_symlink():
        engine_dst.unlink()
    engine_dst.symlink_to(engine_src)

    makefile_path = out_dir / "Makefile"
    makefile_path.write_text(
        "TEX = pdflatex -interaction=nonstopmode -halt-on-error\n\n"
        "runcard.pdf: runcard-v2.tex runcard-config.tex\n"
        "\t$(TEX) runcard-v2.tex\n"
        "\t$(TEX) runcard-v2.tex\n"
        "\tcp runcard-v2.pdf runcard.pdf\n\n"
        ".PHONY: clean veryclean\n"
        "clean:\n"
        "\trm -f *.aux *.log *.out *.toc\n"
        "veryclean: clean\n"
        "\trm -f runcard.pdf runcard-v2.pdf\n"
    )

    print(f"\n  Run card directory: {out_dir}")
    print(f"  Build: cd {out_dir} && make")


if __name__ == "__main__":
    main()
