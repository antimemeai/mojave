from __future__ import annotations

from mojave_calibrate.factors import FactorCalibrator
from mojave_calibrate.schema import validate_factor_structure


def test_factor_calibrator_fits_grm(factor_response_file):
    path, true_loadings = factor_response_file

    calibrator = FactorCalibrator(
        latent_size=3,
        model_type="grm",
        n_cats=3,
        device="cpu",
        max_epochs=200,
        iw_samples=5,
    )
    assert calibrator.name() == "factors"

    result = calibrator.fit(path)

    assert result.items is None
    assert result.factors is not None

    factors = result.factors
    validate_factor_structure(factors, result.metadata)

    assert len(factors["latent_factors"]) == 3
    assert len(factors["loadings"]) == 12
    assert len(factors["loadings"][0]) == 3
    assert len(factors["covariance"]) == 3


def test_factor_calibrator_custom_names(factor_response_file):
    path, _ = factor_response_file

    calibrator = FactorCalibrator(
        latent_size=3,
        model_type="grm",
        n_cats=3,
        device="cpu",
        max_epochs=50,
        iw_samples=5,
        factor_names=["reasoning", "code", "retrieval"],
    )
    result = calibrator.fit(path)
    assert result.factors["latent_factors"] == ["reasoning", "code", "retrieval"]


def test_factor_calibrator_metadata(factor_response_file):
    path, _ = factor_response_file

    calibrator = FactorCalibrator(
        latent_size=3,
        model_type="grm",
        n_cats=3,
        device="cpu",
        max_epochs=50,
        iw_samples=5,
    )
    result = calibrator.fit(path)

    assert result.metadata["model_type"] == "grm"
    assert result.metadata["latent_size"] == 3
    assert result.metadata["package"] == "deepirtools"
    assert "n_subjects" in result.metadata
    assert "n_items" in result.metadata
    assert "timestamp" in result.metadata
