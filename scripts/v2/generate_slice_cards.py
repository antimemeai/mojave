#!/usr/bin/env python3
"""Generate per-axis-value slice cards from rescored data.

For each perturbation axis and each value that axis takes, generates a
1-page slice card showing conditional accuracy stats, histogram, and
bf16 vs fp8 comparison within that slice.

Usage:
    python generate_slice_cards.py <rescored.json> <analysis.json> \
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


def slug(s: str) -> str:
    return s.replace("=", "").replace(" ", "").replace(".", "").lower()


def welch_t(a: list[float], b: list[float]) -> dict[str, Any]:
    if len(a) < 2 or len(b) < 2:
        return {
            "t": 0.0,
            "p": 1.0,
            "df": 0.0,
            "m1": sum(a) / max(len(a), 1),
            "s1": 0.0,
            "n1": len(a),
            "m2": sum(b) / max(len(b), 1),
            "s2": 0.0,
            "n2": len(b),
        }
    n1, n2 = len(a), len(b)
    m1 = sum(a) / n1
    m2 = sum(b) / n2
    s1 = math.sqrt(sum((x - m1) ** 2 for x in a) / (n1 - 1))
    s2 = math.sqrt(sum((x - m2) ** 2 for x in b) / (n2 - 1))
    se = math.sqrt(s1**2 / n1 + s2**2 / n2)
    t_stat = (m1 - m2) / se if se > 0 else 0.0
    num = (s1**2 / n1 + s2**2 / n2) ** 2
    denom = (s1**2 / n1) ** 2 / (n1 - 1) + (s2**2 / n2) ** 2 / (n2 - 1)
    df = num / denom if denom > 0 else n1 + n2 - 2
    z = abs(t_stat)
    p = 2 * (1 - 0.5 * (1 + math.erf(z / math.sqrt(2))))
    return {
        "t": t_stat,
        "p": p,
        "df": df,
        "m1": m1,
        "s1": s1,
        "n1": n1,
        "m2": m2,
        "s2": s2,
        "n2": n2,
    }


def make_histogram(accs: list[float], n_bins: int = 20) -> str:
    bin_width = 1.0 / n_bins
    counts = [0] * n_bins
    for a in accs:
        idx = min(int(a / bin_width), n_bins - 1)
        counts[idx] += 1
    return ",".join(str(c) for c in counts)


def generate_slice_config(
    cells: list[dict[str, Any]],
    axis: str,
    value: str,
    eval_name: str,
    model: str,
    total_cells: int,
    overall_mean: float,
    all_axes: list[str],
    rescored_path: Path,
    analysis_path: Path,
) -> str:
    accs = [c["rescore_acc"] for c in cells]
    n = len(accs)
    mean_acc = sum(accs) / n
    sd = math.sqrt(sum((x - mean_acc) ** 2 for x in accs) / (n - 1)) if n > 1 else 0.0
    min_acc = min(accs)
    max_acc = max(accs)
    spread = max_acc - min_acc
    diff_overall = mean_acc - overall_mean

    axis_esc = tex_escape(axis)
    value_esc = tex_escape(value)
    eval_esc = tex_escape(eval_name)
    model_esc = tex_escape(model)
    run_id = f"MOJAVE-V2-{eval_name.upper()}-{axis.upper()}-{value.upper()}"
    run_id = tex_escape(run_id.replace("=", "").replace(" ", "").replace(".", ""))
    parent_id = tex_escape(f"MOJAVE-V2-{eval_name.upper()}")

    other_axes = [tex_escape(a) for a in all_axes if a != axis]
    other_str = ", ".join(other_axes)

    lines: list[str] = []
    lines.append(r"% " + "=" * 76)
    lines.append(f"% MOJAVE V2 SLICE CARD -- {eval_name} / {axis}={value}")
    lines.append(r"% " + "=" * 76)
    lines.append("")

    lines.append(rf"\rcset{{run.id}}            {{{run_id}}}")
    lines.append(rf"\rcset{{parent.run.id}}     {{{parent_id}}}")
    lines.append(rf"\rcset{{date.issued}}       {{{time.strftime('%Y-%m-%d')}}}")
    lines.append(rf"\rcset{{benchmark.name}}    {{{eval_esc}}}")
    lines.append(r"\rcset{benchmark.version} {inspect\_evals 0.12.0}")
    lines.append(rf"\rcset{{model.name}}        {{{model_esc}}}")
    lines.append(r"\rcset{model.quant}       {bf16 + fp8 (perturbation axis)}")
    lines.append(r"\rcset{model.serving}     {vLLM on RunPod}")
    lines.append(r"\rcset{evaluator.org}     {antimeme.ai}")
    lines.append(r"\rcset{evaluator.tool}    {UK AISI Inspect AI}")
    lines.append("")

    lines.append(rf"\rcset{{slice.axis}}        {{{axis_esc}}}")
    lines.append(rf"\rcset{{slice.value}}       {{{value_esc}}}")
    lines.append(rf"\rcset{{slice.n.cells}}     {{{n}}}")
    lines.append(rf"\rcset{{total.n.cells}}     {{{total_cells}}}")
    lines.append(rf"\rcset{{slice.other.axes}}  {{{other_str}}}")
    lines.append("")

    lines.append(rf"\rcset{{acc.overall}}       {{{fmt(mean_acc)}}}")
    lines.append(rf"\rcset{{acc.sd}}            {{{fmt(sd)}}}")
    lines.append(rf"\rcset{{acc.min}}           {{{fmt(min_acc)}}}")
    lines.append(rf"\rcset{{acc.max}}           {{{fmt(max_acc)}}}")
    lines.append(rf"\rcset{{acc.spread}}        {{{fmt(spread)}}}")
    sign = "+" if diff_overall >= 0 else ""
    lines.append(
        rf"\rcset{{acc.vs.overall}}    {{{sign}{fmt(diff_overall)}pp "
        rf"vs.\ overall mean {fmt(overall_mean)}}}"
    )
    lines.append("")

    hist = make_histogram(accs)
    lines.append(rf"\rcset{{acc.hist}}          {{{hist}}}")
    lines.append("")

    # bf16 vs fp8 within slice (skip if axis IS quantization)
    if axis != "quantization":
        bf16 = [c["rescore_acc"] for c in cells if c["quantization"] == "bf16"]
        fp8 = [c["rescore_acc"] for c in cells if c["quantization"] == "fp8"]
        if bf16 and fp8:
            qt = welch_t(bf16, fp8)
            lines.append(rf"\rcset{{quant.bf16.n}}      {{{qt['n1']}}}")
            lines.append(rf"\rcset{{quant.bf16.mean}}   {{{fmt(qt['m1'])}}}")
            lines.append(rf"\rcset{{quant.bf16.sd}}     {{{fmt(qt['s1'])}}}")
            lines.append(rf"\rcset{{quant.fp8.n}}       {{{qt['n2']}}}")
            lines.append(rf"\rcset{{quant.fp8.mean}}    {{{fmt(qt['m2'])}}}")
            lines.append(rf"\rcset{{quant.fp8.sd}}      {{{fmt(qt['s2'])}}}")
            lines.append(rf"\rcset{{quant.diff}}        {{{fmt(qt['m1'] - qt['m2'])}pp}}")
            lines.append(rf"\rcset{{quant.t}}           {{{fmt(qt['t'], 3)}}}")
            lines.append(rf"\rcset{{quant.p}}           {{{fmt(qt['p'], 4)}}}")
            if qt["p"] < 0.05:
                lines.append(
                    r"\rcset{quant.interp}      "
                    r"{Statistically significant at $\alpha = 0.05$.}"
                )
            else:
                lines.append(
                    r"\rcset{quant.interp}      "
                    r"{NOT statistically significant at $\alpha = 0.05$.}"
                )
    else:
        lines.append(
            r"\rcset{quant.interp}      "
            r"{N/A --- this slice IS a quantization level.}"
        )
    lines.append("")

    # companion hashes
    name_file_esc = eval_name.replace("_", r"\_")
    rescored_hash = compute_file_sha256(rescored_path)
    analysis_hash = compute_file_sha256(analysis_path)
    mid_r = len(rescored_hash) // 2
    mid_a = len(analysis_hash) // 2
    lines.append(
        rf"\rcset{{companion.file}}    "
        rf"{{\texttt{{data/v2/{name_file_esc}\_rescored.json}}}}"
    )
    lines.append(
        rf"\rcset{{companion.hash}}    "
        rf"{{\texttt{{sha256:{rescored_hash[:mid_r]}"
        rf"\allowbreak {rescored_hash[mid_r:]}}}}}"
    )
    lines.append(
        rf"\rcset{{companion.file.analysis}}"
        rf"{{\texttt{{data/v2/{name_file_esc}\_sobol\_analysis.json}}}}"
    )
    lines.append(
        rf"\rcset{{companion.hash.analysis}}"
        rf"{{\texttt{{sha256:{analysis_hash[:mid_a]}"
        rf"\allowbreak {analysis_hash[mid_a:]}}}}}"
    )
    lines.append("")

    # audit seal placeholders — patched by main() after sealing
    lines.append(r"\rcset{audit.chain.tip}   {}")
    lines.append(r"\rcset{audit.chain.seq}   {}")
    lines.append(r"\rcset{audit.signed}      {}")

    return "\n".join(lines) + "\n"


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("rescored", type=Path)
    parser.add_argument("analysis", type=Path)
    parser.add_argument("--output-dir", type=Path, default=Path("data/run-cards-v2"))
    args = parser.parse_args()

    cells: list[dict[str, Any]] = json.loads(args.rescored.read_text())
    analysis: dict[str, Any] = json.loads(args.analysis.read_text())

    eval_name = analysis["eval"]
    model = analysis["model"]
    total_cells = len(cells)
    overall_mean = sum(c["rescore_acc"] for c in cells) / total_cells

    axes = [
        "prompt_template",
        "system_prompt",
        "decoding",
        "choice_order",
        "quantization",
        "n_shot_frac",
    ]

    base_dir = args.output_dir / eval_name.replace("_", "-") / "slices"
    base_dir.mkdir(parents=True, exist_ok=True)

    engine_rel = Path("../../../../../templates/run-card/single-run-card/runcard-v2-slice.tex")

    card_count = 0
    for axis in axes:
        values = sorted(set(c[axis] for c in cells))
        for value in values:
            subset = [c for c in cells if c[axis] == value]
            dir_name = f"{axis}--{slug(value)}"
            card_dir = base_dir / dir_name
            card_dir.mkdir(parents=True, exist_ok=True)

            config = generate_slice_config(
                subset,
                axis,
                value,
                eval_name,
                model,
                total_cells,
                overall_mean,
                axes,
                rescored_path=args.rescored,
                analysis_path=args.analysis,
            )
            config_path = card_dir / "runcard-config.tex"
            config_path.write_text(config)

            run_id_raw = (
                (f"MOJAVE-V2-{eval_name.upper()}-{axis.upper()}-{value.upper()}")
                .replace("=", "")
                .replace(" ", "")
                .replace(".", "")
            )
            seal = audit_seal(
                run_id=run_id_raw,
                eval_name=eval_name,
                data_file=args.rescored,
                actor="generate_slice_cards.py",
            )
            if seal:
                tip = seal["chain_tip_hash"]
                tip_mid = len(tip) // 2
                patched = (
                    config.replace(
                        r"\rcset{audit.chain.tip}   {}",
                        rf"\rcset{{audit.chain.tip}}   "
                        rf"{{\texttt{{{tip[:tip_mid]}"
                        rf"\allowbreak {tip[tip_mid:]}}}}}",
                    )
                    .replace(
                        r"\rcset{audit.chain.seq}   {}",
                        rf"\rcset{{audit.chain.seq}}   {{{seal['chain_tip_seq']}}}",
                    )
                    .replace(
                        r"\rcset{audit.signed}      {}",
                        r"\rcset{audit.signed}      {No --- chain only (unsigned)}",
                    )
                )
                config_path.write_text(patched)

            engine_dst = card_dir / "runcard-v2-slice.tex"
            if engine_dst.exists() or engine_dst.is_symlink():
                engine_dst.unlink()
            engine_dst.symlink_to(engine_rel)

            (card_dir / "Makefile").write_text(
                "TEX = pdflatex -interaction=nonstopmode -halt-on-error\n\n"
                "runcard.pdf: runcard-v2-slice.tex runcard-config.tex\n"
                "\t$(TEX) runcard-v2-slice.tex\n"
                "\t$(TEX) runcard-v2-slice.tex\n"
                "\tcp runcard-v2-slice.pdf runcard.pdf\n\n"
                ".PHONY: clean veryclean\n"
                "clean:\n"
                "\trm -f *.aux *.log *.out *.toc\n"
                "veryclean: clean\n"
                "\trm -f runcard.pdf runcard-v2-slice.pdf\n"
            )
            card_count += 1

    print(f"Generated {card_count} slice cards under {base_dir}/")
    print(f'Build all: for d in {base_dir}/*/; do make -C "$d"; done')


if __name__ == "__main__":
    main()
