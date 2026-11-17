"""Independent LibRaw, libtiff, libpng and OpenEXR qualification oracles."""

import argparse
import ctypes
import hashlib
import json
from pathlib import Path

import cv2
import numpy as np
import OpenEXR
import rawpy
import tifffile

RGB16 = np.array(
    [0, 1, 255, 256, 257, 258, 1023, 4095, 4096, 16383, 32769, 65535],
    dtype=np.uint16,
).reshape(2, 2, 3)
FLOAT = np.array(
    [
        -0.25,
        0.0,
        0.125,
        1.0,
        2.5,
        65504.0,
        np.nextafter(np.float32(0.5), np.float32(1.0)),
        -1.0,
        1.25,
        32.0,
        1.5,
        0.25,
    ],
    dtype=np.float32,
).reshape(2, 2, 3)


def verify_icc(output):
    """Compare real RGB transforms with independently loaded system Little CMS."""
    values = np.arange(65536, dtype=np.uint16)
    reverse = np.array(
        [int(f"{int(v):016b}"[::-1], 2) for v in values], dtype=np.uint16
    )
    expected_noop = np.stack((values, reverse, values + np.uint16(1)), axis=1)
    actual_noop = np.fromfile(output / "icc-noop.u16", dtype="<u2").reshape(-1, 3)
    np.testing.assert_array_equal(actual_noop, expected_noop)
    cms = ctypes.CDLL("/opt/homebrew/lib/liblcms2.dylib")
    cms.cmsOpenProfileFromMem.argtypes = [ctypes.c_void_p, ctypes.c_uint32]
    cms.cmsOpenProfileFromMem.restype = ctypes.c_void_p
    cms.cmsCreateTransform.argtypes = [
        ctypes.c_void_p,
        ctypes.c_uint32,
        ctypes.c_void_p,
        ctypes.c_uint32,
        ctypes.c_uint32,
        ctypes.c_uint32,
    ]
    cms.cmsCreateTransform.restype = ctypes.c_void_p
    cms.cmsDoTransform.argtypes = [
        ctypes.c_void_p,
        ctypes.c_void_p,
        ctypes.c_void_p,
        ctypes.c_uint32,
    ]
    cms.cmsDoTransform.restype = None
    cms.cmsDeleteTransform.argtypes = [ctypes.c_void_p]
    cms.cmsDeleteTransform.restype = None
    cms.cmsCloseProfile.argtypes = [ctypes.c_void_p]
    cms.cmsCloseProfile.restype = ctypes.c_int
    cms.cmsGetEncodedCMMversion.restype = ctypes.c_uint32
    source_bytes = ctypes.create_string_buffer((output / "srgb.icc").read_bytes())
    target_bytes = ctypes.create_string_buffer(
        (output / "linear-srgb.icc").read_bytes()
    )
    source_profile = cms.cmsOpenProfileFromMem(source_bytes, len(source_bytes) - 1)
    target_profile = cms.cmsOpenProfileFromMem(target_bytes, len(target_bytes) - 1)
    if not source_profile or not target_profile:
        raise AssertionError("Independent LCMS could not parse profiles")
    # TYPE_RGB_16 from installed lcms2.h: COLORSPACE_SH(PT_RGB)|CHANNELS_SH(3)|BYTES_SH(2).
    rgb16_format = (4 << 16) | (3 << 3) | 2
    transform = cms.cmsCreateTransform(
        source_profile, rgb16_format, target_profile, rgb16_format, 1, 0
    )
    if not transform:
        raise AssertionError("Independent LCMS could not create transform")
    pixels = RGB16.reshape(-1, 3).copy()
    expected = np.empty_like(pixels)
    try:
        cms.cmsDoTransform(
            transform, pixels.ctypes.data, expected.ctypes.data, pixels.shape[0]
        )
    finally:
        cms.cmsDeleteTransform(transform)
        cms.cmsCloseProfile(source_profile)
        cms.cmsCloseProfile(target_profile)
    actual = np.fromfile(output / "icc-linear.u16", dtype="<u2").reshape(-1, 3)
    error = int(np.abs(actual.astype(np.int32) - expected.astype(np.int32)).max())
    if error > 1:
        raise AssertionError(
            f"ICC conversion exceeds fixed 1-u16 oracle tolerance: {error}"
        )
    return {
        "equal_profile_exhaustive_noop": True,
        "real_icc_transform_max_u16_error": error,
        "oracle_lcms_encoded_version": cms.cmsGetEncodedCMMversion(),
    }


