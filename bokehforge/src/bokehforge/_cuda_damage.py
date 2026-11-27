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
def _reflect(index, size):
    period = 2 * size
    position = ((index % period) + period) % period
    return tl.where(position < size, position, 2 * size - position - 1)


@triton.jit
def _stencil_positions(h, w, row_blocks, packed: tl.constexpr, block: tl.constexpr):
    tile = tl.program_id(0)
    lanes = tl.arange(0, block)
    if packed:
        p = tile * block + lanes
        y, x = p // w, p % w
        valid = p < h * w
    else:
        y = tile // row_blocks
        x = (tile % row_blocks) * block + lanes
        p = y * w + x
        valid = x < w
    return y, x, p, valid


@triton.jit
def _gaussian(source, weights, output, denominator, h, w, row_blocks,
              radius: tl.constexpr, vertical: tl.constexpr, single: tl.constexpr,
              normalize: tl.constexpr, packed: tl.constexpr, block: tl.constexpr):
    y, x, p, valid = _stencil_positions(h, w, row_blocks, packed, block)
    if vertical:
        index, size, stride, first = y, h, w, x
    else:
        index, size, stride, first = x, w, 1, y * w
    value = tl.load(source + p, valid, 0).to(tl.float64) * tl.load(weights + radius)
    for step in tl.range(0, radius, loop_unroll_factor=4):
        d = radius - step
        if single:
            a = tl.maximum(index - d, d - index - 1)
            b = tl.minimum(index + d, 2 * size - index - d - 1)
        else:
            a = _reflect(index - d, size)
            b = _reflect(index + d, size)
        left = tl.load(source + first + a * stride, valid, 0).to(tl.float64)
        right = tl.load(source + first + b * stride, valid, 0).to(tl.float64)
        pair = left + right
        value += pair * tl.load(weights + step)
    if normalize:
        # Apply division to the same rounded value as the unfused filter.
        weight = tl.load(denominator + p, valid, 0)
        value = tl.where(weight > 0, tl.div_rn(value.to(tl.float32), weight), 0)
    tl.store(output + p, value, valid)


@triton.jit
def _mask_sources(candidate, target, delta, support, output, n, block: tl.constexpr):
    p = tl.program_id(0) * block + tl.arange(0, block)
    valid = p < n
    weight = tl.load(support + p, valid, 0).to(tl.float32)
    tl.store(output + p, tl.load(candidate + p, valid, 0) * weight, valid)
    tl.store(output + n + p, tl.load(target + p, valid, 0) * weight, valid)
    tl.store(output + 2 * n + p, tl.load(delta + p, valid, 0) * weight, valid)
    tl.store(output + 3 * n + p, weight, valid)


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
    return lost, extra


@triton.jit
def _stronger(error, largest, largest_denominator):
    denominator = error + 5.
    # Compare the ratios with their rounded denominators. If the rounded
    # products tie, FMA recovers their exact rounding residuals for comparison.
    # The nonzero products of these float32-derived errors are normal float64s.
    left = error * largest_denominator
    right = largest * denominator
    left_residual = tl.fma(error, largest_denominator, -left)
    right_residual = tl.fma(largest, denominator, -right)
    greater = (left > right) | ((left == right) & (left_residual > right_residual))
    return tl.where(greater, error, largest), tl.where(greater, denominator, largest_denominator)


@triton.jit
def _directions(candidate, target, support, lost, extra, h, w, row_blocks, masked: tl.constexpr,
                initial: tl.constexpr, diag_norm: tl.constexpr, packed: tl.constexpr, block: tl.constexpr):
    y, x, p, valid = _stencil_positions(h, w, row_blocks, packed, block)
    c, t = tl.load(candidate + p, valid, 0), tl.load(target + p, valid, 0)
    if masked:
        supported = valid & tl.load(support + p, valid, 0)
    else:
        supported = valid
    largest_loss = tl.full((block,), 0., tl.float64)
    largest_gain = tl.full((block,), 0., tl.float64)
    loss_denominator = tl.full((block,), 5., tl.float64)
    gain_denominator = tl.full((block,), 5., tl.float64)
    for direction in tl.static_range(8):
        cell = direction if direction < 4 else direction + 1
        dy, dx = cell // 3 - 1, cell % 3 - 1
        q = p + dy * w + dx
        use = supported & (y + dy >= 0) & (y + dy < h) & (x + dx >= 0) & (x + dx < w)
        if masked:
            use &= tl.load(support + q, use, 0)
        cg, tg = tl.load(candidate + q, use, 0), tl.load(target + q, use, 0)
        loss, gain = _loss_gain(c, t, cg, tg, dy != 0 and dx != 0, diag_norm, block)
        largest_loss, loss_denominator = _stronger(tl.where(use, loss, 0.), largest_loss, loss_denominator)
        largest_gain, gain_denominator = _stronger(tl.where(use, gain, 0.), largest_gain, gain_denominator)
    # Rounding the winning ratio gives the maximum of the rounded ratios.
    largest_loss = largest_loss / loss_denominator
    largest_gain = largest_gain / gain_denominator
    if not initial:
        largest_loss = tl.maximum(largest_loss.to(tl.float32), tl.load(lost + p, valid, 0))
        largest_gain = tl.maximum(largest_gain.to(tl.float32), tl.load(extra + p, valid, 0))
    tl.store(lost + p, largest_loss, valid)
    tl.store(extra + p, largest_gain, valid)


