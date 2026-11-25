"""Match a delivered crop to its reference without fitting away shape errors."""

from dataclasses import dataclass
from math import ceil, cos, radians, sin

import numpy as np
from scipy import ndimage as ndi
from scipy import optimize, signal

from .scoring import _measure, scene_score, validate_rgb

CROP_SCORE_VERSION = "reference-fidelity-crop-v1"
MAX_ROTATION_DEGREES = 3.0
_SEARCH_SIDE = 384
_MAX_HYPOTHESES = 3
_MIN_AGREEMENT = 0.20
_FULL_AGREEMENT = 0.65
_AMBIGUITY_MARGIN = 0.05


@dataclass(frozen=True)
class _Placement:
    top: float
    left: float
    rotation: float
    agreement: float


def _texture(rgb, step=1):
    # Match brightness patterns only. The original RGB, including its range
    # and color errors, still goes into the scorer.
    light = np.einsum("ijk,k->ij", np.clip(rgb, 0, 1),
                      np.array([0.2126, 0.7152, 0.0722], dtype=np.float32))
    if step > 1:
        light = ndi.gaussian_filter(light, step / 2, mode="reflect")[::step, ::step]
    return np.clip(light - ndi.gaussian_filter(light, 1.3, mode="reflect"), -0.08, 0.08)


def _coarse_agreement(reference, crop, angle):
    patch = ndi.rotate(crop, angle, reshape=False, order=1, prefilter=False)
    mask = np.ones(crop.shape, dtype=np.float32)
    if min(crop.shape) > 8:
        mask[:2] = mask[-2:] = 0
        mask[:, :2] = mask[:, -2:] = 0
    mask = ndi.rotate(mask, angle, reshape=False, order=0, prefilter=False)
    count = mask.sum(dtype=np.float64)
    if count < 3:
        return None
    template = (patch - (patch * mask).sum(dtype=np.float64) / count) * mask
    energy = np.square(template).sum(dtype=np.float64)
    if energy <= count * 1e-10:
        return None

    def correlate(image, kernel):
        return signal.fftconvolve(image, kernel[::-1, ::-1], mode="valid")

    total = correlate(reference, mask)
    variance = correlate(np.square(reference), mask) - np.square(total) / count
    return np.divide(correlate(reference, template), np.sqrt(energy * np.maximum(variance, 1e-10)),
                     out=np.full(variance.shape, -1.0), where=variance > count * 1e-10).clip(-1, 1)


def _coarse_placements(candidate, target, step, limits):
    crop, reference = _texture(candidate, step), _texture(target, step)
    peaks = []
    for angle in np.linspace(-MAX_ROTATION_DEGREES, MAX_ROTATION_DEGREES, 9):
        agreement = _coarse_agreement(reference, crop, angle)
        if agreement is None:
            continue
        for _ in range(_MAX_HYPOTHESES):
            y, x = np.unravel_index(np.argmax(agreement), agreement.shape)
            if agreement[y, x] <= 0:
                break
            peaks.append(_Placement(float(min(y * step, limits[0])),
                                    float(min(x * step, limits[1])),
                                    float(angle), float(agreement[y, x])))
            agreement[max(0, y - 2):y + 3, max(0, x - 2):x + 3] = -1
    return _distinct(peaks, max(2, step * 2))


def _distinct(placements, distance):
    chosen = []
    for fit in sorted(placements, key=lambda p: (-p.agreement, abs(p.rotation), p.top, p.left)):
        if all(max(abs(fit.top - other.top), abs(fit.left - other.left)) > distance
               for other in chosen):
            chosen.append(fit)
            if len(chosen) == _MAX_HYPOTHESES:
                break
    return chosen


def _correlation(a, b):
    a, b = a - a.mean(), b - b.mean()
    energy = np.sqrt(np.dot(a, a) * np.dot(b, b))
    return float(np.clip(np.dot(a, b) / energy, -1, 1)) if energy > len(a) * 1e-10 else -1.0


