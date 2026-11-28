"""Reference coverage and native-grid image quality measurements for BokehForge."""

from dataclasses import dataclass

import numpy as np

from . import _rust
from ._device import Device, validate_device
from ._score_contract import (
    FAMILY_WEIGHTS, INSPECTION_MAX_MULTIPLIER, INSPECTION_MIX, INSPECTION_SCORE_VERSION, SCORE_DAMAGE_SCALE,
    SCORE_DETAIL_LEVELS, SCORE_EXPLANATION_VERSION, SCORE_VERSION, ScoreDetails,
)

FEATURES = tuple(f"native.{family}" for family in FAMILY_WEIGHTS)


@dataclass(frozen=True)
class Measurement:
    features: dict[str, float]
    diagnostics: dict
    explanation: dict | None = None
    alignment: dict | None = None

    @property
    def score(self):
        return scene_score(self)


def _validate_details(details):
    if details is not None and details not in SCORE_DETAIL_LEVELS:
        raise ValueError(f"Expected score details to be None or one of {SCORE_DETAIL_LEVELS}, got {details!r}")


def validate_rgb(pixels):
    """Check the delivery grid, precision, and finite sample contract."""
    _rust.validate_rgb(pixels)


def measure(candidate, target, *, details: ScoreDetails | None = None, inspection_heatmap=None, attention=True,
            device: Device = "auto"):
    """Score one delivered RGB image against the reference at its native resolution.

    Alignment and per-placement score terms are always available in `.alignment`.
    Request `summary` for damage pooling components or `tiles` for local values.
    Attention is generated from the reference unless disabled. A supplied heatmap
    replaces generation and must contain finite values in [0, 1] on the reference grid.
    Device selection applies to scoring and generated attention.
    """
    return _measure_delivery(
        candidate, target, details=details, device=device, attention=bool(attention),
        heatmap=inspection_heatmap if attention else None,
    )


def _measure_delivery(candidate, target, *, details, device, heatmap=None, generate=None,
                      retry_on_oom=True, attention=False, on_digest=None):
    """Run native scoring after obtaining any caller-supplied inspection map."""
    _validate_details(details)
    validate_device(device)
    if generate is not None:
        heatmap = generate(target)
    if attention and heatmap is None:
        from ._attention_setup import prepare_attention

        prepare_attention()
    features, diagnostics, explanation, alignment, digest = _rust.measure_delivery(
        candidate, target, heatmap, (details, device, attention, retry_on_oom),
    )
    if digest is not None and on_digest is not None:
        on_digest(digest)
    return Measurement(features, diagnostics, explanation, alignment)


def validate_heatmap(values, shape):
    if (not isinstance(values, np.ndarray) or values.shape != shape or values.dtype.kind != "f"
            or not np.isfinite(values).all() or values.min() < 0 or values.max() > 1):
        raise ValueError(f"Expected a finite floating-point inspection map in [0, 1] with shape {shape}")
    return values.astype(np.float32, copy=False)


def _weighted_damage(features):
    return _rust.weighted_damage([(weight, features[f"native.{family}"]) for family, weight in FAMILY_WEIGHTS.items()])


def _placement_total(rows):
    contributions = [row["score_contribution"] for row in rows]
    return _rust.placement_total(contributions) if contributions else 0


def scene_score(measurement):
    """The final reference-fidelity score of a measurement."""
    if measurement.alignment is not None:
        return _placement_total(measurement.alignment["placements"])
    return float(100 * np.exp(-SCORE_DAMAGE_SCALE * _weighted_damage(measurement.features)))
