#!/usr/bin/env python3
"""Generate v2 run cards with Sobol' indices from analysis data.

Reads a v2 analysis JSON (from analyze_sobol.py, which calls mojave-gsa
analyze) and creates a run card directory with runcard-config.tex + symlinks
to the template engine.

Usage:
    python generate_run_cards.py <analysis.json> [--output-dir data/run-cards-v2]
"""

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
import time
from pathlib import Path
from typing import Any


def compute_file_sha256(path: Path) -> str:
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(8192), b""):
            h.update(chunk)
    return h.hexdigest()


def audit_seal(run_id: str, eval_name: str, data_file: Path) -> dict[str, Any] | None:
    data_sha256 = compute_file_sha256(data_file)
    seal_input = {
        "run_id": run_id,
        "eval_name": eval_name,
        "date_issued": time.strftime("%Y-%m-%d"),
        "data_file": str(data_file),
        "data_sha256": data_sha256,
        "actor": {"kind": "System", "id": "generate_run_cards_v2.py"},
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
        return json.loads(result.stdout)  # type: ignore[no-any-return]
    except FileNotFoundError:
        print("  WARN: mojave binary not found, skipping audit seal")
        return None
    except subprocess.TimeoutExpired:
        print("  WARN: mojave audit seal timed out")
        return None


def fmt(v: float, decimals: int = 4) -> str:
    return f"{v:.{decimals}f}"


def generate_sobol_table_latex(indices: list[dict[str, Any]]) -> str:
    rows = []
    for idx in indices:
        rows.append(
            rf"    {idx['axis']} & {fmt(idx['S1'])} & "
            rf"[{fmt(idx['S1_ci_low'])}, {fmt(idx['S1_ci_high'])}] & "
            rf"{fmt(idx['ST'])} & "
            rf"[{fmt(idx['ST_ci_low'])}, {fmt(idx['ST_ci_high'])}] \\"
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
        rows.append(rf"    {idx['axis']} & {fmt(idx['delta'])} \\")
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


def generate_config(analysis: dict[str, Any], analysis_path: Path) -> str:
    agg = analysis["aggregate"]
    design = analysis["design"]
    sobol = analysis["sobol_indices"]
    borgonovo = analysis["borgonovo_indices"]
    diag = analysis["sobol_diagnostics"]

    eval_name = analysis["eval"]
    model = analysis["model"]
    run_id = f"MOJAVE-V2-{eval_name.upper()}"

    dominant = sobol[0] if sobol else None

    lines: list[str] = []
    lines.append(r"% " + "=" * 76)
    lines.append(f"% MOJAVE V2 RUN CARD -- {eval_name}")
    lines.append(r"% " + "=" * 76)
    lines.append("")

    lines.append(rf"\rcset{{run.id}}            {{{run_id}}}")
    lines.append(rf"\rcset{{date.issued}}       {{{time.strftime('%Y-%m-%d')}}}")
    lines.append(rf"\rcset{{benchmark.name}}    {{{eval_name}}}")
    lines.append(r"\rcset{benchmark.version} {inspect\_evals 0.12.0}")
    lines.append(r"\rcset{benchmark.source}  {\texttt{cais/wmdp}}")
    lines.append(rf"\rcset{{model.name}}        {{{model}}}")
    lines.append(r"\rcset{model.revision}    {\texttt{HuggingFace (default)}}")
    lines.append(r"\rcset{model.quant}       {bf16 + fp8 (perturbation axis)}")
    lines.append(r"\rcset{model.serving}     {vLLM on RunPod}")
    lines.append(r"\rcset{evaluator.org}     {antimeme.ai}")
    lines.append(r"\rcset{evaluator.tool}    {UK AISI Inspect AI}")
    lines.append("")

    lines.append(
        rf"\rcset{{n.items}}           "
        rf"{{{analysis.get('n_items', '--')}}}"
    )
    lines.append(rf"\rcset{{n.variants}}        {{{analysis['n_cells']}}}")
    lines.append(
        rf"\rcset{{perturb.design}}    {{Saltelli radial: "
        rf"$N = {design['N_base']}$, $k = {design['k']}$, "
        rf"cells $= {analysis['n_cells']}$.}}"
    )
    lines.append("")

    lines.append(rf"\rcset{{acc.overall}}       {{{fmt(agg['mean_accuracy'])}}}")
    if agg.get("spread") is not None:
        lines.append(
            rf"\rcset{{wilson.lower}}      "
            rf"{{{fmt(agg['min_accuracy'])}}}"
        )
        lines.append(
            rf"\rcset{{wilson.upper}}      "
            rf"{{{fmt(agg['max_accuracy'])}}}"
        )
        lines.append(rf"\rcset{{wilson.width}}      {{{fmt(agg['spread'])}}}")
    lines.append("")

    lines.append(r"% ----------------------------------------------- SOBOL' ----")
    lines.append(r"\rcset{sobol.design}      {Saltelli radial (salib-rs 0.1.1)}")
    lines.append(rf"\rcset{{sobol.N.base}}      {{{design['N_base']}}}")
    lines.append(rf"\rcset{{sobol.k}}           {{{design['k']}}}")
    lines.append(rf"\rcset{{sobol.n.cells}}     {{{analysis['n_cells']}}}")
    if dominant:
        lines.append(
            rf"\rcset{{sobol.dominant}}    {{{dominant['axis']} "
            rf"($S_{{T_i}} = {fmt(dominant['ST'])}$)}}"
        )
    else:
        lines.append(r"\rcset{sobol.dominant}    {--}")
    lines.append(rf"\rcset{{sobol.sum.S1}}      {{{diag['sum_s1']}}}")
    lines.append(rf"\rcset{{sobol.sum.ST}}      {{{diag['sum_st']}}}")

    sobol_table = generate_sobol_table_latex(sobol)
    lines.append(rf"\rcset{{sobol.table}}       {{{sobol_table}}}")

    borgonovo_table = generate_borgonovo_table_latex(borgonovo)
    lines.append(rf"\rcset{{borgonovo.table}}   {{{borgonovo_table}}}")
    lines.append("")

    for key in [
        "irt.model",
        "irt.diff.mean",
        "irt.diff.sd",
        "irt.diff.min",
        "irt.diff.max",
        "irt.diff.hist",
        "irt.disc.mean",
        "irt.disc.sd",
        "irt.disc.min",
        "irt.disc.max",
        "irt.disc.hist",
        "irt.disc.caveat",
        "irt.floor.items",
        "irt.ceil.items",
        "irt.nearzero.disc",
        "q3.summary",
        "q3.count.gt2",
        "q3.note",
    ]:
        lines.append(rf"\rcset{{{key}}}        {{}}")
    lines.append("")

    lines.append(r"\rcset{stab.hist}         {}")
    lines.append(r"\rcset{stab.stable}       {}")
    lines.append(r"\rcset{stab.sensitive}    {}")
    lines.append(r"\rcset{dim.order}         {}")
    lines.append(r"\rcset{dim.temp}          {}")
    lines.append(r"\rcset{dim.prompt}        {}")
    lines.append(r"\rcset{dim.mixed}         {}")
    lines.append(r"\rcset{spearman.diff.rho} {}")
    lines.append(r"\rcset{spearman.diff.lo}  {}")
    lines.append(r"\rcset{spearman.diff.hi}  {}")
    lines.append(r"\rcset{spearman.disc.rho} {}")
    lines.append(r"\rcset{spearman.disc.lo}  {}")
    lines.append(r"\rcset{spearman.disc.hi}  {}")
    lines.append("")

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

    name_escaped = eval_name.replace("_", r"\_")
    lines.append(
        rf"\rcset{{companion.file}}    "
        rf"{{\texttt{{data/v2/{name_escaped}\_analysis.json}}}}"
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
    lines.append("")

    lines.append(
        r"\rcset{disc.2pl}{\textbf{IRT not fitted.} Item Response Theory "
        r"parameters were not estimated for this run.}"
    )
    lines.append(
        r"\rcset{disc.unidim}{\textbf{Single model only.} Perturbation "
        r"stability results are model-specific.}"
    )

    return "\n".join(lines) + "\n"


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("analysis", type=Path, help="Sobol' analysis JSON")
    parser.add_argument(
        "--output-dir",
        type=Path,
        default=Path("data/run-cards-v2"),
    )
    args = parser.parse_args()

    analysis: dict[str, Any] = json.loads(args.analysis.read_text())
    eval_name = analysis["eval"]

    out_dir = args.output_dir / eval_name.replace("_", "-")
    out_dir.mkdir(parents=True, exist_ok=True)

    config_content = generate_config(analysis, args.analysis)
    config_path = out_dir / "runcard-config.tex"
    config_path.write_text(config_content)
    print(f"  {config_path}")

    seal_result = audit_seal(
        run_id=f"MOJAVE-V2-{eval_name.upper()}",
        eval_name=eval_name,
        data_file=args.analysis,
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

    engine_src = Path("../../../templates/run-card/single-run-card/runcard.tex")
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

    print(f"\n  Run card directory: {out_dir}")
    print(f"  Build: cd {out_dir} && make")


if __name__ == "__main__":
    main()
