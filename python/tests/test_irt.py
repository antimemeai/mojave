from __future__ import annotations

from mojave_calibrate.irt import IrtCalibrator
from mojave_calibrate.schema import validate_item_pool
from scipy.stats import spearmanr


def test_irt_calibrator_fits_2pl(irt_response_file):
    path, true_params = irt_response_file

    calibrator = IrtCalibrator(
        model_type="2pl",
        epochs=500,
        device="cpu",
        content_domain="test",
    )
    assert calibrator.name() == "irt"

    result = calibrator.fit(path)

    assert result.items is not None
    assert result.factors is None
    assert len(result.items) > 0

    validate_item_pool(result.items, result.metadata)

    recovered = {item["id"]: item for item in result.items}
    true_diffs = []
    est_diffs = []
    for item_id, params in true_params.items():
        if item_id in recovered:
            true_diffs.append(params["b"])
            est_diffs.append(recovered[item_id]["difficulty"])

    if len(true_diffs) >= 5:
        corr, _ = spearmanr(true_diffs, est_diffs)
        assert corr > 0.7, f"difficulty rank correlation {corr:.3f} too low"


def test_irt_calibrator_metadata(irt_response_file):
    path, _ = irt_response_file

    calibrator = IrtCalibrator(
        model_type="2pl",
        epochs=100,
        device="cpu",
        content_domain="general",
    )
    result = calibrator.fit(path)

    assert result.metadata["model"] == "2pl"
    assert result.metadata["package"] == "py-irt"
    assert "n_items" in result.metadata
    assert "n_subjects" in result.metadata
    assert "timestamp" in result.metadata


def test_irt_calibrator_filters_bad_discrimination(irt_response_file):
    path, _ = irt_response_file

    calibrator = IrtCalibrator(
        model_type="2pl",
        epochs=100,
        device="cpu",
        content_domain="test",
    )
    result = calibrator.fit(path)

    for item in result.items:
        assert item["discrimination"] > 0, (
            f"item {item['id']} has discrimination {item['discrimination']}"
        )
