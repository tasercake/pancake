"""Independent small-case contracts for generated high-bit focal scale data."""

import hashlib
import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

import numpy as np
from t50_generated import Recipe, cleanup_owned, render_tile
from t50_oracle import scalar_pixel


class RenderContracts(unittest.TestCase):
    """Require real 16-bit data, deterministic defocus and known depth truth."""

    def setUp(self):
        self.recipe = Recipe(width=37, height=23, frames=7, seed=20261117, tile=16)

    def test_integer_reference_hash_and_native_coordinates(self):
        expected = np.array(
            [
                [scalar_pixel(self.recipe, 3, x, y) for x in range(37)]
                for y in range(23)
            ],
            dtype=np.uint16,
        )
        actual = render_tile(self.recipe, 3, 0, 0, 23, 37)
        self.assertEqual(actual.dtype, np.uint16)
        self.assertEqual(
            hashlib.sha256(actual.astype("<u2").tobytes()).hexdigest(),
            hashlib.sha256(expected.astype("<u2").tobytes()).hexdigest(),
        )

    def test_focus_frames_are_distinct_and_preserve_genuine_u16(self):
        images = [render_tile(self.recipe, focus, 0, 0, 23, 37) for focus in range(7)]
        self.assertEqual(
            len({hashlib.sha256(image.tobytes()).hexdigest() for image in images}), 7
        )
        for image in images:
            self.assertGreater(np.unique(image).size, 256)
            self.assertTrue(np.any(image % 257 != 0))

    def test_each_known_depth_is_exactly_sharp_in_its_focus_frame(self):
        truth = render_tile(self.recipe, None, 0, 0, 23, 37)
        for focus in range(7):
            image = render_tile(self.recipe, focus, 0, 0, 23, 37)
            mask = np.arange(37) * 7 // 37 == focus
            np.testing.assert_array_equal(image[:, mask], truth[:, mask])

    def test_tiles_equal_direct_native_render(self):
        whole = render_tile(self.recipe, 2, 0, 0, 23, 37)
        tiled = np.empty_like(whole)
        for y in range(0, 23, 16):
            for x in range(0, 37, 16):
                height, width = min(16, 23 - y), min(16, 37 - x)
                tiled[y : y + height, x : x + width] = render_tile(
                    self.recipe, 2, y, x, height, width
                )
        np.testing.assert_array_equal(tiled, whole)

    def test_invalid_geometry_focus_and_tile_are_rejected(self):
        for values in (
            {"width": 0},
            {"height": 0},
            {"frames": 0},
            {"tile": 15},
            {"frames": 100},
        ):
            with self.subTest(values=values), self.assertRaises(ValueError):
                Recipe(
                    **({"width": 37, "height": 23, "frames": 7, "tile": 16} | values)
                )
        for focus, y, x, height, width in (
            (7, 0, 0, 1, 1),
            (-1, 0, 0, 1, 1),
            (0, 0, 37, 1, 1),
            (0, 22, 0, 2, 1),
        ):
            with self.subTest(focus=focus, y=y, x=x), self.assertRaises(ValueError):
                render_tile(self.recipe, focus, y, x, height, width)


