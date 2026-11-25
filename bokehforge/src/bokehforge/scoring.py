"""Fixed-coordinate, full-reference image quality measurements for BokehForge."""

from dataclasses import dataclass
from typing import Literal

import numpy as np
from scipy import ndimage as ndi

SCORE_VERSION = "reference-fidelity-v2"
SCORE_DAMAGE_SCALE = 5
FAMILY_WEIGHTS = {
    "tone": 0.17, "color": 0.16, "lost_detail": 0.18,
    "added_structure": 0.15, "boundary": 0.23, "range": 0.11,
}
FEATURES = tuple(f"native.{family}" for family in FAMILY_WEIGHTS)
SCORE_EXPLANATION_VERSION = 1
SCORE_DETAIL_LEVELS = ("summary", "tiles")
ScoreDetails = Literal["summary", "tiles"]
_POOL_WEIGHTS = {"mean": 0.40, "tail": 0.35, "worst": 0.25}
_POOL_LABELS = {"mean": "Average damage", "tail": "Average of the worst 1% of tiles",
                "worst": "Worst single tile"}
_TAIL_FRACTION = 0.01
_AFFECTED_THRESHOLD = 0.25
_FAMILY_LABELS = {
    "tone": ("Tone", "Brightness and contrast differences"),
    "color": ("Color", "Differences in tint or color intensity"),
    "lost_detail": ("Lost detail", "Local contrast missing from the delivery"),
    "added_structure": ("Added structure", "Excess local contrast and fine variation"),
    "boundary": ("Boundary", "Shape and brightness differences near reference edges"),
    "range": ("Range", "Changes beyond display black or white and near clipping"),
}
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
    explanation: dict | None = None

    @property
    def score(self):
        return scene_score(self)


def _validate_details(details):
    if details is not None and details not in SCORE_DETAIL_LEVELS:
        raise ValueError(f"Expected score details to be None, 'summary', or 'tiles', got {details!r}")


def validate_rgb(pixels, shape=None):
    """Check the delivery grid, precision, and finite sample contract."""
    if not isinstance(pixels, np.ndarray):
        raise ValueError("The benchmark requires an RGB image array.")
    if pixels.ndim != 3 or pixels.shape[2] != 3 or not all(pixels.shape):
        raise ValueError(f"The benchmark requires a nonempty RGB image with three channels. "
                         f"The image has array shape {pixels.shape}.")
    if shape is not None and pixels.shape != shape:
        raise ValueError(f"The image is {pixels.shape[1]} × {pixels.shape[0]} pixels. The benchmark requires "
                         f"the reference's native dimensions of {shape[1]} × {shape[0]} pixels.")
    if pixels.dtype.kind != "f" or pixels.dtype.itemsize != 4:
        raise ValueError(f"The image uses {pixels.dtype} samples. "
                         "The benchmark requires 32-bit floating-point linear-sRGB samples.")
    if not np.isfinite(pixels).all():
        raise ValueError("The image contains non-finite pixel values (NaN or infinity).")


def _lab(rgb):
    # The display clips at SDR black and white. Native range evidence is measured separately.
    rgb = np.clip(rgb, 0, 1)
    xyz = np.stack([rgb[..., 0] * x + rgb[..., 1] * y + rgb[..., 2] * z for x, y, z in _XYZ])
    curve = np.where(xyz > (6 / 29) ** 3, np.cbrt(xyz), xyz * (841 / 108) + 4 / 29)
    x, y, z = curve
    return 116 * y - 16, 500 * (x - y), 200 * (y - z)


def _blur(image, sigma):
    return ndi.gaussian_filter(image, sigma, mode="reflect")


def _supported_blur(support):
    if support is None:
        return _blur
    weights = support.astype(np.float32)
    denominators = {}

    def blur(image, sigma):
        if sigma not in denominators:
            denominators[sigma] = _blur(weights, sigma)
        weight = denominators[sigma]
        return np.divide(_blur(image * weights, sigma), weight,
                         out=np.zeros_like(image), where=weight > 0)

    return blur


