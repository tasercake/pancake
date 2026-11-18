"""Project independent RGB16 inputs onto interior FemtoVG-wgpu screen regions."""

import numpy as np
import tifffile


def load_tiff16(path):
    """Decode the native TIFF independently and recover the app's preview texture."""
    with tifffile.TiffFile(path) as image:
        if len(image.pages) != 1 or image.pages[0].photometric.name != "RGB":
            raise ValueError("one independent RGB TIFF image is required")
        pixels = image.pages[0].asarray()
    if pixels.dtype != np.uint16 or pixels.ndim != 3 or pixels.shape[-1] != 3:
        raise ValueError("independent preview needs genuine native RGB16 pixels")
    height, width, _ = pixels.shape
    step = (max(width, height) + 2047) // 2048
    preview = ((pixels[::step, ::step].astype(np.uint32) + 128) // 257).astype(np.uint8)
    return preview, (width, height)


def _axis(origin, extent, start, count, samples):
    # Slint snaps the final FemtoVG draw origin to the nearest physical pixel.
    snapped = np.copysign(np.floor(abs(origin) + 0.5), origin)
    centers = (np.arange(start, start + count) + 0.5 - snapped) * samples / extent - 0.5
    indices = np.floor(centers).astype(np.intp)
    if indices.min() < 0 or indices.max() >= samples - 1:
        raise ValueError("choose a witness region interior to both rendered states")
    return indices, centers - indices


def render_region(texture, geometry, region):
    """Project physical pixel centers through the pinned linear texture sampler."""
    x, y, width, height = geometry
    left, top, columns, rows = region
    if min(width, height, columns, rows) <= 0:
        raise ValueError("positive rendered size and witness extent are required")
    xs, horizontal = _axis(x, width, left, columns, texture.shape[1])
    ys, vertical = _axis(y, height, top, rows, texture.shape[0])
    h = horizontal[None, :, None]
    v = vertical[:, None, None]
    upper = texture[ys[:, None], xs] * (1 - h) + texture[ys[:, None], xs + 1] * h
    lower = (
        texture[ys[:, None] + 1, xs] * (1 - h) + texture[ys[:, None] + 1, xs + 1] * h
    )
    return np.floor(upper * (1 - v) + lower * v + 0.5).astype(np.uint8)


def calibrate_patch(observed, expected, tolerance):
    """Require actual displayed RGB to match the independent fixed patch."""
    if observed.shape != expected.shape or observed.ndim != 3 or observed.shape[2] != 3:
        raise ValueError("same nonempty observed and expected RGB regions are required")
    difference = int(np.abs(observed.astype(np.int16) - expected).max())
    if difference > tolerance:
        raise ValueError(
            f"observed pixels differ from independent projection by {difference}"
        )
    return difference
