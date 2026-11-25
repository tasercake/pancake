"""Display rendering and interchange with external stackers."""

from contextlib import closing
from io import BytesIO
from pathlib import Path

import numpy as np
from PIL import Image
import tifffile

from ..data._canonical import decode, encode_tiff


def srgb_samples(pixels, bits):
    linear = np.clip(pixels, 0, 1)
    low = linear <= 0.0031308
    linear[low] *= 12.92
    linear[~low] = 1.055 * linear[~low] ** (1 / 2.4) - 0.055
    return np.rint(linear * ((1 << bits) - 1)).astype(f"uint{bits}")


def preview(pixels, size, thumbnail):
    image = Image.fromarray(srgb_samples(pixels, 8))
    if size:
        image.thumbnail((size, size), Image.Resampling.LANCZOS, reducing_gap=2)
    with BytesIO() as stream:
        if thumbnail:
            image.save(stream, format="JPEG", quality=90, subsampling=0)
        else:
            image.save(stream, format="PNG", compress_level=1)
        return stream.getvalue()


def stage_frame(path, pixels):
    tifffile.imwrite(path, srgb_samples(pixels, 16), photometric="rgb",
                     compression="deflate", compressionargs={"level": 2}, maxworkers=1)


def normalize_output(delivery, destination):
    with closing(decode(delivery, "srgb_assumed", page=0)) as decoded:
        pixels, _, metadata = next(decoded)
    destination.parent.mkdir(parents=True, exist_ok=True)
    pending = destination.with_suffix(".tmp")
    try:
        encode_tiff(pending, pixels)
        pending.replace(destination)
    finally:
        pending.unlink(missing_ok=True)
    return {"width": pixels.shape[1], "height": pixels.shape[0],
            "native_dtype": metadata["native_dtype"],
            "output_transfer": "embedded ICC" if "icc_transform" in metadata else "assumed sRGB"}


def output_pixels(path: Path):
    return tifffile.imread(path, maxworkers=1)