def _supported_mean(values, support, axis=None):
    if support is None:
        return values.mean(axis=axis, dtype=np.float64)
    valid = support[..., None] if values.ndim == 3 else support
    return values.mean(axis=axis, dtype=np.float64, where=valid)


def _visibility(error, scale):
    # Continuous soft response. Increasing discrepancy remains visible at every contrast.
    return error / (error + scale)


def _orientation(image, dy, dx):
    """Adjacent directional difference, including two-pixel-period texture."""
    return (image[dy:, dx:] - image[:image.shape[0] - dy, :image.shape[1] - dx]) / np.hypot(dy, dx)


def _detail(candidate, target, blur, support):
    lost, extra = np.zeros_like(target), np.zeros_like(target)
    for sigma in (0, 1.1, 2.5):
        c = blur(candidate, sigma) if sigma else candidate
        t = blur(target, sigma) if sigma else target
        for dy, dx in ((0, 1), (1, 0), (1, 1), (-1, 1)):
            # Flipping the rows lets the same forward operator cover both diagonals.
            cs, ts = (c[::-1], t[::-1]) if dy < 0 else (c, t)
            cg, tg = _orientation(cs, abs(dy), dx), _orientation(ts, abs(dy), dx)
            matching = cg * tg > 0
            earned = np.where(matching, np.minimum(abs(cg), abs(tg)), 0)
            losses = _visibility(abs(tg) - earned, 5.0)
            gains = _visibility(abs(cg) - earned, 5.0)
            if support is not None:
                valid = support[::-1] if dy < 0 else support
                adjacent = valid[abs(dy):, dx:] & valid[:valid.shape[0] - abs(dy), :valid.shape[1] - dx]
                losses *= adjacent
                gains *= adjacent
            for destination, damage in ((lost, losses), (extra, gains)):
                dest = destination[::-1] if dy < 0 else destination
                first = dest[:dest.shape[0] - abs(dy), :dest.shape[1] - dx]
                second = dest[abs(dy):, dx:]
                np.maximum(first, damage, out=first)
                np.maximum(second, damage, out=second)
    return lost, extra


def _maps(candidate, target, support=None):
    cl, ca, cb = _lab(candidate)
    tl, ta, tb = _lab(target)
    delta = cl - tl
    blur = _supported_blur(support)
    local_delta = blur(delta, 2.5)
    # Tone carries broad gain, contrast, local lighting, and seam differences.
    tone = _visibility(np.maximum(abs(local_delta), abs(blur(delta, 10))), 8.0)
    # CIELAB opponent-plane distance keeps a cast visible alongside a tone error.
    color = _visibility(np.hypot(ca - ta, cb - tb), 10.0)

    lost, extra = _detail(cl, tl, blur, support)
    upper = ndi.maximum_filter(tl, size=3, mode="reflect")
    lower = ndi.minimum_filter(tl, size=3, mode="reflect")
    edge_weight = _visibility(upper - lower, 6.0)
    near_edge = ndi.maximum_filter(edge_weight, size=9, mode="reflect")
    # Residual grain is useful evidence in smooth backgrounds. At an existing
    # transition, the signed detail bank and edge neighborhood carry the signal.
    grain = _visibility(abs(delta - blur(delta, 0.85)), 5.0) * (1 - edge_weight)
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
    c_exposure = np.maximum(_supported_mean(c64, support, axis=(0, 1)), 0) @ _XYZ[1]
    t_exposure = np.maximum(_supported_mean(t64, support, axis=(0, 1)), 0) @ _XYZ[1]
    return (tone, color, lost, extra, boundary, range_error), {
        "mean_delta_lightness": float(_supported_mean(delta, support)),
        "global_exposure_delta_stops": float(np.log2((c_exposure + 1e-5) / (t_exposure + 1e-5))),
        "mean_delta_lab_a": float(_supported_mean(ca - ta, support)),
        "mean_delta_lab_b": float(_supported_mean(cb - tb, support)),
    }


def _axis_tiles(length, side, offset):
    edges = np.arange(offset, length, side, dtype=int)
    return np.unique(np.r_[0, edges, length])


