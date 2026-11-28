import numpy as np
import pytest

from bokehforge import _rust, scoring


@pytest.mark.parametrize("shape", [(1, 1), (1, 7), (8, 1), (11, 13), (319, 445)])
@pytest.mark.parametrize("mask", ["none", "empty", "holes"])
def test_native_lightness_matches_scipy_at_each_pixel(monkeypatch, shape, mask):
    rng = np.random.default_rng(198)
    candidate, target = (rng.uniform(-5, 100, shape).astype(np.float32) for _ in range(2))
    support = None if mask == "none" else np.zeros(shape, dtype=bool)
    if mask == "holes":
        support[...] = rng.random(shape) > 0.25
    with monkeypatch.context() as reference:
        reference.setattr(scoring, "_rust", None)
        expected = scoring._lightness_damage(candidate, target, candidate - target, support)
    mask_bytes = None if support is None else support.view(np.uint8)
    actual = _rust.lightness_damage(
        candidate, target, candidate - target, mask_bytes, scoring._lightness_weights()
    )
    for left, right in zip(actual, expected, strict=True):
        np.testing.assert_array_equal(left.view(np.uint32), right.view(np.uint32))


def test_native_lightness_uses_precomputed_extrema_and_nonzero_mask_bytes(monkeypatch):
    rng = np.random.default_rng(199)
    shape = (25, 68)
    candidate, target, strength, near_edge = (
        rng.random(shape, dtype=np.float32) for _ in range(4)
    )
    delta = candidate - target
    extrema = target + np.float32(.25), target - np.float32(.25), strength, near_edge
    mask_bytes = (rng.random(shape) > .2).astype(np.uint8) * 255
    with monkeypatch.context() as reference:
        reference.setattr(scoring, "_rust", None)
        expected = scoring._lightness_damage(candidate, target, delta, mask_bytes.view(np.bool_), extrema)
    actual = _rust.lightness_damage(
        candidate, target, delta, mask_bytes, scoring._lightness_weights(), extrema
    )
    for left, right in zip(actual, expected, strict=True):
        np.testing.assert_array_equal(left.view(np.uint32), right.view(np.uint32))


@pytest.mark.parametrize("layout", ["contiguous", "crop", "strided", "byteswapped"])
def test_native_range_matches_numpy_with_finite_float32_extremes(monkeypatch, layout):
    rng = np.random.default_rng(124)
    fmax = np.finfo(np.float32).max
    fmin = np.nextafter(np.float32(0), np.float32(1))
    values = np.array([-fmax, -1, -fmin, -0.0, 0, fmin, .006, .012, .5, .988, .994, 1, 2, fmax], dtype=np.float32)
    candidate, target = (rng.choice(values, (129, 19, 3)) for _ in range(2))
    if layout == "crop":
        candidate, target = candidate[:, 1:-1], target[:, 1:-1]
    elif layout == "strided":
        candidate, target = candidate[::-1, ::2], target[:, ::-2]
    elif layout == "byteswapped":
        candidate, target = (array.astype(array.dtype.newbyteorder("S")) for array in (candidate, target))
    with monkeypatch.context() as reference:
        reference.setattr(scoring, "_rust", None)
        expected = scoring._range_damage(candidate, target)
    actual = scoring._range_damage(candidate, target)
    np.testing.assert_array_equal(actual.view(np.uint32), expected.view(np.uint32))


def test_native_lightness_falls_back_for_strided_images_and_support(monkeypatch):
    rng = np.random.default_rng(194)
    candidate, target = (rng.uniform(0, 100, (7, 18)).astype(np.float32) for _ in range(2))
    candidate, target = candidate[::-1, 1::2], target[:, ::2]
    support = (rng.random((7, 18)) > 0.2)[:, ::2]
    with monkeypatch.context() as reference:
        reference.setattr(scoring, "_rust", None)
        expected = scoring._lightness_damage(candidate, target, candidate - target, support)
    actual = scoring._lightness_damage(candidate, target, candidate - target, support)
    for left, right in zip(actual, expected, strict=True):
        np.testing.assert_array_equal(left.view(np.uint32), right.view(np.uint32))


@pytest.mark.parametrize("argument", ["image", "weights", "extrema"])
def test_native_lightness_rejects_unaligned_inputs(argument):
    def unaligned(shape, dtype):
        buffer = bytearray(np.prod(shape) * np.dtype(dtype).itemsize + 1)
        return np.ndarray(shape, dtype=dtype, buffer=buffer, offset=1)

    shape = (3, 5)
    image = np.zeros(shape, dtype=np.float32)
    candidate = unaligned(shape, np.float32) if argument == "image" else image
    weights = list(scoring._lightness_weights())
    if argument == "weights":
        weights[0] = unaligned(weights[0].shape, np.float64)
    extrema = (image, image, image, unaligned(shape, np.float32)) if argument == "extrema" else None
    with pytest.raises(ValueError, match="aligned"):
        _rust.lightness_damage(candidate, image, image, None, weights, extrema)


@pytest.mark.parametrize("layout", ["fortran", "byteswapped"])
def test_native_lightness_rejects_incompatible_array_layout(layout):
    image = np.zeros((3, 5), dtype=np.float32)
    candidate = np.asfortranarray(image) if layout == "fortran" else image.astype(image.dtype.newbyteorder("S"))
    with pytest.raises((TypeError, ValueError)):
        _rust.lightness_damage(candidate, image, image, None, scoring._lightness_weights())