def prepare(raw_path, output):
    """Freeze the actual LibRaw sensor samples before Rust decodes the CR3."""
    output.mkdir(parents=True, exist_ok=True)
    with rawpy.imread(str(raw_path)) as raw:
        sensor = raw.raw_image.copy()
        sensor.astype("<u2").tofile(output / "libraw-sensor.u16")
        metadata = {
            "input_sha256": hashlib.sha256(raw_path.read_bytes()).hexdigest(),
            "libraw_version": list(rawpy.libraw_version),
            "rawpy_version": rawpy.__version__,
            "sensor_shape": list(sensor.shape),
            "visible_shape": list(raw.raw_image_visible.shape),
            "white_level": raw.white_level,
            "libraw_left_margin": raw.sizes.left_margin,
            "libraw_top_margin": raw.sizes.top_margin,
            "default_crop_left": raw.sizes.crop_left_margin,
            "default_crop_top": raw.sizes.crop_top_margin,
            "default_crop_width": raw.sizes.crop_width,
            "default_crop_height": raw.sizes.crop_height,
            "sensor_samples_sha256": hashlib.sha256(
                sensor.astype("<u2").tobytes()
            ).hexdigest(),
        }
    (output / "libraw-sensor.json").write_text(json.dumps(metadata, indent=2) + "\n")
    bayer = np.arange(768, dtype=np.uint16).reshape(24, 32) * np.uint16(83)
    bayer.astype("<u2").tofile(output / "bayer-expected.u16")
    tifffile.imwrite(
        output / "bayer.dng",
        bayer,
        photometric=32803,
        metadata=None,
        extratags=[
            (271, "s", 0, "Pancake", False),
            (272, "s", 0, "P0 Bayer Probe", False),
            (33421, "H", 2, (2, 2), False),
            (33422, "B", 4, (0, 1, 1, 2), False),
            (50706, "B", 4, (1, 4, 0, 0), False),
            (50707, "B", 4, (1, 1, 0, 0), False),
            (50708, "s", 0, "Pancake P0 Bayer Probe", False),
            (50710, "B", 3, (0, 1, 2), False),
            (50711, "H", 1, 1, False),
            (50714, "H", 1, 0, False),
            (50717, "I", 1, 65535, False),
            (50719, "I", 2, (0, 0), False),
            (50720, "I", 2, (32, 24), False),
            (
                50721,
                "2i",
                9,
                (1, 1, 0, 1, 0, 1, 0, 1, 1, 1, 0, 1, 0, 1, 0, 1, 1, 1),
                False,
            ),
            (50728, "2I", 3, (1, 1, 1, 1, 1, 1), False),
            (50778, "H", 1, 21, False),
            (50829, "I", 4, (0, 0, 24, 32), False),
        ],
    )
    print(json.dumps(metadata))


def verify(output):
    """Validate all pixels and source precision with independent native codecs."""
    for name in (
        "rgb16.png",
        "rgb16.tiff",
        "float32.tiff",
        "float32.exr",
        "rawler-sensor.u16",
    ):
        if not (output / name).is_file():
            raise AssertionError(f"Missing Rust codec output: {name}")
    png = cv2.imread(str(output / "rgb16.png"), cv2.IMREAD_UNCHANGED)
    np.testing.assert_array_equal(png[..., ::-1], RGB16)
    if png.dtype != np.uint16:
        raise AssertionError(f"PNG precision changed: {png.dtype}")
    with tifffile.TiffFile(output / "rgb16.tiff") as image:
        np.testing.assert_array_equal(image.asarray(), RGB16)
        if image.pages[0].bitspersample != 16:
            raise AssertionError("TIFF is not true 16-bit")
        compression = image.pages[0].compression.name
    with tifffile.TiffFile(output / "float32.tiff") as image:
        np.testing.assert_array_equal(
            image.asarray().view(np.uint32), FLOAT.view(np.uint32)
        )
        if image.pages[0].sampleformat != 3 or image.pages[0].bitspersample != 32:
            raise AssertionError("Float TIFF sample type changed")
    with OpenEXR.File(str(output / "float32.exr")) as image:
        pixels = image.channels()["RGB"].pixels
        np.testing.assert_array_equal(pixels.view(np.uint32), FLOAT.view(np.uint32))
        if pixels.dtype != np.float32:
            raise AssertionError("EXR silently changed precision")
    raw_oracle = json.loads((output / "libraw-sensor.json").read_text())
    bayer = np.fromfile(output / "bayer-rust.u16", dtype="<u2")
    expected_bayer = np.fromfile(output / "bayer-expected.u16", dtype="<u2")
    np.testing.assert_array_equal(bayer, expected_bayer)
    with rawpy.imread(str(output / "bayer.dng")) as raw:
        np.testing.assert_array_equal(raw.raw_image.ravel(), expected_bayer)
    rust_bytes = (output / "rawler-sensor.u16").read_bytes()
    digest = hashlib.sha256(rust_bytes).hexdigest()
    if digest != raw_oracle["sensor_samples_sha256"]:
        raise AssertionError("Rawler sensor samples differ from LibRaw oracle")
    result = {
        "status": "pass",
        "rgb16_png_exact": True,
        "rgb16_tiff_exact": True,
        "tiff_compression": compression,
        "float_tiff_bit_exact": True,
        "float_exr_bit_exact": True,
        "full_sensor_libraw_bit_exact": True,
        "generated_bayer_dng_exact": True,
        "sensor_samples_sha256": digest,
        "versions": {
            "opencv": cv2.__version__,
            "numpy": np.__version__,
            "tifffile": tifffile.__version__,
            "OpenEXR": OpenEXR.__version__,
        },
    }
    result.update(verify_icc(output))
    (output / "independent-results.json").write_text(
        json.dumps(result, indent=2) + "\n"
    )
    print(json.dumps(result))


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("action", choices=("prepare", "verify"))
    parser.add_argument("output", type=Path)
    parser.add_argument("--raw", type=Path)
    args = parser.parse_args()
    if args.action == "prepare":
        if args.raw is None:
            parser.error("prepare requires --raw")
        prepare(args.raw, args.output)
    else:
        verify(args.output)
