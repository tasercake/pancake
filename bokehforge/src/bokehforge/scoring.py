"""Fixed-coordinate, full-reference image quality measurements for BokehForge."""

from dataclasses import dataclass

import numpy as np
from scipy import ndimage as ndi
from scipy.special import expit

SCORE_VERSION = "reference-readiness-v4"
FAMILY_WEIGHTS = {
    "tone": 0.17, "color": 0.16, "lost_detail": 0.18,
    "added_structure": 0.15, "boundary": 0.23, "range": 0.11,
}
VIEW_WEIGHTS = {"fit": 0.30, "native": 0.50, "zoom": 0.20}
FEATURES = tuple(f"{view}.{family}" for view in VIEW_WEIGHTS for family in FAMILY_WEIGHTS)
_VIEW_SIZE = 640
_CROP_SIZE = 96
_ZOOM = 2
# Linear sRGB to D65 XYZ, normalized by the D65 white point.
_XYZ = np.array([
    [0.4124564, 0.3575761, 0.1804375],
    [0.2126729, 0.7151522, 0.0721750],
    [0.0193339, 0.1191920, 0.9503041],
], dtype=np.float32) / np.array([0.95047, 1, 1.08883], dtype=np.float32)[:, None]


@dataclass(frozen=True)
class Measurement:
    features: dict[str, float]
    diagnostics: dict


def validate_rgb(pixels, shape=None):
    """Check the delivery grid, precision, and finite sample contract."""
    if (not isinstance(pixels, np.ndarray) or pixels.dtype.kind != "f" or pixels.dtype.itemsize != 4
            or pixels.ndim != 3 or pixels.shape[2] != 3 or not all(pixels.shape)
            or (shape is not None and pixels.shape != shape) or not np.isfinite(pixels).all()):
        grid = "HxWx3" if shape is None else str(shape)
        raise ValueError(f"Expected finite float32 linear-sRGB RGB on the {grid} grid")


def _lab(rgb):
    # The display clips at SDR black and white. Native range evidence is measured separately.
    rgb = np.clip(rgb, 0, 1)
    xyz = np.stack([rgb[..., 0] * x + rgb[..., 1] * y + rgb[..., 2] * z for x, y, z in _XYZ])
    curve = np.where(xyz > (6 / 29) ** 3, np.cbrt(xyz), xyz * (841 / 108) + 4 / 29)
    x, y, z = curve
    return 116 * y - 16, 500 * (x - y), 200 * (y - z)


def _blur(image, sigma):
    return ndi.gaussian_filter(image, sigma, mode="reflect")


def _visibility(error, scale):
    # Continuous soft response. Increasing discrepancy remains visible at every contrast.
    return error / (error + scale)


def _orientation(image, dy, dx):
    """Adjacent directional difference, including two-pixel-period texture."""
    return (image[dy:, dx:] - image[:image.shape[0] - dy, :image.shape[1] - dx]) / np.hypot(dy, dx)


def _detail(candidate, target):
    lost, extra = np.zeros_like(target), np.zeros_like(target)
    for sigma in (0, 1.1, 2.5):
        c = _blur(candidate, sigma) if sigma else candidate
        t = _blur(target, sigma) if sigma else target
        for dy, dx in ((0, 1), (1, 0), (1, 1), (-1, 1)):
            # Flipping the rows lets the same forward operator cover both diagonals.
            cs, ts = (c[::-1], t[::-1]) if dy < 0 else (c, t)
            cg, tg = _orientation(cs, abs(dy), dx), _orientation(ts, abs(dy), dx)
            matching = cg * tg > 0
            earned = np.where(matching, np.minimum(abs(cg), abs(tg)), 0)
            losses = _visibility(abs(tg) - earned, 5.0)
            gains = _visibility(abs(cg) - earned, 5.0)
            for destination, damage in ((lost, losses), (extra, gains)):
                dest = destination[::-1] if dy < 0 else destination
                first = dest[:dest.shape[0] - abs(dy), :dest.shape[1] - dx]
                second = dest[abs(dy):, dx:]
                np.maximum(first, damage, out=first)
                np.maximum(second, damage, out=second)
    return lost, extra


