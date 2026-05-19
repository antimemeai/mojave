from __future__ import annotations

import json

import pytest
from click.testing import CliRunner
from mojave_calibrate.cli import main


@pytest.fixture()
def runner():
    return CliRunner()


class TestIrtSubcommand:
    def test_produces_valid_json(self, runner, irt_response_file, tmp_path):
        input_path, _ = irt_response_file
        output_path = tmp_path / "out.json"

        result = runner.invoke(
            main,
            [
                "irt",
                "--input",
                str(input_path),
                "--output",
                str(output_path),
                "--model-type",
                "2pl",
                "--epochs",
                "100",
                "--device",
                "cpu",
                "--content-domain",
                "test",
            ],
        )

        assert result.exit_code == 0, f"stderr: {result.output}"
        assert output_path.exists()

        with open(output_path) as f:
            data = json.load(f)

        assert "items" in data
        assert "calibration_metadata" in data
        assert len(data["items"]) > 0

    def test_missing_input_exits_nonzero(self, runner, tmp_path):
        result = runner.invoke(
            main,
            [
                "irt",
                "--input",
                str(tmp_path / "nonexistent.jsonl"),
                "--output",
                str(tmp_path / "out.json"),
                "--content-domain",
                "test",
                "--device",
                "cpu",
            ],
        )
        assert result.exit_code != 0


class TestFactorsSubcommand:
    def test_produces_valid_json(self, runner, factor_response_file, tmp_path):
        input_path, _ = factor_response_file
        output_path = tmp_path / "out.json"

        result = runner.invoke(
            main,
            [
                "factors",
                "--input",
                str(input_path),
                "--output",
                str(output_path),
                "--latent-size",
                "3",
                "--model-type",
                "grm",
                "--n-cats",
                "3",
                "--device",
                "cpu",
                "--max-epochs",
                "50",
                "--iw-samples",
                "5",
            ],
        )

        assert result.exit_code == 0, f"stderr: {result.output}"
        assert output_path.exists()

        with open(output_path) as f:
            data = json.load(f)

        assert "latent_factors" in data
        assert "calibration_metadata" in data


class TestCfaSubcommand:
    def test_produces_valid_json(self, runner, cfa_data_file, tmp_path):
        input_path, model_spec = cfa_data_file
        output_path = tmp_path / "out.json"

        result = runner.invoke(
            main,
            [
                "cfa",
                "--input",
                str(input_path),
                "--output",
                str(output_path),
                "--model",
                model_spec,
            ],
        )

        assert result.exit_code == 0, f"stderr: {result.output}"
        assert output_path.exists()

        with open(output_path) as f:
            data = json.load(f)

        assert "latent_factors" in data
        assert "fit_indices" in data

    def test_model_from_file(self, runner, cfa_data_file, tmp_path):
        input_path, model_spec = cfa_data_file
        model_file = tmp_path / "spec.sem"
        model_file.write_text(model_spec)
        output_path = tmp_path / "out.json"

        result = runner.invoke(
            main,
            [
                "cfa",
                "--input",
                str(input_path),
                "--output",
                str(output_path),
                "--model-file",
                str(model_file),
            ],
        )

        assert result.exit_code == 0, f"stderr: {result.output}"


class TestVerboseFlag:
    def test_verbose_produces_output(self, runner, cfa_data_file, tmp_path):
        input_path, model_spec = cfa_data_file
        output_path = tmp_path / "out.json"

        result = runner.invoke(
            main,
            [
                "--verbose",
                "cfa",
                "--input",
                str(input_path),
                "--output",
                str(output_path),
                "--model",
                model_spec,
            ],
        )

        assert result.exit_code == 0
