"""Independent projection contracts for the sealed FemtoVG display."""

import tempfile
import unittest
from pathlib import Path

import numpy as np
import tifffile
from native_witness import calibrate_patch, load_tiff16, render_region


class PhysicalImageWitness(unittest.TestCase):
    def test_rgb16_decode_retains_native_geometry_and_preview_quantization(self):
        source = np.zeros((2, 2049, 3), dtype=np.uint16)
        source[0, 0] = (0, 32768, 65535)
        source[0, 1] = (0, 0, 0)
        source[0, 2] = (128, 129, 64250)
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "sample.tif"
            tifffile.imwrite(path, source, photometric="rgb", metadata=None)
            texture, native = load_tiff16(path)
        self.assertEqual(native, (2049, 2))
        self.assertEqual(texture.shape, (1, 1025, 3))
        np.testing.assert_array_equal(texture[0, :2], [[0, 128, 255], [0, 1, 250]])

    def test_interior_physical_pixels_use_bilinear_texel_centers(self):
        values = np.arange(16, dtype=np.uint8).reshape(4, 4) * 8
        raster = np.repeat(values[:, :, None], 3, axis=2)
        geometry = (10.3, 19.7, 8.0, 8.0)
        region = (12, 22, 2, 2)
        expected = np.repeat(np.array([[30, 34], [46, 50]])[:, :, None], 3, axis=2)
        actual = render_region(raster, geometry, region)
        np.testing.assert_array_equal(actual, expected)
        self.assertEqual(calibrate_patch(actual, expected, tolerance=1), 0)
        with self.assertRaisesRegex(ValueError, "independent projection"):
            calibrate_patch(actual, expected + 3, tolerance=1)
        with self.assertRaisesRegex(ValueError, "interior"):
            render_region(raster, geometry, (10, 20, 1, 1))


if __name__ == "__main__":
    unittest.main()
