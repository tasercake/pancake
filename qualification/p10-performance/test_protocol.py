"""Contracts for a qualified comparison, using small independent inputs."""

import hashlib
import json
import sys
import tempfile
import unittest
from pathlib import Path

import numpy as np
import tifffile
from protocol import (
    Campaign,
    evaluate_latency,
    pixel_metrics,
    profile_result,
    verify_frames,
    verify_output,
)
from resources import measure

ROOT = Path(__file__).resolve().parents[2]
TARGETS = ROOT / "pancake/docs/performance-targets.json"


class FrozenProfileContracts(unittest.TestCase):
    def setUp(self):
        self.campaign = Campaign.load(TARGETS)

    def test_exact_profile_and_schedule(self):
        self.assertEqual(len(list(self.campaign.profiles("cpu"))), 13)
        profiles = {profile.key: profile for profile in self.campaign.profiles("gpu")}
        recipe = profiles["D150.hybrid.gpu"]
        self.assertEqual(
            recipe.options,
            {
                "device": "gpu",
                "method": "hybrid",
                "align": True,
                "alignment_mode": "breathing",
                "alignment_reference": "first",
                "kernel_size": 6,
                "pyramid_levels": 8,
                "scale_factor": 6,
                "slab_size": 40,
                "slab_overlap": 4,
                "bit_depth": 16,
                "auto": False,
                "auto_crop": True,
                "quality_report": False,
            },
        )
        self.assertEqual(
            profiles["E9.exposure_fusion.gpu"].options["alignment_mode"], "euclidean"
        )
        schedule = self.campaign.schedule()
        self.assertEqual(len(schedule), 34)
        self.assertEqual([kind for kind, _, _ in schedule].count("warmup"), 4)
        self.assertEqual([kind for kind, _, _ in schedule].count("warm"), 20)
        self.assertEqual([kind for kind, _, _ in schedule].count("cold"), 10)
        for first, second, next_first in zip(
            schedule[::2], schedule[1::2], schedule[2::2]
        ):
            self.assertEqual(first[:2], second[:2])
            self.assertNotEqual(first[2], second[2])
            self.assertNotEqual(first[2], next_first[2])

    def test_complete_samples_remain_pending_without_device_memory_or_quality(self):
        samples = []
        for kind, number, target in self.campaign.schedule():
            candidate = target == "candidate"
            samples.append(
                {
                    "kind": kind,
                    "number": number,
                    "target": target,
                    "valid": True,
                    "elapsed_ns": 4_000 if candidate else 10_000,
                    "peak_rss_bytes": 50 if candidate else 100,
                    "peak_gpu_bytes": 51 if candidate else 100,
                }
            )
        result = profile_result(self.campaign, "gpu", samples)
        self.assertEqual(result["gates"]["warm_median"]["status"], "pass")
        self.assertEqual(result["gates"]["peak_gpu"]["status"], "fail")
        self.assertEqual(result["status"], "fail")
        samples[0]["peak_gpu_bytes"] = None
        result = profile_result(self.campaign, "gpu", samples)
        self.assertEqual(result["gates"]["peak_gpu"]["status"], "pending")
        self.assertEqual(result["status"], "pending")
        self.assertEqual(
            profile_result(self.campaign, "cpu", samples[:-1])["status"], "pending"
        )
        samples[0]["valid"] = False
        self.assertEqual(
            profile_result(self.campaign, "cpu", samples)["status"], "fail"
        )

    def test_latency_uses_per_operation_samples_and_deadlines(self):
        native_kinds = ["source", "result", "pan", "zoom", "comparison"]
        viewer = [
            {
                "condition": condition,
                "action": action,
                "dispatched_ns": 100,
                "presented_ns": 33_000_100,
            }
            for condition in ("idle", "loaded")
            for action in native_kinds
            for _ in range(40)
        ]
        lifecycle = [
            {
                "operation": operation,
                "dispatched_ns": 100,
                "acknowledged_ns": 200_000_100,
                "settled_ns": 400_000_100,
            }
            for operation in ("pause", "cancel")
            for _ in range(30)
        ]
        result = evaluate_latency(self.campaign, viewer, lifecycle)
        self.assertEqual(result["viewer"]["idle"]["status"], "pass")
        self.assertEqual(result["lifecycle"]["pause"]["status"], "pass")
        lifecycle[-1]["acknowledged_ns"] = 251_000_100
        self.assertEqual(
            evaluate_latency(self.campaign, viewer, lifecycle)["lifecycle"]["cancel"][
                "status"
            ],
            "fail",
        )
        self.assertEqual(
            evaluate_latency(self.campaign, viewer[1:], lifecycle)["viewer"]["idle"][
                "status"
            ],
            "pending",
        )


class ImageContracts(unittest.TestCase):
    def test_manifest_order_and_immutability(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / "b").write_bytes(b"second")
            (root / "a").write_bytes(b"first")
            records = [
                {
                    "path": name,
                    "bytes": (root / name).stat().st_size,
                    "sha256": hashlib.sha256((root / name).read_bytes()).hexdigest(),
                }
                for name in ("b", "a")
            ]
            self.assertEqual(verify_frames(root, records, 2), [root / "b", root / "a"])
            (root / "a").write_bytes(b"wrong")
            with self.assertRaisesRegex(ValueError, "SHA-256 mismatch"):
                verify_frames(root, records, 2)

    def test_export_precision_and_pixel_metrics(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            expected = np.arange(300, dtype=np.uint16).reshape(10, 10, 3)
            a, b, fake = (
                root / f"{name}.tif" for name in ("reference", "candidate", "promoted")
            )
            tifffile.imwrite(a, expected, photometric="rgb")
            tifffile.imwrite(b, expected + 1, photometric="rgb")
            tifffile.imwrite(
                fake, expected.astype(np.uint16) // 2 * 257, photometric="rgb"
            )
            self.assertEqual(verify_output(a, (10, 10, 3))["shape"], [10, 10, 3])
            self.assertEqual(pixel_metrics(a, b)["mean_absolute_u16"], 1.0)
            self.assertEqual(pixel_metrics(a, b)["maximum_absolute_u16"], 1)
            with self.assertRaisesRegex(ValueError, "promoted 8-bit"):
                verify_output(fake, (10, 10, 3))


class MeasurementContracts(unittest.TestCase):
    @unittest.skipUnless(sys.platform == "linux", "Linux procfs measurement lane")
    def test_real_process_boundaries_and_group_memory(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            out = root / "finished"
            result = measure(
                [
                    sys.executable,
                    "-c",
                    (
                        "from pathlib import Path; import time; blob=bytearray(4<<20); "
                        "time.sleep(0.08); Path('finished').write_text('ok')"
                    ),
                ],
                cwd=root,
                env={},
                directory=root,
                timeout=5,
                gpu=None,
            )
            self.assertEqual(result["returncode"], 0)
            self.assertFalse(result["timed_out"])
            self.assertGreaterEqual(result["elapsed_ns"], 80_000_000)
            self.assertGreaterEqual(result["kernel_process_peak_rss_bytes"], 4 << 20)
            raw = [
                json.loads(line)
                for line in (root / "resources.jsonl").read_text().splitlines()
            ]
            self.assertGreaterEqual(max(sample["rss_bytes"] for sample in raw), 4 << 20)
            if result["maximum_sample_gap_ns"] > 20_000_000:
                self.assertIsNone(result["peak_rss_bytes"])
            else:
                self.assertGreaterEqual(result["peak_rss_bytes"], 4 << 20)
            self.assertEqual(out.read_text(), "ok")
