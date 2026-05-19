from mojave_calibrate.protocol import CalibrationResult


def test_calibration_result_is_frozen():
    result = CalibrationResult(
        items=[{"id": "i1", "difficulty": 0.5, "discrimination": 1.0}],
        factors=None,
        metadata={"model": "2pl"},
    )
    assert result.items is not None
    assert result.factors is None
    assert result.metadata["model"] == "2pl"

    try:
        result.items = []  # type: ignore[misc]
        msg = "should have raised FrozenInstanceError"
        raise AssertionError(msg)
    except AttributeError:
        pass


def test_calibration_result_factors_only():
    result = CalibrationResult(
        items=None,
        factors={"latent_factors": ["f0"], "loadings": [[0.8]]},
        metadata={"model_type": "grm"},
    )
    assert result.items is None
    assert result.factors["latent_factors"] == ["f0"]
