import numpy as np
import pytest

from bokehforge import scoring


@pytest.fixture
def native():
    return pytest.importorskip("bokehforge._scoring_native")


@pytest.mark.parametrize("shape", [(1, 1), (1, 7), (8, 1), (11, 13), (129, 23)])
@pytest.mark.parametrize("mask", ["none", "empty", "holes"])
def test_native_contrast_matches_numpy_at_each_pixel(monkeypatch, native, shape, mask):
    rng = np.random.default_rng(198)
    candidate, target = (rng.uniform(-5, 100, shape).astype(np.float32) for _ in range(2))
    support = None if mask == "none" else np.zeros(shape, dtype=bool)
    if mask == "holes":
        support[...] = rng.random(shape) > 0.25
    with monkeypatch.context() as reference:
        reference.setattr(scoring, "_scoring_native", None)
        expected = scoring._detail(candidate, target, scoring._supported_blur(support), support)
    actual = scoring._detail(candidate, target, scoring._supported_blur(support), support)
    for left, right in zip(actual, expected, strict=True):
        np.testing.assert_array_equal(left.view(np.uint32), right.view(np.uint32))


@pytest.mark.parametrize("layout", ["contiguous", "strided", "byteswapped"])
def test_native_range_matches_numpy_with_finite_float32_extremes(monkeypatch, native, layout):
    rng = np.random.default_rng(124)
    fmax = np.finfo(np.float32).max
    fmin = np.nextafter(np.float32(0), np.float32(1))
    values = np.array([-fmax, -1, -fmin, -0.0, 0, fmin, .006, .012, .5, .988, .994, 1, 2, fmax], dtype=np.float32)
    candidate, target = (rng.choice(values, (129, 19, 3)) for _ in range(2))
    if layout == "strided":
        candidate, target = candidate[::-1, ::2], target[:, ::-2]
    elif layout == "byteswapped":
        candidate, target = (array.astype(array.dtype.newbyteorder("S")) for array in (candidate, target))
    with monkeypatch.context() as reference:
        reference.setattr(scoring, "_scoring_native", None)
        expected = scoring._range_damage(candidate, target)
    actual = scoring._range_damage(candidate, target)
    np.testing.assert_array_equal(actual.view(np.uint32), expected.view(np.uint32))


def test_native_contrast_falls_back_for_strided_images_and_support(monkeypatch, native):
    rng = np.random.default_rng(194)
    candidate, target = (rng.uniform(0, 100, (7, 18)).astype(np.float32) for _ in range(2))
    candidate, target = candidate[::-1, 1::2], target[:, ::2]
    support = (rng.random((7, 18)) > 0.2)[:, ::2]
    with monkeypatch.context() as reference:
        reference.setattr(scoring, "_scoring_native", None)
        expected = scoring._detail(candidate, target, scoring._supported_blur(support), support)
    actual = scoring._detail(candidate, target, scoring._supported_blur(support), support)
    for left, right in zip(actual, expected, strict=True):
        np.testing.assert_array_equal(left.view(np.uint32), right.view(np.uint32))