class StorageAndCleanupContracts(unittest.TestCase):
    """Exercise real ZIP16 streaming and leave unrelated files untouched."""

    def command(self, output, receipt):
        return [
            sys.executable,
            str(Path(__file__).with_name("t50_generated.py")),
            "--width",
            "37",
            "--height",
            "23",
            "--frames",
            "7",
            "--tile",
            "16",
            "--reserve-gib",
            "0",
            "--output",
            str(output),
            "--receipt",
            str(receipt),
        ]

    def test_complete_small_run_verifies_hashes_and_deletes_owned_images(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            output, receipt = root / "created", root / "receipt.json"
            result = subprocess.run(
                self.command(output, receipt),
                capture_output=True,
                text=True,
                check=False,
            )
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertTrue(
                receipt.is_file(),
                "Missing full-stream verification and cleanup receipt",
            )
            data = json.loads(receipt.read_text())
            self.assertEqual(data["dataset_kind"], "GENERATED_SYNTHETIC_FOCAL_SCALE")
            self.assertEqual(data["frame_count"], 7)
            self.assertEqual(len(data["files"]), 9)
            self.assertEqual(data.get("maximum_simultaneous_image_files"), 1)
            self.assertEqual(
                len(
                    {
                        f["decoded_pixel_sha256"]
                        for f in data["files"]
                        if f["role"] == "focus_frame"
                    }
                ),
                7,
            )
            for frame in data["files"]:
                self.assertEqual(
                    frame["generated_pixel_sha256"], frame["decoded_pixel_sha256"]
                )
                self.assertEqual(
                    frame.get("independent_zlib_pixel_sha256"),
                    frame["generated_pixel_sha256"],
                )
                self.assertEqual(frame["compression"], "ADOBE_DEFLATE")
                self.assertEqual(frame["dtype"], "uint16")
                if frame["role"] == "focus_frame":
                    self.assertGreater(frame["unique_values"], 256)
            self.assertEqual(data["cleanup"]["deleted_image_count"], 9)
            self.assertEqual(list(output.glob("*.tif")), [])

    def test_existing_unowned_directory_is_never_modified(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "keep.tif").write_bytes(b"original user bytes")
            result = subprocess.run(
                self.command(root, root / "receipt.json"),
                capture_output=True,
                text=True,
                check=False,
            )
            self.assertNotEqual(
                result.returncode, 0, "Existing output directory must be rejected"
            )
            self.assertEqual((root / "keep.tif").read_bytes(), b"original user bytes")
            self.assertFalse((root / "receipt.json").exists())

    def test_reserve_and_byte_caps_reject_before_creating_images(self):
        for option, value in (
            ("--reserve-gib", "100000"),
            ("--max-generated-bytes", "1"),
        ):
            with (
                self.subTest(option=option),
                tempfile.TemporaryDirectory() as directory,
            ):
                root = Path(directory)
                output, receipt = root / "created", root / "receipt.json"
                result = subprocess.run(
                    self.command(output, receipt) + [option, value],
                    capture_output=True,
                    text=True,
                    check=False,
                )
                self.assertNotEqual(result.returncode, 0)
                self.assertEqual(list(output.glob("*.tif")), [])

    def test_cleanup_cap_counts_retained_bytes_not_deleted_history(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            output, receipt = root / "created", root / "receipt.json"
            # This fits one frame, while all 32 compressed images exceed it.
            one_frame_bound = 4 * 256**2 * 6 * 2 + 4 * 1024 * 1024
            result = subprocess.run(
                self.command(output, receipt)
                + [
                    "--width",
                    "512",
                    "--height",
                    "512",
                    "--frames",
                    "32",
                    "--tile",
                    "256",
                    "--max-generated-bytes",
                    str(one_frame_bound),
                ],
                capture_output=True,
                text=True,
                check=False,
            )
            self.assertEqual(result.returncode, 0, result.stderr)
            data = json.loads(receipt.read_text())
            self.assertEqual(data["cleanup"]["deleted_image_count"], 34)
            self.assertEqual(data["cleanup"]["remaining_tiff_count"], 0)
            self.assertGreater(
                sum(record["bytes"] for record in data["files"]), one_frame_bound
            )

    def test_cleanup_rejects_corruption_before_deleting_any_file(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "ownership.json").write_text(json.dumps({"token": "test-token"}))
            (root / "first.tif").write_bytes(b"first")
            (root / "second.tif").write_bytes(b"changed")
            records = [
                {
                    "name": "first.tif",
                    "file_sha256": hashlib.sha256(b"first").hexdigest(),
                },
                {
                    "name": "second.tif",
                    "file_sha256": hashlib.sha256(b"original").hexdigest(),
                },
            ]
            with self.assertRaises(ValueError):
                cleanup_owned(root, "test-token", records)
            self.assertEqual((root / "first.tif").read_bytes(), b"first")
            self.assertEqual((root / "second.tif").read_bytes(), b"changed")

    def test_cleanup_rejects_wrong_owner_and_path_escape(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "ownership.json").write_text(json.dumps({"token": "test-token"}))
            for token, name in (
                ("wrong-owner", "first.tif"),
                ("test-token", "../first.tif"),
            ):
                with (
                    self.subTest(token=token, name=name),
                    self.assertRaises(ValueError),
                ):
                    cleanup_owned(
                        root, token, [{"name": name, "file_sha256": "0" * 64}]
                    )


if __name__ == "__main__":
    unittest.main()