@triton.jit
def _clamp_index(index, size):
    return tl.minimum(tl.maximum(index, 0), size - 1)


@triton.jit
def _edges(target, upper, lower, strength, h, w, row_blocks, packed: tl.constexpr, block: tl.constexpr):
    y, x, p, valid = _stencil_positions(h, w, row_blocks, packed, block)
    high = tl.load(target + p, valid, 0)
    low = high
    # At the true image edges clamping and reflection visit the same values
    # for an extrema filter, so no halo is required.
    for dy in tl.static_range(-1, 2):
        for dx in tl.static_range(-1, 2):
            if dy != 0 or dx != 0:
                q = _clamp_index(y + dy, h) * w + _clamp_index(x + dx, w)
                value = tl.load(target + q, valid, 0)
                high, low = tl.maximum(high, value), tl.minimum(low, value)
    tl.store(upper + p, high, valid)
    tl.store(lower + p, low, valid)
    tl.store(strength + p, _visible(high - low, 6.), valid)


@triton.jit
def _max9(strength, output, h, w, row_blocks, vertical: tl.constexpr,
          packed: tl.constexpr, block: tl.constexpr):
    y, x, p, valid = _stencil_positions(h, w, row_blocks, packed, block)
    strongest = tl.load(strength + p, valid, 0)
    for d in tl.static_range(-4, 5):
        if d != 0:
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

    def upload(image, dtype=np.float32):
        contiguous = np.asarray(image, dtype=dtype, order="C")
        # A stacker or the decoder can return a read-only view.
        if not contiguous.flags.writeable:
            contiguous = contiguous.copy()
        return torch.from_numpy(contiguous).to(device, non_blocking=True)

    h, w = t.shape
    n = h * w
    block = 128
    grid = (triton.cdiv(n, block),)
    row_blocks = triton.cdiv(w, block)
    # Row tiles share one vertical coordinate. Pack very narrow images to keep
    # blocks from spending most of their lanes on unused columns.
    packed = w < block // 2
    stencil_grid = grid if packed else (h * row_blocks,)
    output = torch.empty((5, h, w), device=device, dtype=torch.float32)
    tone, lost, extra, boundary, channel_range = output.unbind()

    # Range uses the uploaded RGB only once. Release those buffers before the
    # lightness passes so the allocator can reuse them on the current stream.
    raw_c, raw_t = upload(candidate), upload(target)
    _range[grid](raw_c, raw_t, channel_range, n, block, enable_fp_fusion=False)
    del raw_c, raw_t

    c, t, delta = (upload(value) for value in (c, t, delta))
    masked = support is not None
    mask = upload(support, np.bool_) if masked else t
    edge_buffers = t.new_empty((4, h, w))
    if masked:
        # Edge stencils run after the blurs, so their buffers can hold the
        # masked inputs during filtering.
        _mask_sources[grid](c, t, delta, mask, edge_buffers, n, block)
        c_source, t_source, delta_source, mask_source = edge_buffers.unbind()
    else:
        c_source, t_source, delta_source = c, t, delta
    scratch, cs, ts, local = (torch.empty_like(t) for _ in range(4))
    denominator = torch.empty_like(t) if masked else t

    def gaussian(image, destination, sigma, normalize=False):
        weights, radius = _weights(sigma, device)
        _gaussian[stencil_grid](image, weights, scratch, denominator, h, w, row_blocks,
                                radius, True, h >= radius, False, packed, block, enable_fp_fusion=False)
        _gaussian[stencil_grid](scratch, weights, destination, denominator, h, w, row_blocks,
                                radius, False, w >= radius, normalize, packed, block, enable_fp_fusion=False)

    _directions[stencil_grid](c, t, mask, lost, extra, h, w, row_blocks, masked, True,
                              sqrt(2), packed, block, enable_fp_fusion=False)
    for sigma in (1.1, 2.5):
        if masked:
            gaussian(mask_source, denominator, sigma)
        gaussian(c_source, cs, sigma, masked)
        gaussian(t_source, ts, sigma, masked)
        _directions[stencil_grid](cs, ts, mask, lost, extra, h, w, row_blocks, masked, False,
                                  sqrt(2), packed, block, enable_fp_fusion=False)
    # The denominator for the last direction pass is shared with the tone map.
    gaussian(delta_source, local, 2.5, masked)
    fine, broad = cs, ts
    for sigma, destination in ((.85, fine), (10, broad)):
        if masked:
            gaussian(mask_source, denominator, sigma)
        gaussian(delta_source, destination, sigma, masked)

    upper, lower, edge, near = edge_buffers.unbind()
    _edges[stencil_grid](t, upper, lower, edge, h, w, row_blocks, packed, block)
    _max9[stencil_grid](edge, scratch, h, w, row_blocks, True, packed, block)
    _max9[stencil_grid](scratch, near, h, w, row_blocks, False, packed, block)
    _combine[grid](c, delta, fine, local, broad, edge, near, upper, lower,
                   tone, extra, boundary, n, block, enable_fp_fusion=False)

    # One transfer into pinned host memory avoids staging five large pageable
    # copies. The copy is synchronous so NumPy can read the result immediately.
    if n >= 1_000_000:
        try:
            host = torch.empty(output.shape, dtype=output.dtype, device="cpu", pin_memory=True)
        except RuntimeError:
            pass  # Pinned memory may be unavailable even when CUDA is usable.
        else:
            return tuple(host.copy_(output).numpy())
    return tuple(output.cpu().numpy())
