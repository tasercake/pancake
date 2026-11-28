import subprocess
import sys

import numpy as np
import pytest
import tifffile
from PIL import Image

from bokehforge import _rust
from bokehforge.bench import _load_candidate


@pytest.mark.parametrize(
    ("compression", "predictor", "tile", "byteorder", "bigtiff"),
    [
        (None, None, None, "<", False),
        ("deflate", 3, None, "<", False),
        ("deflate", 3, (16, 16), ">", True),
        ("lzw", 3, (16, 16), "<", False),
        ("zstd", 3, None, ">", True),
        ("packbits", None, None, ">", False),
    ],
)
def test_native_delivery_preserves_tiff_samples(tmp_path, compression, predictor, tile, byteorder, bigtiff):
    path = tmp_path / "delivery.tif"
    rng = np.random.default_rng(613)
    pixels = rng.uniform(-0.4, 1.2, (17, 23, 3)).astype(np.float32)
    pixels[0, 0] = [np.finfo(np.float32).max, -0.0, -np.finfo(np.float32).max]
    tifffile.imwrite(path, pixels, photometric="rgb", tile=tile, byteorder=byteorder, bigtiff=bigtiff,
                     compression=compression, predictor=predictor)
    actual = _rust.read_delivery_tiff(path)
    assert actual.dtype == np.float32
    assert actual.flags.c_contiguous and actual.flags.aligned
    np.testing.assert_array_equal(actual.view(np.uint32), pixels.view(np.uint32))


@pytest.mark.parametrize("compression", [None, "deflate"])
def test_native_delivery_reads_parallel_strips_and_tail(tmp_path, compression):
    path = tmp_path / "wide.tif"
    rng = np.random.default_rng(67)
    pixels = np.broadcast_to(rng.uniform(-1, 2, (1121, 1, 3)).astype(np.float32), (1121, 1251, 3))
    options = {"compression": compression, "predictor": 3, "rowsperstrip": 13} if compression else {}
    tifffile.imwrite(path, pixels, photometric="rgb", **options)
    actual = _rust.read_delivery_tiff(path)
    np.testing.assert_array_equal(actual.view(np.uint32), pixels.view(np.uint32))


@pytest.mark.parametrize("dtype", [np.uint8, np.uint16, np.int32, np.float16, np.float64])
def test_native_delivery_does_not_convert_file_dtype(tmp_path, dtype):
    path = tmp_path / "dtype.tif"
    pixels = np.arange(18, dtype=dtype).reshape(2, 3, 3)
    tifffile.imwrite(path, pixels, photometric="rgb", byteorder=">")
    actual = _rust.read_delivery_tiff(path)
    assert actual.dtype == pixels.dtype
    np.testing.assert_array_equal(actual, pixels)


def test_unfamiliar_lossless_tiff_codec_uses_existing_decoder(tmp_path):
    path = tmp_path / "lzma.tif"
    pixels = np.arange(18, dtype=np.float32).reshape(2, 3, 3)
    tifffile.imwrite(path, pixels, photometric="rgb", compression="lzma")
    assert _rust.read_delivery_tiff(path) is None
    np.testing.assert_array_equal(_load_candidate(path, {}), pixels)


@pytest.mark.parametrize(
    ("shape", "tiff_options", "message"),
    [
        ((3, 3, 3), {"extratags": [(274, "H", 1, 6, False)]}, "orientation 6"),
        ((3, 3, 3), {"planarconfig": "separate"}, "SYX axis order"),
        ((3, 3), {"photometric": "minisblack"}, "MINISBLACK photometric"),
    ],
)
def test_invalid_delivery_records_error_and_path(tmp_path, shape, tiff_options, message):
    path = tmp_path / "invalid.tif"
    tifffile.imwrite(path, np.zeros(shape, dtype=np.float32), **{"photometric": "rgb", **tiff_options})
    row = {}
    assert _load_candidate(path, row) is None
    assert row["status"] == "invalid_output"
    assert message in row["error"] and str(path) in row["error"]


def test_multipage_and_missing_delivery_report_distinct_statuses(tmp_path):
    path = tmp_path / "multi.tif"
    with tifffile.TiffWriter(path) as writer:
        for _ in range(3):
            writer.write(np.zeros((2, 3, 3), dtype=np.float32), photometric="rgb")
    for name, expected, message in ((path, "invalid_output", "3 TIFF pages"),
                                    (tmp_path / "missing.tif", "missing_output", "FileNotFoundError")):
        row = {}
        assert _load_candidate(name, row) is None
        assert row["status"] == expected
        assert message in row["error"] and str(name) in row["error"]


def test_standard_reference_and_delivery_read_without_python_tiff_codecs(tmp_path):
    reference, delivery = tmp_path / "source.png", tmp_path / "delivery.tif"
    Image.new("RGB", (3, 2), (64, 128, 160)).save(reference)
    tifffile.imwrite(delivery, np.full((2, 3, 3), 0.25, dtype=np.float32), photometric="rgb",
                     compression="deflate", predictor=3)
    script = """
from pathlib import Path
import sys

class BlockTiffImports:
    def find_spec(self, fullname, *args):
        if fullname.split(".")[0] in {"tifffile", "imagecodecs"}:
            raise AssertionError(f"Unexpected codec import {fullname}")

assert "tifffile" not in sys.modules and "imagecodecs" not in sys.modules
sys.meta_path.insert(0, BlockTiffImports())
from bokehforge.bench import _read_candidate
from bokehforge.data._canonical import decode

[(reference, alpha, metadata)] = decode(Path(sys.argv[1]), "srgb")
assert reference.shape == (2, 3, 3) and alpha is None and metadata["decoder"] == "Pillow"
assert (_read_candidate(Path(sys.argv[2])) == 0.25).all()
"""
    subprocess.run([sys.executable, "-c", script, str(reference), str(delivery)], check=True)
