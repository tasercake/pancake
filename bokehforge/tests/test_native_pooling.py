import subprocess
import sys

import numpy as np
import pytest

from bokehforge import scoring


@pytest.mark.parametrize("masked", [False, True])
def test_pooling_preserves_weighted_tiles_ties_and_threshold(masked, monkeypatch):
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

    def maps(candidate, target, support, **kwargs):
        return tuple(damage.copy() for _ in scoring.FAMILY_WEIGHTS), {}

    monkeypatch.setattr(scoring, "_maps", maps)
    image = np.zeros((*shape, 3), dtype=np.float32)
    kwargs = {"details": "tiles", "inspection": (weights, .1)}
    actual = scoring._measure(image, image, support, **kwargs)
    monkeypatch.setattr(scoring, "_scoring_native", None)
    fallback = scoring._measure(image, image, support, **kwargs)
    assert actual == fallback
    assert actual.score.hex() == fallback.score.hex()

    weighted = damage * weights
    covered = weighted if support is None else weighted[support]
    tone = actual.explanation["families"]["tone"]
    assert tone["affected_fraction"].hex() == float(np.mean(covered >= .25, dtype=np.float64)).hex()
    assert tone["pool"]["mean"]["value"].hex() == float(scoring._supported_mean(weighted, support)).hex()
    prefix = scoring._integral(weighted if support is None else weighted * support)
    support_prefix = scoring._integral(support) if masked else None
    for grid in actual.explanation["tile_grids"].values():
        x, y = np.asarray(grid["x_edges"]), np.asarray(grid["y_edges"])
        expected = (prefix[np.ix_(y[1:], x[1:])] - prefix[np.ix_(y[:-1], x[1:])]
                    - prefix[np.ix_(y[1:], x[:-1])] + prefix[np.ix_(y[:-1], x[:-1])])
        expected = np.clip(expected / grid["nominal_pixels"], 0, scoring.INSPECTION_MAX_MULTIPLIER)
        values = np.asarray(grid["values"]["tone"])
        np.testing.assert_array_equal(values.view(np.uint64), expected.ravel().view(np.uint64))
        if masked:
            area = scoring._tile_totals(support_prefix, y[:-1], y[1:], x[:-1], x[1:])
            np.testing.assert_array_equal(grid["covered_pixels"], area.ravel())
    tail = tone["pool"]["tail"]
    ties = np.asarray(actual.explanation["tile_grids"][tail["grid_id"]]["values"]["tone"])
    assert np.count_nonzero(ties == tail["cutoff"]) > 1


@pytest.mark.parametrize("masked", [False, True])
@pytest.mark.parametrize("layout", ["contiguous", "crop", "strided", "byteswapped"])
def test_rgb_diagnostics_keep_float64_results(masked, layout, monkeypatch):
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
        support[2, 5] = True
    exposure = scoring._rgb_exposure(candidate, support)
    rmse = scoring._rgb_rmse(candidate, target, support)
    monkeypatch.setattr(scoring, "_scoring_native", None)
    assert exposure.hex() == scoring._rgb_exposure(candidate, support).hex()
    assert rmse.hex() == scoring._rgb_rmse(candidate, target, support).hex()


@pytest.mark.parametrize("shape", [(1, 1), (1, 193), (117, 1), (213, 183)])
def test_native_rgb_means_match_the_serial_numpy_reduction(shape):
    native = scoring._scoring_native
    if native is None:
        pytest.skip("Requires the optional native scoring extension")
    rng = np.random.default_rng(605)
    image = np.ldexp(rng.integers(-4, 5, size=(*shape, 3)).astype(np.float32),
                     rng.integers(-149, 123, size=(*shape, 3)))
    support = rng.random(shape) > .4
    support[0, 0] = True
    for mask in [None, support]:
        expected = scoring._supported_mean(image, mask, axis=(0, 1))
        actual = np.asarray(native.rgb_means(image, mask))
        np.testing.assert_array_equal(actual.view(np.uint64), expected.view(np.uint64))


def test_native_buffers_require_matching_pod_formats():
    native = pytest.importorskip("bokehforge._scoring_native")
    image = np.zeros((2, 4, 3), dtype=np.float32)
    mask = np.ones(image.shape[:2], dtype=np.bool_)
    with pytest.raises(ValueError):
        native.rgb_means(image.view(np.int32), mask)
    with pytest.raises(ValueError):
        native.rgb_means(image, mask.view(np.uint8))

    y, x = np.array([0, 2], dtype=np.intp), np.array([0, 4], dtype=np.intp)
    compact = np.empty((2, 2), dtype=np.float64)
    for dtype in map(np.dtype, ("i", "l", "q")):
        if dtype.itemsize == y.itemsize:
            native.tile_integral(mask, None, y.astype(dtype), x.astype(dtype), compact, .25)
            np.testing.assert_array_equal(compact, [[0., 0.], [0., 8.]])
    with pytest.raises(ValueError):
        native.tile_integral(mask, None, y.view(np.uintp), x, compact, .25)

    for dtype in (np.uint64, np.dtype(np.float64).newbyteorder()):
        output = np.full((3, 2, 4), 123, dtype=dtype)
        with pytest.raises(ValueError):
            native.rgb_square_difference(image, image, output)
        assert (output == 123).all()

    xyz = np.zeros((1, 3, 8), dtype=np.float32)
    strided_output = np.full((3, 9), 123, dtype=np.int32)[:, :8]
    with pytest.raises(ValueError):
        native.lab_planes(xyz, xyz, strided_output)
    assert (strided_output == 123).all()


def test_native_rejects_object_output():
    pytest.importorskip("bokehforge._scoring_native")
    # Isolate object storage in case a regression writes over its references.
    script = """
import os
import numpy as np
from bokehforge import _scoring_native as native

image = np.ones((2, 4, 3), dtype=np.float32)
try:
    if np.dtype(object).itemsize == 8:
        output = np.empty((3, 2, 4), dtype=object)
        native.rgb_square_difference(image, image / 2, output)
    else:
        output = np.empty((2, 4), dtype=object)
        native.range_damage(image, image / 2, output)
except ValueError:
    pass
else:
    os._exit(1)
"""
    subprocess.run([sys.executable, "-c", script], check=True)
