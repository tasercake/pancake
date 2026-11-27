"""CUDA implementation of the native lightness stencils and range damage.

The reference and delivery already use the NumPy CIELAB conversion. Keep the
SciPy accumulation order, intermediate float32 stores and IEEE division here,
so the shared CPU pooling consumes the same per-pixel values.
"""

from functools import lru_cache
from math import sqrt

import numpy as np
import torch
import triton
from triton import language as tl


@triton.jit
def _reflect(index, size, single: tl.constexpr):
    if single:
        position = tl.where(index < 0, -index - 1, index)
    else:
        period = 2 * size
        position = ((index % period) + period) % period
    return tl.where(position < size, position, 2 * size - position - 1)


@triton.jit
def _gaussian(source, weights, output, h, w, radius: tl.constexpr,
              vertical: tl.constexpr, single: tl.constexpr, block: tl.constexpr):
    p = tl.program_id(0) * block + tl.arange(0, block)
    valid = p < h * w
    y, x = p // w, p % w
    value = tl.load(source + p, valid, 0).to(tl.float64) * tl.load(weights + radius)
    for d in range(radius, 0, -1):
        if vertical:
            a = _reflect(y - d, h, single) * w + x
            b = _reflect(y + d, h, single) * w + x
        else:
            a = y * w + _reflect(x - d, w, single)
            b = y * w + _reflect(x + d, w, single)
        pair = tl.load(source + a, valid, 0).to(tl.float64) + tl.load(source + b, valid, 0).to(tl.float64)
        value += pair * tl.load(weights + radius - d)
    tl.store(output + p, value, valid)


@triton.jit
def _normalize(numerator, denominator, n, block: tl.constexpr):
    p = tl.program_id(0) * block + tl.arange(0, block)
    weight = tl.load(denominator + p, p < n, 0)
    value = tl.load(numerator + p, p < n, 0)
    tl.store(numerator + p, tl.where(weight > 0, tl.div_rn(value, weight), 0), p < n)


@triton.jit
def _visible(error, scale: tl.constexpr):
    return tl.div_rn(error, error + scale)


@triton.jit
def _loss_gain(c, t, cg, tg, diagonal: tl.constexpr, diag_norm: tl.constexpr, block: tl.constexpr):
    # Subtract in float32 as NumPy does, then promote to float64 for the
    # directional distance and visibility response.
    dc = (cg - c).to(tl.float64)
    dt = (tg - t).to(tl.float64)
    if diagonal:
        norm = tl.full((block,), diag_norm, tl.float64)
        dc = dc / norm
        dt = dt / norm
    match = ((dc > 0) & (dt > 0)) | ((dc < 0) & (dt < 0))
    ac, at = tl.abs(dc), tl.abs(dt)
    earned = tl.where(match, tl.minimum(ac, at), 0)
    lost, extra = at - earned, ac - earned
    return lost / (lost + 5.), extra / (extra + 5.)


@triton.jit
def _directions(candidate, target, support, lost, extra, h, w, masked: tl.constexpr,
                initial: tl.constexpr, diag_norm: tl.constexpr, block: tl.constexpr):
    p = tl.program_id(0) * block + tl.arange(0, block)
    valid = p < h * w
    y, x = p // w, p % w
    c, t = tl.load(candidate + p, valid, 0), tl.load(target + p, valid, 0)
    if masked:
        valid &= tl.load(support + p, valid, 0)
    largest_loss = tl.full((block,), 0., tl.float64)
    largest_gain = tl.full((block,), 0., tl.float64)
    for direction in tl.static_range(8):
        cell = direction if direction < 4 else direction + 1
        dy, dx = cell // 3 - 1, cell % 3 - 1
        q = p + dy * w + dx
        use = valid & (y + dy >= 0) & (y + dy < h) & (x + dx >= 0) & (x + dx < w)
        if masked:
            use &= tl.load(support + q, use, 0)
        cg, tg = tl.load(candidate + q, use, 0), tl.load(target + q, use, 0)
        loss, gain = _loss_gain(c, t, cg, tg, dy != 0 and dx != 0, diag_norm, block)
        largest_loss = tl.maximum(largest_loss, tl.where(use, loss, 0.))
        largest_gain = tl.maximum(largest_gain, tl.where(use, gain, 0.))
    valid = p < h * w
    if not initial:
        largest_loss = tl.maximum(largest_loss.to(tl.float32), tl.load(lost + p, valid, 0))
        largest_gain = tl.maximum(largest_gain.to(tl.float32), tl.load(extra + p, valid, 0))
    tl.store(lost + p, largest_loss, valid)
    tl.store(extra + p, largest_gain, valid)


@triton.jit
def _clamp_index(index, size):
    return tl.minimum(tl.maximum(index, 0), size - 1)


@triton.jit
def _edges(target, upper, lower, strength, h, w, block: tl.constexpr):
    p = tl.program_id(0) * block + tl.arange(0, block)
    valid = p < h * w
    y, x = p // w, p % w
    high = tl.load(target + p, valid, 0)
    low = high
    # At the true image edges clamping and reflection visit the same values
    # for an extrema filter, so no halo is required.
    for dy in tl.static_range(-1, 2):
        for dx in tl.static_range(-1, 2):
            q = _clamp_index(y + dy, h) * w + _clamp_index(x + dx, w)
            value = tl.load(target + q, valid, 0)
            high, low = tl.maximum(high, value), tl.minimum(low, value)
    tl.store(upper + p, high, valid)
    tl.store(lower + p, low, valid)
    tl.store(strength + p, _visible(high - low, 6.), valid)