def _integral(image):
    return np.pad(image.cumsum(0, dtype=np.float64).cumsum(1), ((1, 0), (1, 0)))


def _tile_totals(integral, y0, y1, x0, x1):
    return (integral[np.ix_(y1, x1)] - integral[np.ix_(y0, x1)]
            - integral[np.ix_(y1, x0)] + integral[np.ix_(y0, x0)])


@dataclass(frozen=True)
class _TileGrid:
    side: int
    offset_yx: tuple[int, int]
    x: np.ndarray
    y: np.ndarray
    nominal_pixels: int
    covered: np.ndarray | None

    @property
    def id(self):
        return f"{self.side}:{self.offset_yx[0]}:{self.offset_yx[1]}"

    def totals(self, integral):
        return _tile_totals(integral, self.y[:-1], self.y[1:], self.x[:-1], self.x[1:])

    def box(self, index):
        iy, ix = divmod(index, self.x.size - 1)
        return [int(self.x[ix]), int(self.y[iy]), int(self.x[ix + 1]), int(self.y[iy + 1])]

    def describe(self):
        return {"grid_id": self.id, "tile_size": self.side, "offset_yx": list(self.offset_yx)}

    def describe_tiles(self):
        return {**self.describe(), "x_edges": self.x.tolist(), "y_edges": self.y.tolist(),
                "nominal_pixels": self.nominal_pixels,
                "covered_pixels": None if self.covered is None else self.covered.ravel().tolist(),
                "values": {}}


