#!/usr/bin/env python3
"""Generate a parquet index of all run cards (aggregate + per-variant).

Output: data/run-cards/card-index.parquet
"""

from __future__ import annotations

import json
import math
from pathlib import Path

import pyarrow as pa
import pyarrow.parquet as pq

SPACES_BASE = "https://antimeme-ai-site.nyc3.digitaloceanspaces.com/mojave/run-cards"

EVALS = {
    "arc_challenge": {
        "results": "data/arc-workup/results.json",
        "display": "ARC Challenge",
        "slug": "arc-challenge",
    },
    "cybermetric_2000": {
        "results": "data/cybermetric/results.json",
        "display": "CyberMetric-2000",
        "slug": "cybermetric-2000",
    },
    "mmlu_0_shot": {
        "results": "data/mmlu/results.json",
        "display": "MMLU (0-shot)",
        "slug": "mmlu-0-shot",
    },
    "hellaswag": {
        "results": "data/hellaswag/results.json",
        "display": "HellaSwag",
        "slug": "hellaswag",
    },
    "truthfulqa": {
        "results": "data/truthfulqa/results.json",
        "display": "TruthfulQA",
        "slug": "truthfulqa",
    },
    "gsm8k": {
        "results": "data/gsm8k/results.json",
        "display": "GSM8K",
        "slug": "gsm8k",
    },
}

ANALYSIS = json.loads(Path("data/analysis/full_summary.json").read_text())

TIER_THRESHOLDS = {"strong": 2.0, "middling": 10.0}


def wilson_ci(k: int, n: int, z: float = 1.96) -> tuple[float, float]:
    if n == 0:
        return 0.0, 0.0
    p_hat = k / n
    denom = 1 + z * z / n
    center = (p_hat + z * z / (2 * n)) / denom
    margin = z * math.sqrt(p_hat * (1 - p_hat) / n + z * z / (4 * n * n)) / denom
    return max(0.0, center - margin), min(1.0, center + margin)


def tier_for(sensitive_pct: float) -> str:
    if sensitive_pct < TIER_THRESHOLDS["strong"]:
        return "strong"
    elif sensitive_pct < TIER_THRESHOLDS["middling"]:
        return "middling"
    return "fragile"


def main() -> None:
    rows: list[dict[str, object]] = []

    for eval_name, meta in EVALS.items():
        results_path = Path(meta["results"])
        if not results_path.exists():
            continue

        data = json.loads(results_path.read_text())
        slug = meta["slug"]
        analysis = ANALYSIS.get(eval_name, {})
        stab = analysis.get("perturbation_stability", {})
        sensitive_pct = stab.get("pct_sensitive", 0.0)

        agg = analysis.get("aggregate", {})
        rows.append(
            {
                "eval": meta["display"],
                "variant_id": "aggregate",
                "card_type": "aggregate",
                "accuracy": agg.get("pooled_accuracy"),
                "ci_lo": agg.get("wilson_ci_95", [None, None])[0],
                "ci_hi": agg.get("wilson_ci_95", [None, None])[1],
                "temperature": None,
                "order_seed": None,
                "n_samples": agg.get("pooled_n"),
                "n_variants": analysis.get("n_variants"),
                "sensitive_pct": sensitive_pct,
                "tier": tier_for(sensitive_pct),
                "spread": agg.get("variant_spread"),
                "pdf_url": f"{SPACES_BASE}/{slug}-runcard.pdf",
            }
        )

        for v in data["variants"]:
            if v["accuracy"] is None:
                continue
            n = v["n_samples"]
            k = round(v["accuracy"] * n) if n > 0 else 0
            lo, hi = wilson_ci(k, n)
            rows.append(
                {
                    "eval": meta["display"],
                    "variant_id": v["variant_id"],
                    "card_type": "variant",
                    "accuracy": v["accuracy"],
                    "ci_lo": lo,
                    "ci_hi": hi,
                    "temperature": v.get("temperature"),
                    "order_seed": v.get("order_seed"),
                    "n_samples": n,
                    "n_variants": None,
                    "sensitive_pct": None,
                    "tier": None,
                    "spread": None,
                    "pdf_url": f"{SPACES_BASE}/variants/{slug}-{v['variant_id']}.pdf",
                }
            )

    rows.append(
        {
            "eval": "Cross-Eval Summary",
            "variant_id": "aggregate",
            "card_type": "cross-eval",
            "accuracy": None,
            "ci_lo": None,
            "ci_hi": None,
            "temperature": None,
            "order_seed": None,
            "n_samples": None,
            "n_variants": None,
            "sensitive_pct": None,
            "tier": None,
            "spread": None,
            "pdf_url": f"{SPACES_BASE}/cross-eval-summary.pdf",
        }
    )

    schema = pa.schema(
        [
            ("eval", pa.string()),
            ("variant_id", pa.string()),
            ("card_type", pa.string()),
            ("accuracy", pa.float64()),
            ("ci_lo", pa.float64()),
            ("ci_hi", pa.float64()),
            ("temperature", pa.float64()),
            ("order_seed", pa.int32()),
            ("n_samples", pa.int32()),
            ("n_variants", pa.int32()),
            ("sensitive_pct", pa.float64()),
            ("tier", pa.string()),
            ("spread", pa.float64()),
            ("pdf_url", pa.string()),
        ]
    )

    arrays = {col.name: [r[col.name] for r in rows] for col in schema}
    table = pa.table(arrays, schema=schema)

    out = Path("data/run-cards/card-index.parquet")
    pq.write_table(table, out, compression="zstd")
    print(f"Written {len(rows)} rows to {out} ({out.stat().st_size / 1024:.1f} KB)")


if __name__ == "__main__":
    main()