def _maps(candidate, target):
    cl, ca, cb = _lab(candidate)
    tl, ta, tb = _lab(target)
    delta = cl - tl
    local_delta = _blur(delta, 2.5)
    # Tone carries broad gain, contrast, local lighting, and seam differences.
    tone = _visibility(np.maximum(abs(local_delta), abs(_blur(delta, 10))), 8.0)
    # CIELAB opponent-plane distance keeps a cast visible alongside a tone error.
    color = _visibility(np.hypot(ca - ta, cb - tb), 10.0)

    lost, extra = _detail(cl, tl)
    upper = ndi.maximum_filter(tl, size=3, mode="reflect")
    lower = ndi.minimum_filter(tl, size=3, mode="reflect")
    edge_weight = _visibility(upper - lower, 6.0)
    near_edge = ndi.maximum_filter(edge_weight, size=9, mode="reflect")
    # Residual grain is useful evidence in smooth backgrounds. At an existing
    # transition, the signed detail bank and edge neighborhood carry the signal.
    grain = _visibility(abs(delta - _blur(delta, 0.85)), 5.0) * (1 - edge_weight)
    extra = np.maximum(extra, grain)
    shape = _visibility(abs(delta - local_delta), 6.0) * near_edge
    overshoot = _visibility(np.maximum.reduce([cl - upper, lower - cl, np.zeros_like(cl)]), 6.0)
    boundary = np.maximum(shape, overshoot * near_edge)

    # Work in float64 before subtraction so all finite float32 samples stay finite.
    c64, t64 = candidate.astype(np.float64), target.astype(np.float64)
    c_display, t_display = np.clip(c64, 0, 1), np.clip(t64, 0, 1)
    headroom = abs((c64 - c_display) - (t64 - t_display))
    # Detect flattening to an SDR endpoint relative to the expected sample.
    at_endpoint = 1 - np.clip(np.minimum(c_display, 1 - c_display) / 0.012, 0, 1)
    clipping = abs(c_display - t_display) * at_endpoint
    range_error = _visibility(np.maximum(headroom, clipping).max(axis=2), 0.075).astype(np.float32)
    c_exposure = np.maximum(c64.mean(axis=(0, 1)), 0) @ _XYZ[1]
    t_exposure = np.maximum(t64.mean(axis=(0, 1)), 0) @ _XYZ[1]
    return (tone, color, lost, extra, boundary, range_error), {
        "mean_delta_lightness": float(delta.mean(dtype=np.float64)),
        "global_exposure_delta_stops": float(np.log2((c_exposure + 1e-5) / (t_exposure + 1e-5))),
        "mean_delta_lab_a": float(np.mean(ca - ta, dtype=np.float64)),
        "mean_delta_lab_b": float(np.mean(cb - tb, dtype=np.float64)),
    }


def _axis_tiles(length, side, offset):
    edges = np.arange(offset, length, side, dtype=int)
    edges = np.unique(np.r_[0, edges, length])
    return edges[:-1], edges[1:]


