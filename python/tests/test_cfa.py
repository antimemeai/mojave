from __future__ import annotations

import pytest
from mojave_calibrate.cfa import CfaCalibrator
from mojave_calibrate.schema import validate_factor_structure


def test_cfa_calibrator_fits_model(cfa_data_file):
    path, model_spec = cfa_data_file

    calibrator = CfaCalibrator(model=model_spec, objective="MLW")
    assert calibrator.name() == "cfa"

    result = calibrator.fit(path)

    assert result.items is None
    assert result.factors is not None

    factors = result.factors
    validate_factor_structure(factors, result.metadata)

    assert factors["latent_factors"] == ["f1", "f2", "f3"]
    assert len(factors["loadings"]) == 9
    assert len(factors["loadings"][0]) == 3

    fi = factors["fit_indices"]
    assert "CFI" in fi
    assert "RMSEA" in fi


def test_cfa_calibrator_model_from_file(cfa_data_file, tmp_path):
    path, model_spec = cfa_data_file

    model_file = tmp_path / "model.sem"
    model_file.write_text(model_spec)

    calibrator = CfaCalibrator(model_file=model_file, objective="MLW")
    result = calibrator.fit(path)

    assert result.factors is not None
    assert result.factors["latent_factors"] == ["f1", "f2", "f3"]


def test_cfa_calibrator_metadata(cfa_data_file):
    path, model_spec = cfa_data_file

    calibrator = CfaCalibrator(model=model_spec, objective="MLW")
    result = calibrator.fit(path)

    assert result.metadata["package"] == "semopy"
    assert result.metadata["objective"] == "MLW"
    assert "n_subjects" in result.metadata
    assert "timestamp" in result.metadata


def test_cfa_calibrator_rejects_no_model():
    with pytest.raises(ValueError, match="model"):
        CfaCalibrator(objective="MLW")
