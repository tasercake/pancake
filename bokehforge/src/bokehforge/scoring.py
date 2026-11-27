"""Reference coverage and native-grid image quality measurements for BokehForge."""

from base64 import b64encode
from dataclasses import dataclass
from typing import Literal
from zlib import compress

import numpy as np
from scipy import ndimage as ndi

from ._device import DamageBackend, Device, damage_backend
from ._workers import alignment_worker, workers
from .alignment import _xyxy, align, native_overlap, overlap, project

SCORE_VERSION = "reference-fidelity-v3"
INSPECTION_SCORE_VERSION = "reference-fidelity-v3-inspection-v2"
INSPECTION_MIX = 0.5
INSPECTION_MAX_MULTIPLIER = 2.0
SCORE_DAMAGE_SCALE = 5
FAMILY_WEIGHTS = {
    "tone": 0.17, "color": 0.16, "lost_detail": 0.18,
    "added_structure": 0.15, "boundary": 0.23, "range": 0.11,
}
FEATURES = tuple(f"native.{family}" for family in FAMILY_WEIGHTS)
SCORE_EXPLANATION_VERSION = 2
SCORE_DETAIL_LEVELS = ("summary", "tiles", "attention", "full")
ScoreDetails = Literal["summary", "tiles", "attention", "full"]
_POOL_WEIGHTS = {"mean": 0.40, "tail": 0.35, "worst": 0.25}
_POOL_LABELS = {"mean": "Average damage", "tail": "Average of the worst 1% of tiles",
                "worst": "Worst single tile"}
_TAIL_FRACTION = 0.01
_AFFECTED_THRESHOLD = 0.25
_STRIP_ROWS = 192
_FILTER_HALO = 40
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
    alignment: dict | None = None

    @property
    def score(self):
        return scene_score(self)


def _validate_details(details):
    if details is not None and details not in SCORE_DETAIL_LEVELS:
        raise ValueError(f"Expected score details to be None or one of {SCORE_DETAIL_LEVELS}, got {details!r}")


def validate_rgb(pixels):
    """Check the delivery grid, precision, and finite sample contract."""
    if not isinstance(pixels, np.ndarray):
        raise ValueError("The benchmark requires an RGB image array.")
    if pixels.ndim != 3 or pixels.shape[2] != 3 or not all(pixels.shape):
        raise ValueError(f"The benchmark requires a nonempty RGB image with three channels. "
                         f"The image has array shape {pixels.shape}.")
    if pixels.dtype.kind != "f" or pixels.dtype.itemsize != 4:
        raise ValueError(f"The image uses {pixels.dtype} samples. "
                         "The benchmark requires 32-bit floating-point linear-sRGB samples.")
    if not np.isfinite(pixels).all():
        raise ValueError("The image contains non-finite pixel values (NaN or infinity).")


def _lab(rgb):
    # The display clips at SDR black and white. Native range evidence is measured separately.
    channels = np.empty((3, *rgb.shape[:2]), dtype=np.float32)
    np.clip(np.moveaxis(rgb, -1, 0), 0, 1, out=channels)
    red, green, blue = channels
    xyz = np.empty_like(channels)
    scratch = np.empty_like(red)
    # Work on contiguous planes and retain the original multiply/add order.
    for output, (r, g, b) in zip(xyz, _XYZ, strict=True):
        np.multiply(red, r, out=output)
        np.multiply(green, g, out=scratch)
        np.add(output, scratch, out=output)
        np.multiply(blue, b, out=scratch)
        np.add(output, scratch, out=output)
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


def _inspection_weights(heatmap):
    mean = float(heatmap.mean(dtype=np.float64))
    peak = float(heatmap.max())
    if peak == mean or not mean:
        return None, 0.0
    # Ease toward the upper bound with matching slopes on either side of the shoulder.
    headroom = INSPECTION_MAX_MULTIPLIER - 1
    shoulder = headroom / 2
    peak_boost = INSPECTION_MIX * (peak - mean) / mean
    if peak_boost > shoulder:
        excess = peak_boost - shoulder
        remaining = headroom - shoulder
        peak_boost = shoulder + remaining * excess / (remaining + excess)
    gain = peak_boost / (peak - mean)
    weights = (1 + (heatmap.astype(np.float64) - mean) * gain).astype(np.float32)
    return weights, gain * mean


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