def _agreement_function(crop, reference, dimensions, step):
    center = (np.asarray(dimensions) - 1) / 2
    stride = max(1, ceil(np.sqrt(crop.size / 6000)))
    margin = np.minimum(np.array(crop.shape) // 5, 5)
    yy, xx = np.mgrid[margin[0]:crop.shape[0] - margin[0]:stride,
                      margin[1]:crop.shape[1] - margin[1]:stride]
    values = crop[yy, xx].ravel()
    yy = yy.ravel() * step - center[0]
    xx = xx.ravel() * step - center[1]

    def agreement(top, left, rotation):
        rad = radians(rotation)
        ry = (cos(rad) * yy - sin(rad) * xx + center[0] + top) / step
        rx = (sin(rad) * yy + cos(rad) * xx + center[1] + left) / step
        valid = ((ry >= 0) & (ry <= reference.shape[0] - 1)
                 & (rx >= 0) & (rx <= reference.shape[1] - 1))
        if valid.sum() < 3:
            return -1.0
        sampled = ndi.map_coordinates(reference, (ry[valid], rx[valid]), order=1, prefilter=False)
        return _correlation(values[valid], sampled)

    return agreement


def _refine(placement, agreement, limits, step, radius):
    # Decimation can move a sharp correlation peak by a few pixels. Search
    # the nearby grid before continuous refinement, which can miss such peaks.
    def nearby(origin, limit):
        center = round(origin / step) * step
        reach = min(4, ceil(radius / step))
        return sorted({origin, *(min(limit, max(0, center + i * step))
                                  for i in range(-reach, reach + 1))})

    choices = ((y, x, angle)
               for y in nearby(placement.top, limits[0])
               for x in nearby(placement.left, limits[1])
               for angle in dict.fromkeys((placement.rotation, 0.0)))
    initial = max(choices, key=lambda p: (agreement(*p), -abs(p[2])))
    lower = (0, 0, -MAX_ROTATION_DEGREES)
    upper = (*limits, MAX_ROTATION_DEGREES)
    bounds = [(max(lo, v - delta), min(hi, v + delta))
              for v, lo, hi, delta in zip(initial, lower, upper, (radius, radius, 0.85), strict=True)]
    free = [i for i, (lo, hi) in enumerate(bounds) if hi > lo]

    def parameters(values):
        result = list(initial)
        for index, value in zip(free, values, strict=True):
            result[index] = float(value)
        return result

    def objective(values):
        top, left, rotation = parameters(values)
        return -agreement(top, left, rotation)

    fitted = optimize.minimize(objective, [initial[i] for i in free], method="Powell",
                               bounds=[bounds[i] for i in free],
                               options={"xtol": 0.015, "ftol": 1e-5, "maxfev": 140})
    refined = parameters(fitted.x)
    # Preserve exact pixel placement when the rounded transform has the same
    # evidence. This avoids a second interpolation for an ordinary crop.
    rounded = (round(refined[0]), round(refined[1]), round(refined[2], 2))
    candidates = (initial, refined, rounded, (*rounded[:2], 0.0))
    return max((_Placement(*p, agreement(*p)) for p in candidates),
               key=lambda p: (round(p.agreement, 7), -abs(p.rotation),
                              -abs(p.top - round(p.top)) - abs(p.left - round(p.left))))


def _placements(candidate, target, limits):
    step = max(1, ceil(max(target.shape[:2]) / _SEARCH_SIDE))
    placements = _coarse_placements(candidate, target, step, limits)
    if not placements:
        return []
    middle = ceil(step / 4)
    levels = (middle, 1) if middle > 1 else (1,)
    for level, radius in zip(levels, (step * 1.75, middle * 1.75)):
        agreement = _agreement_function(_texture(candidate, level), _texture(target, level),
                                        candidate.shape[:2], level)
        placements = _distinct([_refine(p, agreement, limits, level, radius) for p in placements],
                               max(2, level))
    return placements


def _project(candidate, shape, placement):
    """Sample the delivered image on the fixed reference grid and mark real support."""
    crop_shape = candidate.shape[:2]
    if placement.rotation == 0 and all(float(v).is_integer() for v in (placement.top, placement.left)):
        y, x = int(placement.top), int(placement.left)
        region = np.s_[y:y + crop_shape[0], x:x + crop_shape[1]]
        aligned = np.zeros(shape, dtype=np.float32)
        support = np.zeros(shape[:2], dtype=bool)
        aligned[region], support[region] = candidate, True
        return aligned, support

    rad = radians(placement.rotation)
    inverse = np.array([[cos(rad), sin(rad)], [-sin(rad), cos(rad)]])
    center = (np.asarray(crop_shape) - 1) / 2
    offset = center - inverse @ (center + [placement.top, placement.left])
    support = ndi.affine_transform(np.ones(crop_shape, dtype=bool), inverse, offset,
                                   output_shape=shape[:2], order=0, mode="grid-constant",
                                   cval=False, prefilter=False)
    aligned = np.empty(shape, dtype=np.float32)
    for channel in range(3):
        ndi.affine_transform(candidate[..., channel], inverse, offset, output=aligned[..., channel],
                             order=1, mode="nearest", prefilter=False)
    return aligned, support


def _placed_score(candidate, target, placement):
    if candidate.shape == target.shape and (placement.top, placement.left, placement.rotation) == (0, 0, 0):
        measurement = _measure(candidate, target)
        return measurement, scene_score(measurement), 1.0
    aligned, support = _project(candidate, target.shape, placement)
    coverage = min(int(support.sum()), candidate.shape[0] * candidate.shape[1]) / support.size
    if not coverage:
        return None, 0.0, 0.0
    measurement = _measure(aligned, target, None if support.all() else support)
    return measurement, scene_score(measurement), coverage


def evaluate_crop(candidate, target):
    """Return scene score, leading measurement, and image-inferred alignment details."""
    validate_rgb(candidate)
    validate_rgb(target)
    limits = tuple(t - c for t, c in zip(target.shape[:2], candidate.shape[:2], strict=True))
    identity = _Placement(0.0, 0.0, 0.0, 1.0)
    if min(limits) < 0:
        return 0.0, None, {"status": "unmatched", "reason": "output_larger_than_reference",
                           "confidence": 0.0, "placements": []}

    if limits == (0, 0):
        # The reference fixes translation at full size. Apply roll only when
        # the image evidence substantially favors it over that known placement.
        placements = [] if np.array_equal(candidate, target) else _placements(candidate, target, limits)
        if placements and placements[0].rotation:
            agreement = _agreement_function(_texture(candidate), _texture(target), candidate.shape[:2], 1)
            best = placements[0]
            if best.agreement < max(_FULL_AGREEMENT, agreement(0, 0, 0) + 0.03):
                placements = []
        else:
            placements = []
        placements, confidence = placements or [identity], 1.0
    elif np.all(target == target[0, 0]):
        # Every placement sees the same target. Center the output deterministically.
        placements = [_Placement(limits[0] // 2, limits[1] // 2, 0.0, 1.0)]
        confidence = 1.0
    else:
        placements = _placements(candidate, target, limits)
        best = placements[0].agreement if placements else -1.0
        confidence = float(np.clip((best - _MIN_AGREEMENT) / (_FULL_AGREEMENT - _MIN_AGREEMENT), 0, 1))
        if not confidence:
            return 0.0, None, {"status": "unmatched", "reason": "weak_image_agreement",
                               "confidence": confidence, "placements": []}

    best = placements[0].agreement
    weights = [max(0.0, 1 - (best - p.agreement) / _AMBIGUITY_MARGIN) ** 2 for p in placements]
    total_weight = sum(weights)
    report, score, leading = [], 0.0, None
    for placement, weight in zip(placements, weights, strict=True):
        if not weight:
            continue
        measurement, quality, coverage = _placed_score(candidate, target, placement)
        share = confidence * weight / total_weight
        score += share * quality * coverage
        if not report:
            leading = measurement
        report.append({"offset_yx": [placement.top, placement.left],
                       "rotation_degrees": placement.rotation, "agreement": placement.agreement,
                       "coverage": coverage, "overlap_score": quality, "weight": share})
    return score, leading, {"status": "matched", "confidence": confidence, "placements": report}
