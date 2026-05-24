#!/usr/bin/env python3
"""Generate per-cell run cards for every Saltelli matrix cell.

Produces one 1-page PDF per cell with perturbation config, accuracy,
and companion data hash. Builds are parallelized via xargs.

Usage:
    python generate_cell_cards.py <rescored.json> <manifest.json> \
        [--output-dir data/run-cards-v2]
"""

from __future__ import annotations

import argparse
import json
import subprocess
import time
from pathlib import Path
from typing import Any

from repo import audit_seal, compute_file_sha256


def fmt(v: float, decimals: int = 4) -> str:
    return f"{v:.{decimals}f}"


def tex_escape(s: str) -> str:
    return s.replace("_", r"\_").replace("&", r"\&").replace("%", r"\%")


def generate_cell_config(
    cell: dict[str, Any],
    manifest_cell: dict[str, Any],
    eval_name: str,
    model: str,
    overall_mean: float,
    rescored_hash: str,
    rescored_path_escaped: str,
) -> str:
    cell_id = cell["cell_id"]
    acc = cell["rescore_acc"]
    correct = cell["rescore_correct"]
    total = cell["total"]
    diff = acc - overall_mean
    sign = "+" if diff >= 0 else ""
    saltelli_idx = manifest_cell.get("saltelli_index", "")

    eval_esc = tex_escape(eval_name)
    model_esc = tex_escape(model)
    run_id = tex_escape(f"MOJAVE-V2-{eval_name.upper()}-{cell_id.upper()}")
    parent_id = tex_escape(f"MOJAVE-V2-{eval_name.upper()}")

    mid = len(rescored_hash) // 2

    lines: list[str] = []
    lines.append(rf"\rcset{{run.id}}            {{{run_id}}}")
    lines.append(rf"\rcset{{parent.run.id}}     {{{parent_id}}}")
    lines.append(rf"\rcset{{date.issued}}       {{{time.strftime('%Y-%m-%d')}}}")
    lines.append(rf"\rcset{{benchmark.name}}    {{{eval_esc}}}")
    lines.append(r"\rcset{benchmark.version} {inspect\_evals 0.12.0}")
    lines.append(rf"\rcset{{model.name}}        {{{model_esc}}}")
    lines.append(rf"\rcset{{model.quant}}       {{{tex_escape(cell['quantization'])}}}")
    lines.append("")
    lines.append(rf"\rcset{{cell.id}}           {{{cell_id}}}")
    lines.append(rf"\rcset{{cell.saltelli.idx}} {{{saltelli_idx}}}")
    lines.append(rf"\rcset{{cell.n.items}}      {{{total}}}")
    lines.append(rf"\rcset{{cell.correct}}      {{{correct}}}")
    lines.append(rf"\rcset{{cell.acc}}          {{{fmt(acc)}}}")
    lines.append(
        rf"\rcset{{cell.vs.overall}}   {{{sign}{fmt(diff)}pp "
        rf"vs.\ overall {fmt(overall_mean)}}}"
    )
    lines.append("")

    for axis in [
        "prompt_template",
        "system_prompt",
        "decoding",
        "choice_order",
        "quantization",
        "n_shot_frac",
    ]:
        val = tex_escape(str(cell.get(axis, "")))
        lines.append(rf"\rcset{{axis.{axis}}}     {{{val}}}")
    lines.append("")

    lines.append(rf"\rcset{{companion.file}}    {{{rescored_path_escaped}}}")
    lines.append(
        rf"\rcset{{companion.hash}}    "
        rf"{{\texttt{{sha256:{rescored_hash[:mid]}"
        rf"\allowbreak {rescored_hash[mid:]}}}}}"
    )
    lines.append("")
    lines.append(r"\rcset{audit.chain.tip}   {}")
    lines.append(r"\rcset{audit.chain.seq}   {}")

    return "\n".join(lines) + "\n"


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("rescored", type=Path)
    parser.add_argument("manifest", type=Path)
    parser.add_argument("--output-dir", type=Path, default=Path("data/run-cards-v2"))
    parser.add_argument(
        "--jobs", "-j", type=int, default=8, help="Parallel pdflatex jobs (default: 8)"
    )
    parser.add_argument(
        "--no-build", action="store_true", help="Generate configs only, skip PDF build"
    )
    args = parser.parse_args()

    cells: list[dict[str, Any]] = json.loads(args.rescored.read_text())
    manifest: dict[str, Any] = json.loads(args.manifest.read_text())

    eval_name = manifest.get("eval", manifest.get("eval_name", "unknown"))
    if eval_name == "unknown":
        for c in manifest.get("cells", []):
            if "eval" in c:
                eval_name = c["eval"]
                break
    # try to get from analysis
    if eval_name == "unknown":
        eval_name = cells[0].get("eval", "unknown") if cells else "unknown"

    # get eval name from the rescored filename as fallback
    if eval_name == "unknown":
        stem = args.rescored.stem
        if "bio" in stem:
            eval_name = "wmdp_bio"
        elif "chem" in stem:
            eval_name = "wmdp_chem"
        elif "truthful" in stem:
            eval_name = "truthfulqa_mc1"

    model = manifest.get("model", "Qwen/Qwen2.5-7B-Instruct")
    overall_mean = sum(c["rescore_acc"] for c in cells) / len(cells)

    manifest_idx = {c["cell_id"]: c for c in manifest["cells"]}

    rescored_hash = compute_file_sha256(args.rescored)
    rescored_path_esc = tex_escape(str(args.rescored))

    base_dir = args.output_dir / eval_name.replace("_", "-") / "mass"
    base_dir.mkdir(parents=True, exist_ok=True)

    engine_rel = Path("../../../../../templates/run-card/single-run-card/runcard-v2-cell.tex")

    print(f"Generating {len(cells)} cell card configs...")
    build_dirs: list[str] = []

    for cell in cells:
        cid = cell["cell_id"]
        card_dir = base_dir / cid
        card_dir.mkdir(parents=True, exist_ok=True)

        mcell = manifest_idx.get(cid, {})
        config = generate_cell_config(
            cell,
            mcell,
            eval_name,
            model,
            overall_mean,
            rescored_hash,
            rescored_path_esc,
        )
        config_path = card_dir / "runcard-config.tex"
        config_path.write_text(config)

        run_id_raw = f"MOJAVE-V2-{eval_name.upper()}-{cid.upper()}"
        seal = audit_seal(
            run_id=run_id_raw,
            eval_name=eval_name,
            data_file=args.rescored,
            actor="generate_cell_cards.py",
        )
        if seal:
            tip = seal["chain_tip_hash"]
            tip_mid = len(tip) // 2
            patched = config.replace(
                r"\rcset{audit.chain.tip}   {}",
                rf"\rcset{{audit.chain.tip}}   "
                rf"{{\texttt{{{tip[:tip_mid]}"
                rf"\allowbreak {tip[tip_mid:]}}}}}",
            ).replace(
                r"\rcset{audit.chain.seq}   {}",
                rf"\rcset{{audit.chain.seq}}   {{{seal['chain_tip_seq']}}}",
            )
            config_path.write_text(patched)

        engine_dst = card_dir / "runcard-v2-cell.tex"
        if engine_dst.exists() or engine_dst.is_symlink():
            engine_dst.unlink()
        engine_dst.symlink_to(engine_rel)

        (card_dir / "Makefile").write_text(
            "TEX = pdflatex -interaction=nonstopmode -halt-on-error\n\n"
            "runcard.pdf: runcard-v2-cell.tex runcard-config.tex\n"
            "\t$(TEX) runcard-v2-cell.tex\n"
            "\t$(TEX) runcard-v2-cell.tex\n"
            "\tcp runcard-v2-cell.pdf runcard.pdf\n\n"
            ".PHONY: clean veryclean\n"
            "clean:\n"
            "\trm -f *.aux *.log *.out *.toc\n"
            "veryclean: clean\n"
            "\trm -f runcard.pdf runcard-v2-cell.pdf\n"
        )
        build_dirs.append(str(card_dir))

    print(f"  {len(build_dirs)} configs written to {base_dir}/")

    if args.no_build:
        print("  --no-build: skipping PDF generation")
        return

    print(f"  Building PDFs with {args.jobs} parallel jobs...")
    dirs_file = base_dir / "_build_dirs.txt"
    dirs_file.write_text("\n".join(build_dirs) + "\n")

    with open(dirs_file) as f:
        result = subprocess.run(
            ["xargs", "-P", str(args.jobs), "-I", "{}", "make", "-C", "{}", "-s"],
            stdin=f,
            capture_output=True,
            text=True,
        )

    # count successes
    pdfs = sum(1 for d in build_dirs if (Path(d) / "runcard.pdf").exists())
    print(f"  Built {pdfs}/{len(build_dirs)} PDFs")
    if result.returncode != 0 and pdfs < len(build_dirs):
        print(f"  WARN: {len(build_dirs) - pdfs} builds failed")
        print(result.stderr[:500] if result.stderr else "")

    dirs_file.unlink(missing_ok=True)


if __name__ == "__main__":
    main()