def _lightness_damage(cl, tl, delta, support, extrema=None):
    blur = _supported_blur(support)
    local_delta = blur(delta, 2.5)
    # Tone carries broad gain, contrast, local lighting, and seam differences.
    tone = _visibility(np.maximum(abs(local_delta), abs(blur(delta, 10))), 8.0)

    lost, extra = _detail(cl, tl, blur, support)
    if extrema is None:
        upper = ndi.maximum_filter(tl, size=3, mode="reflect")
        lower = ndi.minimum_filter(tl, size=3, mode="reflect")
        edge_weight = _visibility(upper - lower, 6.0)
        near_edge = ndi.maximum_filter(edge_weight, size=9, mode="reflect")
    else:
        upper, lower, edge_weight, near_edge = extrema
    # Residual grain is useful evidence in smooth backgrounds. At an existing
    # transition, the signed detail bank and edge neighborhood carry the signal.
    grain = _visibility(abs(delta - blur(delta, 0.85)), 5.0) * (1 - edge_weight)
    extra = np.maximum(extra, grain)
    shape = _visibility(abs(delta - local_delta), 6.0) * near_edge
    overshoot = _visibility(np.maximum.reduce([cl - upper, lower - cl, np.zeros_like(cl)]), 6.0)
    boundary = np.maximum(shape, overshoot * near_edge)

    return tone, lost, extra, boundary


def _damage(candidate, target, support):
    cl, ca, cb = _lab(candidate)
    tl, ta, tb = _lab(target)
    delta, delta_a, delta_b = cl - tl, ca - ta, cb - tb
    tone, lost, extra, boundary = _lightness_damage(cl, tl, delta, support)
    # CIELAB opponent-plane distance keeps a cast visible alongside a tone error.
    color = _visibility(np.hypot(delta_a, delta_b), 10.0)
    return (tone, color, lost, extra, boundary, _range_damage(candidate, target)), (delta, delta_a, delta_b)


def _range_damage(candidate, target):
    # Work in float64 before subtraction so all finite float32 samples stay finite.
    # Work on one channel at a time to limit peak memory at native resolution.
    range_error = np.zeros(candidate.shape[:2], dtype=np.float32)
    for start in range(0, candidate.shape[0], 128):
        rows = slice(start, start + 128)
        channels = zip(np.moveaxis(candidate[rows], -1, 0), np.moveaxis(target[rows], -1, 0), strict=True)
        for ci, ti in channels:
            c64, t64 = ci.astype(np.float64), ti.astype(np.float64)
            c_display, t_display = np.clip(c64, 0, 1), np.clip(t64, 0, 1)
            headroom = abs((c64 - c_display) - (t64 - t_display))
            at_endpoint = 1 - np.clip(np.minimum(c_display, 1 - c_display) / 0.012, 0, 1)
            clipping = abs(c_display - t_display) * at_endpoint
            np.maximum(range_error[rows], _visibility(np.maximum(headroom, clipping), 0.075), out=range_error[rows])
    return range_error


def _rgb_exposure(pixels, support):
    return np.maximum(_supported_mean(pixels, support, axis=(0, 1)), 0) @ _XYZ[1]


def _lab_comparison(candidate, target):
    height, width = target.shape[:2]
    cl, tl, delta, delta_a, delta_b, color = np.empty((6, height, width), dtype=np.float32)

    def prepare(start):
        rows = slice(start, start + _STRIP_ROWS)
        c, a, b = _lab(candidate[rows])
        t, ta, tb = _lab(target[rows])
        cl[rows], tl[rows] = c, t
        delta[rows] = c - t
        np.subtract(a, ta, out=delta_a[rows])
        np.subtract(b, tb, out=delta_b[rows])
        color[rows] = _visibility(np.hypot(delta_a[rows], delta_b[rows]), 10.)

    list(workers().map(prepare, range(0, height, _STRIP_ROWS)))
    return cl, tl, (delta, delta_a, delta_b), color


