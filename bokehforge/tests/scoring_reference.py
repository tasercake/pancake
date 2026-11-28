"""Independent NumPy and SciPy reference kernels used by scoring tests."""

from functools import lru_cache

import numpy as np
from scipy import ndimage as ndi

# Linear sRGB to D65 XYZ, normalized by the D65 white point.
XYZ = np.array([
    [0.4124564, 0.3575761, 0.1804375],
    [0.2126729, 0.7151522, 0.0721750],
    [0.0193339, 0.1191920, 0.9503041],
], dtype=np.float32) / np.array([0.95047, 1, 1.08883], dtype=np.float32)[:, None]


def lab(rgb):
    # The display clips at SDR black and white. Native range evidence is measured separately.
    channels = np.empty((3, *rgb.shape[:2]), dtype=np.float32)
    np.clip(np.moveaxis(rgb, -1, 0), 0, 1, out=channels)
    red, green, blue = channels
    xyz = np.empty_like(channels)
    scratch = np.empty_like(red)
    # Work on contiguous planes and retain the original multiply/add order.
    for output, (r, g, b) in zip(xyz, XYZ, strict=True):
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


def supported_mean(values, support, axis=None):
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


@lru_cache(maxsize=1)
def lightness_weights():
    kernels = []
    for sigma in (0.85, 1.1, 2.5, 10):
        radius = int(4 * sigma + .5)
        offsets = np.arange(-radius, radius + 1)
        weights = np.exp(-.5 / (sigma * sigma) * offsets ** 2)
        kernels.append((weights / weights.sum())[:radius + 1])
    return tuple(kernels)


def lightness_damage(cl, tl, delta, support, extrema=None):
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
    np.maximum(extra, grain, out=extra)
    shape = _visibility(abs(delta - local_delta), 6.0) * near_edge
    overshoot = cl - upper
    np.maximum(overshoot, lower - cl, out=overshoot)
    np.maximum(overshoot, 0, out=overshoot)
    np.maximum(shape, _visibility(overshoot, 6.0) * near_edge, out=shape)

    return tone, lost, extra, shape


def damage(candidate, target, support):
    cl, ca, cb = lab(candidate)
    tl, ta, tb = lab(target)
    delta, delta_a, delta_b = cl - tl, ca - ta, cb - tb
    tone, lost, extra, boundary = lightness_damage(cl, tl, delta, support)
    # CIELAB opponent-plane distance keeps a cast visible alongside a tone error.
    color = _visibility(np.hypot(delta_a, delta_b), 10.0)
    return (tone, color, lost, extra, boundary, range_damage(candidate, target)), (delta, delta_a, delta_b)


def range_damage(candidate, target):
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


def integral(image):
    return np.pad(image.cumsum(0, dtype=np.float64).cumsum(1), ((1, 0), (1, 0)))


def tile_totals(integral, y0, y1, x0, x1):
    totals = integral[np.ix_(y1, x1)]
    totals -= integral[np.ix_(y0, x1)]
    totals -= integral[np.ix_(y1, x0)]
    totals += integral[np.ix_(y0, x0)]
    return totals
