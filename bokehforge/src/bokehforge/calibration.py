"""Fit or load a frozen proportional-odds model for publication-readiness grades."""

from collections import Counter
from dataclasses import dataclass
from hashlib import sha256
import json
from pathlib import Path

import numpy as np
from scipy.optimize import LinearConstraint, minimize
from scipy.special import log_expit

from .scoring import FEATURES, SCORE_VERSION


@dataclass(frozen=True)
class ScoreCalibration:
    weights: dict[str, float]
    thresholds: tuple[float, float, float, float]

    def __post_init__(self):
        if (set(self.weights) != set(FEATURES)
                or not any(self.weights.values())
                or not all(np.isfinite(value) and 0 <= value <= 1000 for value in self.weights.values())):
            raise ValueError("Calibration requires nonnegative finite weights for every score feature")
        if (len(self.thresholds) != 4
                or not all(np.isfinite(threshold) and abs(threshold) <= 30 for threshold in self.thresholds)
                or not all(a > b for a, b in zip(self.thresholds, self.thresholds[1:]))):
            raise ValueError("Calibration requires four strictly decreasing finite thresholds in [-30, 30]")

    def as_dict(self):
        return {"score_version": SCORE_VERSION, "weights": self.weights, "thresholds": self.thresholds}

    @classmethod
    def load(cls, path):
        payload = Path(path).read_bytes()
        data = json.loads(payload)
        if not isinstance(data, dict) or data.get("score_version") != SCORE_VERSION:
            raise ValueError(f"Calibration must target {SCORE_VERSION}")
        try:
            weights, thresholds = data["weights"], data["thresholds"]
            if (not isinstance(weights, dict) or not isinstance(thresholds, list)
                    or any(type(value) not in {int, float} for value in (*weights.values(), *thresholds))):
                raise ValueError("Calibration weights and thresholds must be numeric")
            model = cls({key: float(weights[key]) for key in FEATURES}, tuple(map(float, thresholds)))
        except (KeyError, TypeError, OverflowError) as exc:
            raise ValueError("Invalid calibration weights or thresholds") from exc
        if weights.keys() != model.weights.keys():
            raise ValueError("Calibration feature names must match the score profile")
        return model, sha256(payload).hexdigest()


def fit_calibration(features, grades, groups, *, regularization=0.01):
    """Fit from blinded grades (0–4) and independent development-group IDs.

    Each group has equal total weight. Use separate held-out groups and stackers
    for qualification before publishing this model as a benchmark profile.
    """
    x = np.asarray([[row[key] for key in FEATURES] for row in features], dtype=np.float64)
    y = np.asarray(grades)
    groups = tuple(groups)
    counts = Counter(groups)
    if (x.shape != (len(groups), len(FEATURES)) or y.shape != (len(groups),)
            or len(counts) < 2 or set(y.tolist()) != set(range(5)) or y.dtype.kind not in "iu"
            or not np.isfinite(x).all() or not ((x >= 0) & (x <= 1)).all()
            or not np.isfinite(regularization) or regularization <= 0):
        raise ValueError("Fit requires bounded score features, all five integer grades, and multiple groups")
    row_weight = np.array([1 / (len(counts) * counts[group]) for group in groups])
    interior = (y > 0) & (y < 4)

    def objective(parameters):
        weights, thresholds = parameters[:-4], parameters[-4:]
        logits = thresholds - (x @ weights)[:, None]
        likelihood = np.empty(len(y))
        likelihood[y == 0] = log_expit(-logits[y == 0, 0])
        likelihood[y == 4] = log_expit(logits[y == 4, -1])
        lower = logits[interior, y[interior]]
        upper = logits[interior, y[interior] - 1]
        # Stable log(sigmoid(upper) - sigmoid(lower)), even near a threshold tie.
        likelihood[interior] = (log_expit(upper) + log_expit(-lower)
                                + np.log(-np.expm1(np.minimum(lower - upper, -1e-12))))
        return -(row_weight @ likelihood) + regularization * (weights @ weights)

    ordered = np.zeros((3, x.shape[1] + 4))
    for index in range(3):
        ordered[index, x.shape[1] + index:x.shape[1] + index + 2] = (1, -1)
    result = minimize(objective, np.r_[np.zeros(x.shape[1]), (3, 2, 1, 0)], method="SLSQP",
                      bounds=[(0.0, 1000.0)] * x.shape[1] + [(-30.0, 30.0)] * 4,
                      constraints=[LinearConstraint(ordered, 0.001, np.inf)])
    if not result.success:
        raise ValueError(f"Calibration fit failed: {result.message}")
    return ScoreCalibration(dict(zip(FEATURES, map(float, result.x[:-4]), strict=True)),
                            tuple(map(float, result.x[-4:])))