def _cuda_maps(candidate, target, support):
    from ._cuda_damage import damage_maps

    cl, tl, deltas, color = _lab_comparison(candidate, target)
    delta, _, _ = deltas
    tone, lost, extra, boundary, rgb_range = damage_maps(candidate, target, cl, tl, delta, support)
    return (tone, color, lost, extra, boundary, rgb_range), deltas


def _damage_strips(shape, count, calculate):
    height, width = shape
    arrays = np.empty((count, height, width), dtype=np.float32)

    def strip(start):
        # The radius-40 tone filter is the widest stencil. Preserve reflection
        # at the actual image edges while keeping strip work cache-local.
        end = min(start + _STRIP_ROWS, height)
        first, last = max(0, start - _FILTER_HALO), min(height, end + _FILTER_HALO)
        for output, values in zip(arrays, calculate(slice(first, last)), strict=True):
            output[start:end] = values[start - first:end - first]

    list(workers().map(strip, range(0, height, _STRIP_ROWS)))
    return arrays


def _mps_maps(candidate, target, support):
    from ._mps_damage import lightness_extrema, neighborhood_strength

    cl, tl, deltas, color = _lab_comparison(candidate, target)
    delta, _, _ = deltas
    upper, lower = lightness_extrema(tl)
    # Keep division in NumPy and in its original position before the max filter.
    strength = _visibility(upper - lower, 6.0)
    extrema = upper, lower, strength, neighborhood_strength(strength)

    def calculate(rows):
        lightness = _lightness_damage(cl[rows], tl[rows], delta[rows],
                                      None if support is None else support[rows], tuple(plane[rows] for plane in extrema))
        return (*lightness, _range_damage(candidate[rows], target[rows]))

    tone, lost, extra, boundary, rgb_range = _damage_strips(target.shape[:2], 5, calculate)
    return (tone, color, lost, extra, boundary, rgb_range), deltas


def _maps(candidate, target, support=None, *, backend: DamageBackend = "cpu"):
    height, width = target.shape[:2]
    large = height * width >= 1_000_000
    exposure = [workers().submit(_rgb_exposure, image, support) for image in (candidate, target)] if large else None
    if backend == "cuda":
        maps, (delta, delta_a, delta_b) = _cuda_maps(candidate, target, support)
    elif backend == "mps":
        maps, (delta, delta_a, delta_b) = _mps_maps(candidate, target, support)
    elif height >= _STRIP_ROWS and large:
        def calculate(rows):
            maps, signed = _damage(candidate[rows], target[rows], None if support is None else support[rows])
            return (*maps, *signed)

        arrays = _damage_strips((height, width), len(FAMILY_WEIGHTS) + 3, calculate)
        maps, (delta, delta_a, delta_b) = arrays[:len(FAMILY_WEIGHTS)], arrays[len(FAMILY_WEIGHTS):]
    else:
        maps, (delta, delta_a, delta_b) = _damage(candidate, target, support)
    if exposure is None:
        c_exposure, t_exposure = (_rgb_exposure(image, support) for image in (candidate, target))
    else:
        c_exposure, t_exposure = (future.result() for future in exposure)
    return maps, {
        "mean_delta_lightness": float(_supported_mean(delta, support)),
        "global_exposure_delta_stops": float(np.log2((c_exposure + 1e-5) / (t_exposure + 1e-5))),
        "mean_delta_lab_a": float(_supported_mean(delta_a, support)),
        "mean_delta_lab_b": float(_supported_mean(delta_b, support)),
    }


def _axis_tiles(length, side, offset):
    edges = np.arange(offset, length, side, dtype=int)
    return np.unique(np.r_[0, edges, length])


def _integral(image):
    return np.pad(image.cumsum(0, dtype=np.float64).cumsum(1), ((1, 0), (1, 0)))


def _tile_totals(integral, y0, y1, x0, x1):
    return (integral[np.ix_(y1, x1)] - integral[np.ix_(y0, x1)]
            - integral[np.ix_(y1, x0)] + integral[np.ix_(y0, x0)])