def _pool(damage):
    """Pool typical damage and supported defects on four overlapping tile phases."""
    mean = float(damage.mean(dtype=np.float64))
    integral = np.pad(damage.cumsum(0, dtype=np.float64).cumsum(1), ((1, 0), (1, 0)))
    worst, tail, region = 0.0, 0.0, None
    height, width = damage.shape
    for side in (8, 32):
        for y_offset, x_offset in ((0, 0), (0, side // 2), (side // 2, 0), (side // 2, side // 2)):
            y0, y1 = _axis_tiles(height, side, y_offset)
            x0, x1 = _axis_tiles(width, side, x_offset)
            totals = (integral[np.ix_(y1, x1)] - integral[np.ix_(y0, x1)]
                      - integral[np.ix_(y1, x0)] + integral[np.ix_(y0, x0)])
            # A small edge tile needs the same supporting area as an interior tile.
            tiles = np.clip(totals / (min(side, height) * min(side, width)), 0, 1)
            iy, ix = np.unravel_index(np.argmax(tiles), tiles.shape)
            if tiles[iy, ix] > worst:
                worst = float(tiles[iy, ix])
                region = [int(x0[ix]), int(y0[iy]), int(x1[ix]), int(y1[iy])]
            # A phase partitions the image, so its tail never counts the same pixel twice.
            flat = tiles.ravel()
            count = max(1, int(np.ceil(0.01 * flat.size)))
            tail = max(tail, float(np.partition(flat, flat.size - count)[-count:].mean()))
    return 0.40 * mean + 0.35 * tail + 0.25 * worst, {
        "mean": mean, "tail": tail, "worst": worst, "worst_box_view_xyxy": region,
        "affected_fraction": float(np.mean(damage >= 0.25)),
    }


def _fit(rgb):
    height, width = rgb.shape[:2]
    scale = min(1, _VIEW_SIZE / max(height, width))
    if scale == 1:
        return rgb
    shape = max(1, round(height * scale)), max(1, round(width * scale))
    filtered = _blur(rgb, (0.5 / scale, 0.5 / scale, 0))
    return ndi.zoom(filtered, (shape[0] / height, shape[1] / width, 1), order=1,
                    grid_mode=True, mode="reflect")


def _crops(height, width):
    h, w = min(height, _CROP_SIZE), min(width, _CROP_SIZE)
    origins = ((0, 0), (0, width - w), (height - h, 0),
               (height - h, width - w), ((height - h) // 2, (width - w) // 2))
    return [(x, y, x + w, y + h) for y, x in dict.fromkeys(origins)]


def _zoom(rgb):
    return ndi.zoom(rgb, (_ZOOM, _ZOOM, 1), order=1, grid_mode=True, mode="reflect")


def _view(candidate, target, box):
    maps, signed = _maps(candidate, target)
    rows = {}
    x, y, right, bottom = box
    sx, sy = (right - x) / target.shape[1], (bottom - y) / target.shape[0]
    for name, damage in zip(FAMILY_WEIGHTS, maps, strict=True):
        burden, summary = _pool(damage)
        if local := summary["worst_box_view_xyxy"]:
            x0, y0, x1, y1 = local
            summary["worst_box_native_xyxy"] = [x + x0 * sx, y + y0 * sy, x + x1 * sx, y + y1 * sy]
        rows[name] = {"burden": burden, **summary}
    return {"native_box_xyxy": box, "view_size_wh": target.shape[1::-1], "families": rows, **signed}


def measure(candidate, target):
    """Compare two canonical float32 linear-sRGB images on the target grid."""
    validate_rgb(target)
    validate_rgb(candidate, target.shape)
    height, width = target.shape[:2]
    box = (0, 0, width, height)
    # Bound interpolation and filtering, including on extreme but finite input.
    # All native out-of-SDR samples still participate in the range measurement.
    fit_candidate, fit_target = (np.clip(image, -8, 8) for image in (candidate, target))
    fit = _view(_fit(fit_candidate), _fit(fit_target), box)
    native = _view(candidate, target, box)
    zoom = [
        _view(_zoom(fit_candidate[y:bottom, x:right]), _zoom(fit_target[y:bottom, x:right]),
              (x, y, right, bottom))
        for x, y, right, bottom in _crops(height, width)
    ]
    views = {"fit": [fit], "native": [native], "zoom": zoom}
    features = {}
    for view, records in views.items():
        for family in FAMILY_WEIGHTS:
            values = [record["families"][family]["burden"] for record in records]
            features[f"{view}.{family}"] = (max(values) + sum(values) / len(values)) / 2
    rmse = float(np.sqrt(np.square(np.subtract(candidate, target, dtype=np.float64)).mean()))
    return Measurement(features, {"views": views, "linear_rgb_rmse": rmse})


def scene_score(measurement, calibration=None):
    """Return the only optimization target, a bounded 0–100 scene score."""
    features = measurement.features
    if not any(features.values()):
        return 100.0
    if calibration is not None:
        damage = sum(weight * features[key] for key, weight in calibration.weights.items())
        probabilities = expit(np.asarray(calibration.thresholds) - damage)
        # Anchor a perfect delivery without adding a discontinuity at zero damage.
        return float(100 * probabilities.sum() / expit(calibration.thresholds).sum())
    damage = sum(v * w * features[f"{view}.{family}"]
                 for view, v in VIEW_WEIGHTS.items() for family, w in FAMILY_WEIGHTS.items())
    return float(100 * np.exp(-5 * damage))
