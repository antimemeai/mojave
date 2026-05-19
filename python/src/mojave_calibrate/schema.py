from __future__ import annotations

import json
import math
from typing import TYPE_CHECKING, Any

if TYPE_CHECKING:
    from pathlib import Path


class SchemaError(Exception):
    pass


def validate_item_pool(items: list[dict[str, Any]], metadata: dict[str, Any]) -> None:
    if not items:
        raise SchemaError("items list must not be empty")

    seen_ids: set[str] = set()
    for i, item in enumerate(items):
        item_id = item.get("id", "")
        if not item_id:
            raise SchemaError(f"item {i}: id must be non-empty")
        if item_id in seen_ids:
            raise SchemaError(f"item {i}: duplicate id '{item_id}'")
        seen_ids.add(item_id)

        diff = item.get("difficulty", 0.0)
        disc = item.get("discrimination", 0.0)

        if not math.isfinite(diff):
            raise SchemaError(f"item '{item_id}': difficulty must be finite, got {diff}")
        if not math.isfinite(disc):
            raise SchemaError(f"item '{item_id}': discrimination must be finite, got {disc}")
        if disc <= 0:
            raise SchemaError(f"item '{item_id}': discrimination must be > 0, got {disc}")


def validate_factor_structure(factors: dict[str, Any], metadata: dict[str, Any]) -> None:
    latent = factors.get("latent_factors", [])
    loadings = factors.get("loadings", [])
    intercepts = factors.get("intercepts", [])
    covariance = factors.get("covariance", [])

    n_factors = len(latent)
    n_items = len(intercepts)

    if loadings and n_items != len(loadings):
        raise SchemaError(
            f"loadings has {len(loadings)} rows but intercepts has {n_items} "
            f"entries — row count must match"
        )

    if loadings and loadings[0]:
        cols = len(loadings[0])
        if cols != n_factors:
            raise SchemaError(
                f"loadings has {cols} columns but latent_factors has "
                f"{n_factors} entries — column count must match"
            )

    if covariance and len(covariance) != n_factors:
        raise SchemaError(
            f"covariance is {len(covariance)}x{len(covariance)} but expected "
            f"{n_factors}x{n_factors}"
        )


def write_item_pool(
    items: list[dict[str, Any]],
    metadata: dict[str, Any],
    output: Path,
) -> None:
    validate_item_pool(items, metadata)
    doc = {"items": items, "calibration_metadata": metadata}
    output.write_text(json.dumps(doc, indent=2))


def write_factor_structure(
    factors: dict[str, Any],
    metadata: dict[str, Any],
    output: Path,
) -> None:
    validate_factor_structure(factors, metadata)
    doc = {**factors, "calibration_metadata": metadata}
    output.write_text(json.dumps(doc, indent=2))
