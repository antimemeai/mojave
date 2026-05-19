from __future__ import annotations

import logging
import sys
from pathlib import Path

import click

from mojave_calibrate.schema import write_factor_structure, write_item_pool

logger = logging.getLogger("mojave_calibrate")


@click.group()
@click.option("--verbose", is_flag=True, help="Enable debug logging to stderr.")
def main(verbose: bool) -> None:
    """mojave-calibrate: offline calibration pipeline for the mojave measurement engine."""
    level = logging.DEBUG if verbose else logging.WARNING
    logging.basicConfig(
        level=level,
        format="%(name)s %(levelname)s: %(message)s",
        stream=sys.stderr,
    )


@main.command()
@click.option("--input", "input_path", required=True, type=click.Path(exists=True, path_type=Path))
@click.option("--output", "output_path", required=True, type=click.Path(path_type=Path))
@click.option("--model-type", default="2pl", type=click.Choice(["1pl", "2pl", "4pl"]))
@click.option("--epochs", default=2000, type=int)
@click.option("--lr", default=0.1, type=float)
@click.option("--lr-decay", default=0.9999, type=float)
@click.option("--priors", default="vague", type=click.Choice(["vague", "hierarchical"]))
@click.option("--device", default="cpu", type=str)
@click.option("--seed", default=None, type=int)
@click.option("--content-domain", required=True, type=str)
def irt(
    input_path: Path,
    output_path: Path,
    model_type: str,
    epochs: int,
    lr: float,
    lr_decay: float,
    priors: str,
    device: str,
    seed: int | None,
    content_domain: str,
) -> None:
    """Fit IRT model via py-irt and emit item pool JSON."""
    from mojave_calibrate.irt import IrtCalibrator

    try:
        calibrator = IrtCalibrator(
            model_type=model_type,
            epochs=epochs,
            lr=lr,
            lr_decay=lr_decay,
            priors=priors,
            device=device,
            seed=seed,
            content_domain=content_domain,
        )
        result = calibrator.fit(input_path)
        assert result.items is not None
        write_item_pool(result.items, result.metadata, output_path)
        logger.info("wrote item pool to %s", output_path)
    except Exception as exc:
        logger.error("IRT calibration failed: %s", exc)
        raise SystemExit(2) from exc


@main.command()
@click.option("--input", "input_path", required=True, type=click.Path(exists=True, path_type=Path))
@click.option("--output", "output_path", required=True, type=click.Path(path_type=Path))
@click.option("--latent-size", required=True, type=int)
@click.option("--model-type", default="grm", type=click.Choice(["grm", "gpcm", "nominal"]))
@click.option("--n-cats", default=3, type=int)
@click.option("--q-matrix", default=None, type=click.Path(exists=True, path_type=Path))
@click.option("--device", default="cpu", type=str)
@click.option("--max-epochs", default=100_000, type=int)
@click.option("--iw-samples", default=5000, type=int)
@click.option("--seed", default=None, type=int)
@click.option("--factor-names", default=None, type=str, help="Comma-separated factor names.")
def factors(
    input_path: Path,
    output_path: Path,
    latent_size: int,
    model_type: str,
    n_cats: int,
    q_matrix: Path | None,
    device: str,
    max_epochs: int,
    iw_samples: int,
    seed: int | None,
    factor_names: str | None,
) -> None:
    """Fit factor model via deepirtools IWAVE and emit factor structure JSON."""
    from mojave_calibrate.factors import FactorCalibrator

    try:
        names_list = factor_names.split(",") if factor_names else None
        calibrator = FactorCalibrator(
            latent_size=latent_size,
            model_type=model_type,
            n_cats=n_cats,
            q_matrix_path=q_matrix,
            device=device,
            max_epochs=max_epochs,
            iw_samples=iw_samples,
            factor_names=names_list,
        )
        result = calibrator.fit(input_path)
        assert result.factors is not None
        write_factor_structure(result.factors, result.metadata, output_path)
        logger.info("wrote factor structure to %s", output_path)
    except Exception as exc:
        logger.error("factor calibration failed: %s", exc)
        raise SystemExit(2) from exc


@main.command()
@click.option("--input", "input_path", required=True, type=click.Path(exists=True, path_type=Path))
@click.option("--output", "output_path", required=True, type=click.Path(path_type=Path))
@click.option("--model", "model_spec", default=None, type=str)
@click.option("--model-file", default=None, type=click.Path(exists=True, path_type=Path))
@click.option(
    "--objective", default="MLW", type=click.Choice(["MLW", "FIML", "ULS", "GLS", "WLS", "DWLS"])
)
def cfa(
    input_path: Path,
    output_path: Path,
    model_spec: str | None,
    model_file: Path | None,
    objective: str,
) -> None:
    """Fit CFA/SEM model via semopy and emit factor structure JSON."""
    from mojave_calibrate.cfa import CfaCalibrator

    try:
        calibrator = CfaCalibrator(
            model=model_spec,
            model_file=model_file,
            objective=objective,
        )
        result = calibrator.fit(input_path)
        assert result.factors is not None
        write_factor_structure(result.factors, result.metadata, output_path)
        logger.info("wrote factor structure to %s", output_path)
    except Exception as exc:
        logger.error("CFA calibration failed: %s", exc)
        raise SystemExit(2) from exc