def _tile_integral(image, support, edges):
    """Compute the original vertical-then-horizontal sums at tile corners."""
    ys, xs = edges
    height, width = image.shape
    slab_rows = 96
    buffer = np.zeros((min(height, slab_rows) + 1, width), dtype=np.float64)
    sampled = np.zeros((len(ys), width), dtype=np.float64)
    for start in range(0, height, slab_rows):
        stop = min(start + slab_rows, height)
        slab = buffer[:stop - start + 1]
        if support is None:
            slab[1:] = image[start:stop]
        else:
            np.multiply(image[start:stop], support[start:stop], out=slab[1:])
        np.cumsum(slab, axis=0, out=slab)
        first, last = np.searchsorted(ys, (start, stop), side="right")
        np.take(slab, ys[first:last] - start, axis=0, out=sampled[first:last])
        # Seed the next slab with the preceding sum, preserving every addition.
        buffer[0] = slab[-1]
    compact = np.zeros((len(ys), len(xs)), dtype=np.float64)
    sampled.cumsum(axis=1, out=sampled)
    compact[:, 1:] = sampled[:, xs[1:] - 1]
    return compact


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


def _tile_grids(height, width, support, exact_box=None):
    support_integral = None if support is None else _integral(support)
    grids = []
    for side in (8, 32):
        for y_offset, x_offset in ((0, 0), (0, side // 2), (side // 2, 0), (side // 2, side // 2)):
            y = _axis_tiles(height, side, y_offset)
            x = _axis_tiles(width, side, x_offset)
            covered = None if support_integral is None else _tile_totals(
                support_integral, y[:-1], y[1:], x[:-1], x[1:]).astype(np.int32)
            if exact_box is not None and exact_box != (0, 0, height, width):
                top, left, bottom, right = exact_box
                rows = np.maximum(0, np.minimum(y[1:], bottom) - np.maximum(y[:-1], top))
                columns = np.maximum(0, np.minimum(x[1:], right) - np.maximum(x[:-1], left))
                covered = rows[:, None] * columns
            grids.append(_TileGrid(side, (y_offset, x_offset), x, y, min(side, height) * min(side, width), covered))
    return grids


def _pool(damage, support, grids, edges, *, explain=False, tiles_by_grid=None, family=None, damage_max=1):
    """Compute the metric and optionally retain the evidence from those same tiles."""
    exact = damage is None
    mean = 0.0 if exact else float(_supported_mean(damage, support))
    integral = None if exact else _tile_integral(damage, support, edges)
    worst, tail, region = 0.0, 0.0, None
    worst_tile, tail_selection, winning_tiles = None, None, None
    for grid in grids:
        # Even a short edge tile uses the full nominal area as its denominator.
        if exact:
            tiles = np.zeros((grid.x.size - 1) * (grid.y.size - 1))
        else:
            y, x = (np.searchsorted(axis, grid_axis) for axis, grid_axis in zip(edges, (grid.y, grid.x), strict=True))
            totals = _tile_totals(integral, y[:-1], y[1:], x[:-1], x[1:])
            tiles = np.clip(totals / grid.nominal_pixels, 0, damage_max).ravel()
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
        "affected_fraction": 0.0 if exact else float(_supported_mean(damage >= _AFFECTED_THRESHOLD, support)),
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


def _rgb_rmse(candidate, target, support):
    mse = np.mean([_supported_mean(np.square(np.subtract(c, t, dtype=np.float64)), support)
                   for c, t in zip(np.moveaxis(candidate, -1, 0), np.moveaxis(target, -1, 0), strict=True)])
    return float(np.sqrt(mse))


def _measure(candidate, target, support=None, *, details=None, exact_box=None, inspection=None,
             backend: DamageBackend = "cpu"):
    height, width = target.shape[:2]
    large = height * width >= 1_000_000
    rmse_work = None
    if exact_box is None:
        if large:
            rmse_work = workers().submit(_rgb_rmse, candidate, target, support)
        maps, signed = _maps(candidate, target, support, backend=backend)
        measured_pixels = height * width if support is None else int(support.sum())
    else:
        top, left, bottom, right = exact_box
        measured_pixels = (bottom - top) * (right - left)
        maps = (None,) * len(FAMILY_WEIGHTS)
        signed = dict.fromkeys(("mean_delta_lightness", "global_exposure_delta_stops",
                                "mean_delta_lab_a", "mean_delta_lab_b"), 0.0)
    grids = _tile_grids(height, width, support, exact_box) if exact_box is None or details is not None else []
    edges = tuple(np.unique(np.concatenate([getattr(grid, axis) for grid in grids])) for axis in ("y", "x")) \
        if exact_box is None else None
    tile_data = {grid.id: grid.describe_tiles() for grid in grids} if details in ("tiles", "full") else None
    features = {}
    rows = {}
    families = {}
    weights, mix = inspection or (None, 0.0)
    damage_max = 1 if inspection is None else INSPECTION_MAX_MULTIPLIER

    def pool_family(item):
        name, damage = item
        if damage is not None and weights is not None:
            damage *= weights
        return _pool(damage, support, grids, edges, explain=details is not None,
                     tiles_by_grid=tile_data, family=name, damage_max=damage_max)

    dispatch = workers().map if large and exact_box is None and tile_data is None else map
    pooled = dispatch(pool_family, zip(FAMILY_WEIGHTS, maps, strict=True))
    for (name, weight), (burden, summary, components) in zip(FAMILY_WEIGHTS.items(), pooled, strict=True):
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
    if measured_pixels < height * width:
        native["covered_fraction"] = measured_pixels / (height * width)
    rmse = 0.0 if exact_box is not None else (
        _rgb_rmse(candidate, target, support) if rmse_work is None else rmse_work.result()
    )
    explanation = None
    if details is not None:
        explanation = {
            "reference_size_wh": [width, height], "damage_range": [0, damage_max], "affected_threshold": _AFFECTED_THRESHOLD,
            "weighted_damage": _weighted_damage(features), "families": families,
            "measured_pixels": measured_pixels,
            "diagnostics": {**signed, "linear_rgb_rmse": rmse},
        }
        if inspection is not None:
            explanation["inspection_mix"] = mix
        if tile_data is not None:
            explanation["tile_grids"] = tile_data
    return Measurement(features, {"views": {"native": [native]}, "linear_rgb_rmse": rmse}, explanation)


def measure(candidate, target, *, details: ScoreDetails | None = None, inspection_heatmap=None, attention=True,
            device: Device = "auto"):
    """Score one delivered RGB image against the reference at its native resolution.

    Alignment and per-placement score terms are always available in `.alignment`.
    Request `summary` for damage pooling components or `tiles` for local values.
    Attention is generated from the reference unless disabled. A supplied heatmap
    replaces generation and must contain finite values in [0, 1] on the reference grid.
    """
    _validate_details(details)
    validate_rgb(target)
    validate_rgb(candidate)
    generate = None
    if not attention:
        inspection_heatmap = None
    elif inspection_heatmap is None:
        from .inspection import inspection_for

        def generate(reference):
            return inspection_for(device).generate(reference)[0]

    return _measure_delivery(candidate, target, details=details, device=device,
                             heatmap=inspection_heatmap, generate=generate)


def _measure_delivery(candidate, target, *, details, device, heatmap=None, generate=None):
    """Score images already checked against the RGB input contract."""
    large = target.shape[0] * target.shape[1] >= 1_000_000
    backend = damage_backend(device) if large or device != "auto" else "cpu"
    pending = alignment_worker().submit(align, candidate, target) if large and generate is not None else None
    try:
        if generate is not None:
            heatmap = generate(target)
        if heatmap is not None:
            heatmap = validate_heatmap(heatmap, target.shape[:2])
        inspection = None if heatmap is None else _inspection_weights(heatmap)
    except BaseException:
        if pending is not None and not pending.cancel():
            pending.exception()
        raise
    placements, alignment = align(candidate, target) if pending is None else pending.result()
    rows, explanations = [], []
    leading = None
    for placement in placements:
        geometry, resolution, polygon = overlap(placement, target.shape[:2])
        comparison = None
        if geometry > 0:
            if placement.evidence["source"] in {"exact_native_delivery_in_reference", "exact_reference_in_native_delivery"}:
                comparison = _measure(None, target, details=details, exact_box=native_overlap(placement, target.shape),
                                      inspection=inspection)
            else:
                aligned, support = project(candidate, target.shape, placement)
                if support is None or support.any():
                    try:
                        comparison = _measure(aligned, target, support, details=details,
                                              inspection=inspection, backend=backend)
                    except RuntimeError as exc:
                        if backend == "cpu" or device != "auto":
                            raise
                        from torch import OutOfMemoryError

                        # PyTorch's MPS allocator uses RuntimeError for exhaustion.
                        if not isinstance(exc, OutOfMemoryError) and not (
                            backend == "mps" and str(exc).startswith("MPS backend out of memory")
                        ):
                            raise
                        backend = "cpu"
                    if comparison is None:
                        comparison = _measure(aligned, target, support, details=details, inspection=inspection)
        quality = 0.0 if comparison is None else comparison.score
        coverage = min(geometry, resolution)
        weighted = placement.weight * coverage * quality
        row = {**placement.pose.describe(), "evidence": placement.evidence,
               "candidate_content_box_xyxy": _xyxy(placement.box), "padding": list(placement.padding),
               "reference_overlap_polygon_xy": polygon, "geometric_coverage": geometry,
               "resolution_cap": resolution, "coverage": coverage, "weight": placement.weight,
               "overlap_score": quality, "score_contribution": weighted,
               "comparison_status": "no_measured_reference_centers" if comparison is None else "measured"}
        rows.append(row)
        if leading is None:
            leading = comparison
        if details is not None:
            explanations.append({**row, "comparison": None if comparison is None else comparison.explanation})
    alignment["placements"] = rows
    score = sum(row["score_contribution"] for row in rows)
    explanation = None
    if details is not None:
        explanation = {
            "schema_version": SCORE_EXPLANATION_VERSION,
            "profile": SCORE_VERSION if inspection is None else INSPECTION_SCORE_VERSION, "detail_level": details,
            "score": score, "damage_scale": SCORE_DAMAGE_SCALE,
            "reference_size_wh": alignment["reference_size_wh"], "candidate_size_wh": alignment["candidate_size_wh"],
            "confidence": alignment["confidence"], "alignment_status": alignment["status"],
            "reason": alignment["reason"], "placements": explanations,
        }
        if heatmap is not None:
            weights, mix = inspection
            diagnostic = {
                "size_wh": [heatmap.shape[1], heatmap.shape[0]],
                "range": [0, 1],
                "mean": float(heatmap.mean(dtype=np.float64)),
                "mix": mix,
                "damage_multiplier_range": [1.0, 1.0] if weights is None
                else [float(weights.min()), float(weights.max())],
            }
            if details in ("attention", "full"):
                pixels = heatmap.astype("<f4", copy=False).tobytes()
                diagnostic.update(encoding="base64+zlib+float32-le", data=b64encode(compress(pixels, 1)).decode("ascii"))
            explanation["attention"] = diagnostic
    return Measurement(leading.features if leading else {}, leading.diagnostics if leading else {},
                       explanation, alignment)


def validate_heatmap(values, shape):
    if (not isinstance(values, np.ndarray) or values.shape != shape or values.dtype.kind != "f"
            or not np.isfinite(values).all() or values.min() < 0 or values.max() > 1):
        raise ValueError(f"Expected a finite floating-point inspection map in [0, 1] with shape {shape}")
    return values.astype(np.float32, copy=False)


def _weighted_damage(features):
    return sum(weight * features[f"native.{family}"] for family, weight in FAMILY_WEIGHTS.items())


def scene_score(measurement):
    """The final reference-fidelity score of a measurement."""
    if measurement.alignment is not None:
        return sum(row["score_contribution"] for row in measurement.alignment["placements"])
    return float(100 * np.exp(-SCORE_DAMAGE_SCALE * _weighted_damage(measurement.features)))
