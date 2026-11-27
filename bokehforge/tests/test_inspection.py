import numpy as np
from PIL import Image
import pytest

from bokehforge.inspection import reference_preview


@pytest.mark.parametrize("shape, dtype", [((37, 41, 3), "<f4"), ((1001, 1025, 3), ">f4")])
def test_reference_preview_matches_interleaved_encoding_and_resize(shape, dtype):
    cutoffs = (np.arange(255) + .5) / 255
    boundaries = np.where(cutoffs <= .04045, cutoffs / 12.92, ((cutoffs + .055) / 1.055) ** 2.4).astype(np.float32)
    samples = np.concatenate((boundaries, np.nextafter(boundaries, np.float32(-np.inf)),
                              np.nextafter(boundaries, np.float32(np.inf)),
                              [-np.finfo(np.float32).max, -1, 0, .0031308, 1, np.finfo(np.float32).max]))
    reference = np.resize(samples, shape).astype(dtype)[::-1, :, ::-1]
    height, width = shape[:2]
    pixels = np.empty(shape, dtype=np.uint8)
    for start in range(0, height, 128):
        rows = slice(start, start + 128)
        linear = np.clip(reference[rows], 0, 1)
        srgb = np.where(linear <= .0031308, 12.92 * linear, 1.055 * linear ** (1 / 2.4) - .055)
        pixels[rows] = np.rint(srgb * 255)
    scale = min(1, 640 / max(height, width))
    size = (round(width * scale), round(height * scale))
    expected = Image.fromarray(pixels).resize(size, Image.Resampling.LANCZOS)

    preview = reference_preview(reference)

    assert preview.mode == expected.mode == "RGB"
    assert preview.size == size
    assert preview.tobytes() == expected.tobytes()