def _tile_grids(height, width, support):
    support_integral = None if support is None else _integral(support)
    grids = []
    for side in (8, 32):
        for y_offset, x_offset in ((0, 0), (0, side // 2), (side // 2, 0), (side // 2, side // 2)):
            y = _axis_tiles(height, side, y_offset)
            x = _axis_tiles(width, side, x_offset)
            covered = None if support_integral is None else _tile_totals(
                support_integral, y[:-1], y[1:], x[:-1], x[1:]).astype(np.int32)
            grids.append(_TileGrid(side, (y_offset, x_offset), x, y, min(side, height) * min(side, width), covered))
    return grids


def _pool(damage, support, grids, *, explain=False, tiles_by_grid=None, family=None):
    """Compute the metric and optionally retain the evidence from those same tiles."""
    mean = float(_supported_mean(damage, support))
    integral = _integral(damage if support is None else damage * support)
    worst, tail, region = 0.0, 0.0, None
    worst_tile, tail_selection, winning_tiles = None, None, None
    for grid in grids:
        # Even a short edge tile uses the full nominal area as its denominator.
        tiles = np.clip(grid.totals(integral) / grid.nominal_pixels, 0, 1).ravel()
        index = int(np.argmax(tiles))
        if tiles[index] > worst:
            worst = float(tiles[index])
            region = grid.box(index)
            if explain:
                worst_tile = {**grid.describe(), "tile_index": index, "box_xyxy": region}
        # A grid partitions the image, so its tail never counts the same pixel twice.
        eligible = tiles if grid.covered is None else tiles[grid.covered.ravel() > 0]
        count = max(1, int(np.ceil(_TAIL_FRACTION * eligible.size)))
        top = np.partition(eligible, eligible.size - count)[-count:]
        tile_tail = float(top.mean())
        if tile_tail > tail or (explain and tail_selection is None):
            tail = tile_tail
            if explain:
                tail_selection = {**grid.describe(), "tile_count": count, "eligible_tile_count": int(eligible.size),
                                  "cutoff": float(top[0])}
                if tiles_by_grid is not None:
                    winning_tiles = (grid, tiles)
        if tiles_by_grid is not None:
            tiles_by_grid[grid.id]["values"][family] = tiles.tolist()

    values = {"mean": mean, "tail": tail, "worst": worst}
    contributions = {key: _POOL_WEIGHTS[key] * value for key, value in values.items()}
    summary = {
        "mean": mean, "tail": tail, "worst": worst, "worst_box_view_xyxy": region,
        "affected_fraction": float(_supported_mean(damage >= _AFFECTED_THRESHOLD, support)),
    }
    components = None
    if explain:
        components = {key: {"label": _POOL_LABELS[key], "value": value, "weight": _POOL_WEIGHTS[key],
                            "contribution": contributions[key]} for key, value in values.items()}
        components["worst"].update(worst_tile or dict.fromkeys(
            ("grid_id", "tile_size", "offset_yx", "tile_index", "box_xyxy")))
        components["tail"].update(tail_selection)
        components["tail"]["fraction"] = _TAIL_FRACTION
        if winning_tiles is not None:
            grid, tiles = winning_tiles
            cutoff = tail_selection["cutoff"]
            above = np.flatnonzero(tiles > cutoff)
            ties = tiles == cutoff
            if grid.covered is not None:
                ties &= grid.covered.ravel() > 0
            indices = np.flatnonzero(ties)[:tail_selection["tile_count"] - above.size]
            components["tail"]["tile_indices"] = np.sort(np.r_[above, indices]).tolist()
    burden = contributions["mean"] + contributions["tail"] + contributions["worst"]
    return burden, summary, components


def _measure(candidate, target, support=None, *, details=None):
    height, width = target.shape[:2]
    maps, signed = _maps(candidate, target, support)
    grids = _tile_grids(height, width, support)
    tile_data = {grid.id: grid.describe_tiles() for grid in grids} if details == "tiles" else None
    features = {}
    rows = {}
    families = {}
    for (name, weight), damage in zip(FAMILY_WEIGHTS.items(), maps, strict=True):
        burden, summary, components = _pool(damage, support, grids, explain=details is not None,
                                             tiles_by_grid=tile_data, family=name)
        if region := summary["worst_box_view_xyxy"]:
            summary["worst_box_native_xyxy"] = region
        features[f"native.{name}"] = burden
        rows[name] = {"burden": burden, **summary}
        if components is not None:
            label, description = _FAMILY_LABELS[name]
            families[name] = {"label": label, "description": description, "weight": weight, "burden": burden,
                              "weighted_damage": weight * burden, "pool": components,
                              "affected_fraction": summary["affected_fraction"]}
    native = {"native_box_xyxy": (0, 0, width, height), "view_size_wh": (width, height),
              "families": rows, **signed}
    if support is not None:
        native["covered_fraction"] = float(support.mean())
    rmse = float(np.sqrt(_supported_mean(np.square(np.subtract(candidate, target, dtype=np.float64)), support)))
    explanation = None
    if details is not None:
        explanation = {
            "reference_size_wh": [width, height], "damage_range": [0, 1], "affected_threshold": _AFFECTED_THRESHOLD,
            "weighted_damage": _weighted_damage(features), "families": families,
            "measured_pixels": height * width if support is None else int(support.sum()),
            "diagnostics": {**signed, "linear_rgb_rmse": rmse},
        }
        if tile_data is not None:
            explanation["tile_grids"] = tile_data
    return Measurement(features, {"views": {"native": [native]}, "linear_rgb_rmse": rmse}, explanation)


def measure(candidate, target, *, details: ScoreDetails | None = None):
    """Compare two canonical float32 linear-sRGB images on the target grid.

    The result has a scalar `.score`. Request `summary` for weighted components,
    or `tiles` for the exact grids of patch values used to pool those components.
    """
    _validate_details(details)
    validate_rgb(target)
    validate_rgb(candidate, target.shape)
    return _measure(candidate, target, details=details)


def _weighted_damage(features):
    return sum(weight * features[f"native.{family}"] for family, weight in FAMILY_WEIGHTS.items())


def scene_score(measurement):
    """Apply the fixed reference-fidelity profile to the measured damage."""
    return float(100 * np.exp(-SCORE_DAMAGE_SCALE * _weighted_damage(measurement.features)))


def _score_explanation(profile, details, candidate, target, score, confidence, placements):
    return {
        "schema_version": SCORE_EXPLANATION_VERSION, "profile": profile, "detail_level": details, "score": score,
        "damage_scale": SCORE_DAMAGE_SCALE, "reference_size_wh": list(target.shape[1::-1]),
        "candidate_size_wh": list(candidate.shape[1::-1]), "confidence": confidence, "placements": placements,
    }
