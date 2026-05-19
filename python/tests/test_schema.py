import json

import pytest
from mojave_calibrate.protocol import CalibrationResult
from mojave_calibrate.schema import (
    SchemaError,
    validate_factor_structure,
    validate_item_pool,
    write_factor_structure,
    write_item_pool,
)


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


# --- Item pool validation ---


class TestValidateItemPool:
    def test_valid_item_pool(self):
        items = [
            {
                "id": "task_001",
                "difficulty": 0.5,
                "discrimination": 1.2,
                "content_domain": "math",
                "exposure_count": 0,
            },
            {
                "id": "task_002",
                "difficulty": -1.0,
                "discrimination": 0.8,
                "content_domain": "math",
                "exposure_count": 0,
            },
        ]
        validate_item_pool(items, {"model": "2pl"})

    def test_rejects_zero_discrimination(self):
        items = [
            {
                "id": "bad",
                "difficulty": 0.0,
                "discrimination": 0.0,
                "content_domain": "x",
                "exposure_count": 0,
            }
        ]
        with pytest.raises(SchemaError, match="discrimination"):
            validate_item_pool(items, {})

    def test_rejects_negative_discrimination(self):
        items = [
            {
                "id": "bad",
                "difficulty": 0.0,
                "discrimination": -0.5,
                "content_domain": "x",
                "exposure_count": 0,
            }
        ]
        with pytest.raises(SchemaError, match="discrimination"):
            validate_item_pool(items, {})

    def test_rejects_nan_difficulty(self):
        items = [
            {
                "id": "bad",
                "difficulty": float("nan"),
                "discrimination": 1.0,
                "content_domain": "x",
                "exposure_count": 0,
            }
        ]
        with pytest.raises(SchemaError, match="finite"):
            validate_item_pool(items, {})

    def test_rejects_nan_discrimination(self):
        items = [
            {
                "id": "bad",
                "difficulty": 0.0,
                "discrimination": float("nan"),
                "content_domain": "x",
                "exposure_count": 0,
            }
        ]
        with pytest.raises(SchemaError, match="finite"):
            validate_item_pool(items, {})

    def test_rejects_empty_id(self):
        items = [
            {
                "id": "",
                "difficulty": 0.0,
                "discrimination": 1.0,
                "content_domain": "x",
                "exposure_count": 0,
            }
        ]
        with pytest.raises(SchemaError, match="id"):
            validate_item_pool(items, {})

    def test_rejects_duplicate_ids(self):
        items = [
            {
                "id": "dup",
                "difficulty": 0.0,
                "discrimination": 1.0,
                "content_domain": "x",
                "exposure_count": 0,
            },
            {
                "id": "dup",
                "difficulty": 0.5,
                "discrimination": 1.2,
                "content_domain": "x",
                "exposure_count": 0,
            },
        ]
        with pytest.raises(SchemaError, match="duplicate"):
            validate_item_pool(items, {})

    def test_rejects_empty_items(self):
        with pytest.raises(SchemaError, match="empty"):
            validate_item_pool([], {})


# --- Factor structure validation ---


class TestValidateFactorStructure:
    def test_valid_factor_structure(self):
        factors = {
            "latent_factors": ["f0", "f1"],
            "loadings": [[0.8, 0.1], [0.1, 0.9]],
            "intercepts": [1.0, 0.5],
            "covariance": [[1.0, 0.3], [0.3, 1.0]],
            "fit_indices": {"log_likelihood": -100.0},
        }
        validate_factor_structure(factors, {})

    def test_rejects_mismatched_loadings_rows(self):
        factors = {
            "latent_factors": ["f0", "f1"],
            "loadings": [[0.8, 0.1]],
            "intercepts": [1.0, 0.5],
            "covariance": [[1.0, 0.3], [0.3, 1.0]],
            "fit_indices": {},
        }
        with pytest.raises(SchemaError, match="intercept"):
            validate_factor_structure(factors, {})

    def test_rejects_mismatched_loadings_cols(self):
        factors = {
            "latent_factors": ["f0", "f1"],
            "loadings": [[0.8, 0.1, 0.0], [0.1, 0.9, 0.0]],
            "intercepts": [1.0, 0.5],
            "covariance": [[1.0, 0.3], [0.3, 1.0]],
            "fit_indices": {},
        }
        with pytest.raises(SchemaError, match="latent_factors"):
            validate_factor_structure(factors, {})


# --- Write helpers ---


class TestWriteItemPool:
    def test_roundtrip(self, tmp_path):
        items = [
            {
                "id": "t1",
                "difficulty": 0.5,
                "discrimination": 1.0,
                "content_domain": "math",
                "exposure_count": 0,
            }
        ]
        metadata = {"model": "2pl"}
        out = tmp_path / "pool.json"
        write_item_pool(items, metadata, out)

        with open(out) as f:
            data = json.load(f)

        assert len(data["items"]) == 1
        assert data["items"][0]["id"] == "t1"
        assert data["calibration_metadata"]["model"] == "2pl"


class TestWriteFactorStructure:
    def test_roundtrip(self, tmp_path):
        factors = {
            "latent_factors": ["f0"],
            "loadings": [[0.8]],
            "intercepts": [1.0],
            "covariance": [[1.0]],
            "fit_indices": {"CFI": 0.95},
        }
        metadata = {"model_type": "grm"}
        out = tmp_path / "factors.json"
        write_factor_structure(factors, metadata, out)

        with open(out) as f:
            data = json.load(f)

        assert data["latent_factors"] == ["f0"]
        assert data["calibration_metadata"]["model_type"] == "grm"
