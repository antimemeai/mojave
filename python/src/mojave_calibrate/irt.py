from __future__ import annotations

import logging
from datetime import UTC, datetime
from typing import TYPE_CHECKING, Any

if TYPE_CHECKING:
    from pathlib import Path

from py_irt.config import IrtConfig
from py_irt.training import IrtModelTrainer

from mojave_calibrate.protocol import CalibrationResult

logger = logging.getLogger(__name__)


class IrtCalibrator:
    def __init__(
        self,
        model_type: str = "2pl",
        epochs: int = 2000,
        lr: float = 0.1,
        lr_decay: float = 0.9999,
        priors: str = "vague",
        device: str = "cpu",
        seed: int | None = None,
        content_domain: str = "general",
    ) -> None:
        self._model_type = model_type
        self._epochs = epochs
        self._lr = lr
        self._lr_decay = lr_decay
        self._priors = priors
        self._device = device
        self._seed = seed
        self._content_domain = content_domain

    def name(self) -> str:
        return "irt"

    def fit(self, data: Path, **kwargs: Any) -> CalibrationResult:
        config = IrtConfig(
            model_type=self._model_type,
            epochs=self._epochs,
            lr=self._lr,
            lr_decay=self._lr_decay,
            priors=self._priors,
            seed=self._seed,
        )

        trainer = IrtModelTrainer(config=config, data_path=data)
        trainer.train(device=self._device)
        params = trainer.best_params

        item_ids_map: dict[str, str] = params.get("item_ids", {})
        diffs: list[float] = params.get("diff", [])
        discs: list[float] = params.get("disc", [])

        items: list[dict[str, Any]] = []
        n_skipped = 0
        for idx_str, item_id in sorted(item_ids_map.items(), key=lambda kv: int(kv[0])):
            idx = int(idx_str)
            if idx >= len(diffs):
                continue

            disc = discs[idx] if idx < len(discs) else 1.0
            if disc <= 0:
                logger.warning("item '%s': discrimination %.4f <= 0, skipping", item_id, disc)
                n_skipped += 1
                continue

            items.append(
                {
                    "id": item_id,
                    "difficulty": diffs[idx],
                    "discrimination": disc,
                    "content_domain": self._content_domain,
                    "exposure_count": 0,
                }
            )

        if n_skipped:
            logger.info("skipped %d items with non-positive discrimination", n_skipped)

        n_subjects = len(params.get("ability", []))

        metadata: dict[str, Any] = {
            "model": self._model_type,
            "n_items": len(items),
            "n_subjects": n_subjects,
            "timestamp": datetime.now(UTC).isoformat(),
            "package": "py-irt",
            "package_version": _pyirt_version(),
        }

        return CalibrationResult(items=items, factors=None, metadata=metadata)


def _pyirt_version() -> str:
    try:
        from importlib.metadata import version

        return version("py-irt")
    except Exception:
        return "unknown"
