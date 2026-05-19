from __future__ import annotations

from dataclasses import dataclass
from typing import TYPE_CHECKING, Any, Protocol

if TYPE_CHECKING:
    from pathlib import Path


@dataclass(frozen=True)
class CalibrationResult:
    items: list[dict[str, Any]] | None
    factors: dict[str, Any] | None
    metadata: dict[str, Any]


class Calibrator(Protocol):
    def fit(self, data: Path, **kwargs: Any) -> CalibrationResult: ...

    def name(self) -> str: ...
