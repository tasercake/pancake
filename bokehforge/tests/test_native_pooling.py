import subprocess
import sys

import numpy as np
import pytest

from bokehforge import _rust
from bokehforge.scoring import INSPECTION_MAX_MULTIPLIER
from scoring_reference import XYZ, integral, supported_mean, tile_totals


@pytest.mark.parametrize("masked", [False, True])
def test_pooling_preserves_weighted_tiles_ties_and_threshold(masked):
    rng = np.random.default_rng(734)
    shape = (137, 117)
    weights = np.ones(shape, dtype=np.float32)
    weights[32:64, 32:64] = 1.5
    damage = np.full(shape, np.nextafter(np.float32(.25), np.float32(0)), dtype=np.float32)
    damage[8:24] = .75
    damage[40:64, 56:80] = .25
    damage[:, 96:] = np.exp2(-rng.integers(0, 150, (shape[0], 21)).astype(np.float32))
    support = rng.random(shape) > .2 if masked else None
    if masked:
        support[8:24] = True
        support[-16:, -16:] = False

    weighted = damage * weights
    covered = weighted if support is None else weighted[support]
    prefix = integral(weighted if support is None else weighted * support)
    support_prefix = integral(support) if masked else None

    def axis(length, side, offset):
        return np.unique(np.r_[0, np.arange(offset, length, side, dtype=np.intp), length])

    grids = [(side, axis(shape[0], side, dy), axis(shape[1], side, dx))
             for side in (8, 32) for dy in (0, side // 2) for dx in (0, side // 2)]
    ys, xs = (np.unique(np.concatenate([grid[i] for grid in grids])) for i in (1, 2))
    requests = []
    for side, y, x in grids:
        area = None if support_prefix is None else tile_totals(support_prefix, y[:-1], y[1:], x[:-1], x[1:])
        requests.append(((np.searchsorted(ys, y), np.searchsorted(xs, x)),
                         None if area is None else area.astype(np.int32), side ** 2))
    mask = None if support is None else support.view(np.uint8)

    def pool(keep_tiles):
        return _rust.pool_tiles(weighted, mask, (ys, xs), requests,
                                (.25, INSPECTION_MAX_MULTIPLIER, .01, keep_tiles, np.getbufsize()))

    total, affected, actual = pool(True)
    normal_total, normal_affected, normal = pool(False)
    assert (total, affected) == (normal_total, normal_affected)
    assert affected == np.count_nonzero(covered >= .25)
    assert (total / covered.size).hex() == float(supported_mean(weighted, support)).hex()
    tails = []
    for grid, request, actual_grid, normal_grid in zip(grids, requests, actual, normal, strict=True):
        side, y, x = grid
        expected = tile_totals(prefix, y[:-1], y[1:], x[:-1], x[1:]) / side ** 2
        expected = np.clip(expected, 0, INSPECTION_MAX_MULTIPLIER).ravel()
        statistics, values, _ = actual_grid
        assert actual_grid[0] == normal_grid[0]
        np.testing.assert_array_equal(values.view(np.uint64), expected.ravel().view(np.uint64))
        worst, worst_index, exact_tail, cutoff, count, eligible_count = statistics
        assert (worst, worst_index) == (expected.max(), expected.argmax())
        area = request[1]
        eligible = expected if area is None else expected[area.ravel() > 0]
        assert eligible_count == len(eligible)
        assert count == max(1, int(np.ceil(.01 * len(eligible))))
        upper = np.sort(eligible)[-count:]
        assert cutoff == upper[0]
        if exact_tail is not None:
            assert exact_tail == pytest.approx(upper.mean(), rel=0, abs=1e-14)
        tails.append((upper.mean(), np.count_nonzero(eligible == cutoff)))
    assert max(tails, key=lambda row: row[0])[1] > 1


def test_native_tile_prefix_requires_signed_aligned_edges():
    native = _rust
    mask = np.ones((2, 4), dtype=np.uint8)
    # A NumPy bool view can contain noncanonical true bytes.
    mask[0, 0] = 7
    y, x = np.array([0, 2], dtype=np.intp), np.array([0, 4], dtype=np.intp)
    for dtype in map(np.dtype, ("i", "l", "q")):
        if dtype.itemsize == y.itemsize:
            compact = native.coverage_tile_prefix(mask, (y.astype(dtype), x.astype(dtype)))
            np.testing.assert_array_equal(compact, [[0., 0.], [0., 8.]])

    swapped = y.byteswap().view(y.dtype.newbyteorder())
    unaligned = np.ndarray(y.shape, dtype=y.dtype, buffer=bytearray(y.nbytes + 1), offset=1)
    unaligned[:] = y
    for invalid in (y.view(np.uintp), swapped, unaligned):
        with pytest.raises((TypeError, ValueError)):
            native.coverage_tile_prefix(mask, (invalid, x))


@pytest.mark.parametrize("masked", [False, True])
@pytest.mark.parametrize("layout", ["contiguous", "crop", "strided", "byteswapped"])
def test_rgb_diagnostics_keep_float64_results(masked, layout):
    rng = np.random.default_rng(98)
    shape = (67, 61, 3)
    values = [np.ldexp(rng.integers(-8, 9, size=shape).astype(np.float32),
                       rng.integers(-149, 123, size=shape)) for _ in range(2)]
    values[0][2, 5, 0] = np.finfo(np.float32).max
    values[1][2, 5, 0] = -np.finfo(np.float32).max
    if layout == "crop":
        values = [image[:, 2:-1] for image in values]
    elif layout == "strided":
        values = [image[::-1, ::2] for image in values]
    elif layout == "byteswapped":
        values = [image.byteswap().view(image.dtype.newbyteorder()) for image in values]
    candidate, target = values
    support = rng.random(target.shape[:2]) > .3 if masked else None
    if masked:
        support[:5] = True
    mask = None if support is None else support.view(np.uint8)
    features, diagnostics = _rust._measure_overlap(candidate, target, mask)
    exposure = []
    for image in (candidate, target):
        ordered = np.asarray(image, dtype=np.float32, order="C")
        means = supported_mean(ordered, support, axis=(0, 1))
        exposure.append(np.maximum(means, 0) @ XYZ[1])
    expected = np.log2((exposure[0] + 1e-5) / (exposure[1] + 1e-5))
    observed = diagnostics["views"]["native"][0]["global_exposure_delta_stops"]
    assert observed == pytest.approx(expected, rel=1e-14)
    means = [supported_mean(np.square(np.subtract(c, t, dtype=np.float64)), support)
             for c, t in zip(np.moveaxis(candidate, -1, 0), np.moveaxis(target, -1, 0), strict=True)]
    assert diagnostics["linear_rgb_rmse"] == pytest.approx(np.sqrt(np.mean(means)), rel=1e-14)
    assert all(np.isfinite(value) for value in features.values())


@pytest.mark.parametrize("shape", [(1, 1), (1, 193), (117, 1), (213, 183)])
def test_native_rgb_means_match_the_serial_numpy_reduction(shape):
    native = _rust
    rng = np.random.default_rng(605)
    image = np.ldexp(rng.integers(-4, 5, size=(*shape, 3)).astype(np.float32),
                     rng.integers(-149, 123, size=(*shape, 3)))
    support = rng.random(shape) > .4
    support[0, 0] = True
    for mask in [None, support]:
        expected = supported_mean(image, mask, axis=(0, 1))
        actual = np.asarray(native.rgb_means(image, None if mask is None else mask.view(np.uint8)))
        np.testing.assert_array_equal(actual.view(np.uint64), expected.view(np.uint64))


@pytest.mark.parametrize("masked", [False, True])
def test_native_rgb_mse_preserves_pairwise_blocks_across_padded_rows(masked):
    rng = np.random.default_rng(186)
    shape = (259, 523, 3)
    candidate, target = (
        np.ldexp(rng.uniform(-1, 1, shape).astype(np.float32),
                 rng.integers(-20, 10, shape, dtype=np.int32))[:, 2:-1]
        for _ in range(2)
    )
    support = np.ones(candidate.shape[:2], dtype=np.uint8) if masked else None
    if masked:
        support.ravel()[65_537::10_003] = 0
    expected = [supported_mean(np.square(np.subtract(c, t, dtype=np.float64)),
                               None if support is None else support.view(np.bool_))
                for c, t in zip(np.moveaxis(candidate, -1, 0), np.moveaxis(target, -1, 0), strict=True)]
    actual = _rust.rgb_mse(candidate, target, support)
    np.testing.assert_array_equal(np.asarray(actual).view(np.uint64), np.asarray(expected).view(np.uint64))


def test_native_pixel_buffers_require_matching_pod_formats():
    native = _rust
    image = np.zeros((2, 4, 3), dtype=np.float32)
    mask = np.ones(image.shape[:2], dtype=np.uint8)
    with pytest.raises(TypeError):
        native.rgb_means(image.view(np.int32), mask)
    with pytest.raises(TypeError):
        native.rgb_means(image, mask.view(np.bool_))

    for dtype in (np.uint32, np.dtype(np.float32).newbyteorder()):
        output = np.full((3, 8), 123, dtype=dtype)
        with pytest.raises(TypeError):
            native.lab_xyz((image,), XYZ, output, 0)
        assert (output == 123).all()

    xyz = np.zeros((3, 8), dtype=np.float32)
    strided_output = np.full((3, 9), 123, dtype=np.int32)[:, :8]
    with pytest.raises(TypeError):
        native.lab_planes(xyz, xyz, tuple(strided_output))
    assert (strided_output == 123).all()


def test_native_rejects_object_output():
    # Isolate object storage in case a regression writes over its references.
    script = """
import os
import numpy as np
from bokehforge import _rust as native

xyz = np.zeros((3, 8), dtype=np.float32)
output = np.empty_like(xyz, dtype=object)
try:
    native.lab_planes(xyz, xyz, tuple(output))
except TypeError:
    pass
else:
    os._exit(1)
"""
    subprocess.run([sys.executable, "-c", script], check=True)
