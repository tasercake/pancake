import numpy as np
import pytest

from bokehforge import _rust
from scoring_reference import lightness_damage, range_damage


@pytest.mark.parametrize("shape", [(1, 1), (1, 7), (8, 1), (11, 13), (319, 445)])
@pytest.mark.parametrize("mask", ["none", "empty", "holes", "full"])
def test_native_lightness_matches_scipy_at_each_pixel(shape, mask):
    rng = np.random.default_rng(198)
    candidate, target = (rng.uniform(-5, 100, shape).astype(np.float32) for _ in range(2))
    mask_bytes = None if mask == "none" else np.zeros(shape, dtype=np.uint8)
    if mask == "holes":
        mask_bytes[rng.random(shape) > 0.25] = 255
    elif mask == "full":
        mask_bytes.fill(255)
    support = None if mask_bytes is None else mask_bytes.view(np.bool_)
    delta = candidate - target
    expected = lightness_damage(candidate, target, delta, support)
    actual = _rust._accelerated_lightness(candidate, target, delta, mask_bytes, "cpu")
    for left, right in zip(actual, expected, strict=True):
        np.testing.assert_allclose(left, right, rtol=2e-6, atol=8e-6)


@pytest.mark.parametrize("layout", ["contiguous", "crop", "strided", "byteswapped"])
def test_native_range_matches_numpy_with_finite_float32_extremes(layout):
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
    expected = range_damage(candidate, target)
    if layout in {"strided", "byteswapped"}:
        candidate, target = (
            np.ascontiguousarray(array, dtype=np.float32) for array in (candidate, target)
        )
    actual = _rust.range_damage(candidate, target)
    np.testing.assert_array_equal(actual.view(np.uint32), expected.view(np.uint32))


def test_native_lightness_matches_reference_for_packed_image_views():
    rng = np.random.default_rng(194)
    candidate, target = (rng.uniform(0, 100, (7, 18)).astype(np.float32) for _ in range(2))
    candidate, target = candidate[::-1, 1::2], target[:, ::2]
    support = (rng.random((7, 18)) > 0.2)[:, ::2]
    delta = candidate - target
    expected = lightness_damage(candidate, target, delta, support)
    actual = _rust._accelerated_lightness(
        np.ascontiguousarray(candidate), np.ascontiguousarray(target), delta,
        np.ascontiguousarray(support).view(np.uint8), "cpu",
    )
    for left, right in zip(actual, expected, strict=True):
        np.testing.assert_allclose(left, right, rtol=2e-6, atol=8e-6)


def test_native_lightness_rejects_unaligned_inputs():
    image = np.zeros((3, 5), dtype=np.float32)
    buffer = bytearray(image.nbytes + 1)
    unaligned = np.ndarray(image.shape, dtype=image.dtype, buffer=buffer, offset=1)
    with pytest.raises(ValueError, match="aligned"):
        _rust._accelerated_lightness(unaligned, image, image, None, "cpu")


@pytest.mark.parametrize("layout", ["fortran", "byteswapped"])
def test_native_lightness_rejects_incompatible_array_layout(layout):
    image = np.zeros((3, 5), dtype=np.float32)
    candidate = np.asfortranarray(image) if layout == "fortran" else image.astype(image.dtype.newbyteorder("S"))
    with pytest.raises((TypeError, ValueError)):
        _rust._accelerated_lightness(candidate, image, image, None, "cpu")