@triton.jit
def _max9(strength, output, h, w, vertical: tl.constexpr, block: tl.constexpr):
    p = tl.program_id(0) * block + tl.arange(0, block)
    valid = p < h * w
    y, x = p // w, p % w
    strongest = tl.load(strength + p, valid, 0)
    for d in tl.static_range(-4, 5):
        if vertical:
            q = _clamp_index(y + d, h) * w + x
        else:
            q = y * w + _clamp_index(x + d, w)
        strongest = tl.maximum(strongest, tl.load(strength + q, valid, 0))
    tl.store(output + p, strongest, valid)


@triton.jit
def _combine(candidate, delta, fine, local, broad, edge, near, upper, lower,
             tone, extra, boundary, n, block: tl.constexpr):
    p = tl.program_id(0) * block + tl.arange(0, block)
    valid = p < n
    c = tl.load(candidate + p, valid, 0)
    d = tl.load(delta + p, valid, 0)
    local_d = tl.load(local + p, valid, 0)
    near_edge = tl.load(near + p, valid, 0)
    grain = _visible(tl.abs(d - tl.load(fine + p, valid, 0)), 5.) * (1. - tl.load(edge + p, valid, 0))
    shape = _visible(tl.abs(d - local_d), 6.) * near_edge
    overshoot = tl.maximum(tl.maximum(c - tl.load(upper + p, valid, 0), tl.load(lower + p, valid, 0) - c), 0.)
    tl.store(tone + p, _visible(tl.maximum(tl.abs(local_d), tl.abs(tl.load(broad + p, valid, 0))), 8.), valid)
    tl.store(extra + p, tl.maximum(grain, tl.load(extra + p, valid, 0)), valid)
    tl.store(boundary + p, tl.maximum(shape, _visible(overshoot, 6.) * near_edge), valid)


@triton.jit
def _range(candidate, target, output, n, block: tl.constexpr):
    p = tl.program_id(0) * block + tl.arange(0, block)
    valid = p < n
    endpoint = tl.full((block,), .012, tl.float64)
    scale = tl.full((block,), .075, tl.float64)
    maximum = tl.full((block,), 0., tl.float64)
    for channel in tl.static_range(3):
        c = tl.load(candidate + 3 * p + channel, valid, 0).to(tl.float64)
        t = tl.load(target + 3 * p + channel, valid, 0).to(tl.float64)
        cd, td = tl.clamp(c, 0., 1.), tl.clamp(t, 0., 1.)
        headroom = tl.abs((c - cd) - (t - td))
        at_endpoint = 1. - tl.clamp(tl.minimum(cd, 1. - cd) / endpoint, 0., 1.)
        clipping = tl.abs(cd - td) * at_endpoint
        maximum = tl.maximum(maximum, tl.maximum(headroom, clipping))
    tl.store(output + p, maximum / (maximum + scale), valid)


@lru_cache(maxsize=16)
def _weights(sigma, device):
    radius = int(4 * sigma + .5)
    x = np.arange(-radius, radius + 1)
    weights = np.exp(-.5 / (sigma * sigma) * x ** 2)
    return torch.from_numpy(weights / weights.sum()).to(device), radius


def damage_maps(candidate, target, c, t, delta, support):
    """Return the lightness and range damage arrays."""
    device = torch.device("cuda", torch.cuda.current_device())
    c, t, delta = (torch.from_numpy(value).to(device) for value in (c, t, delta))
    h, w = t.shape
    n = h * w
    block = 128
    grid = (triton.cdiv(n, block),)
    masked = support is not None
    mask = torch.from_numpy(support).to(device) if masked else t
    scratch = torch.empty_like(t)
    denominators = {}

    def gaussian(image, sigma):
        weights, radius = _weights(sigma, device)
        output = torch.empty_like(image)
        _gaussian[grid](image, weights, scratch, h, w, radius, True, h >= radius, block, enable_fp_fusion=False)
        _gaussian[grid](scratch, weights, output, h, w, radius, False, w >= radius, block, enable_fp_fusion=False)
        return output

    def blur(image, sigma):
        if not masked:
            return gaussian(image, sigma)
        if sigma not in denominators:
            denominators[sigma] = gaussian(mask.float(), sigma)
        output = gaussian(image * mask, sigma)
        _normalize[grid](output, denominators[sigma], n, block)
        return output

    tone, lost, extra, boundary = (torch.empty_like(t) for _ in range(4))
    for sigma in (0, 1.1, 2.5):
        cs, ts = (blur(c, sigma), blur(t, sigma)) if sigma else (c, t)
        _directions[grid](cs, ts, mask, lost, extra, h, w, masked, not sigma,
                          sqrt(2), block, enable_fp_fusion=False)
    upper, lower, edge, near = (torch.empty_like(t) for _ in range(4))
    _edges[grid](t, upper, lower, edge, h, w, block)
    _max9[grid](edge, scratch, h, w, True, block)
    _max9[grid](scratch, near, h, w, False, block)
    fine, local, broad = (blur(delta, sigma) for sigma in (.85, 2.5, 10))
    _combine[grid](c, delta, fine, local, broad, edge, near, upper, lower,
                   tone, extra, boundary, n, block, enable_fp_fusion=False)
    def upload(image):
        contiguous = np.asarray(image, dtype=np.float32, order="C")
        # A stacker or the decoder can return a read-only view.
        if not contiguous.flags.writeable:
            contiguous = contiguous.copy()
        return torch.from_numpy(contiguous).to(device)

    raw_c, raw_t = upload(candidate), upload(target)
    channel_range = torch.empty_like(t)
    _range[grid](raw_c, raw_t, channel_range, n, block, enable_fp_fusion=False)
    return tuple(image.cpu().numpy() for image in (tone, lost, extra, boundary, channel_range))
