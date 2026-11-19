"""Regression contracts for Linux sampling cadence and attribution."""

import json
import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest.mock import patch

import resources


class ResourceCadence(unittest.TestCase):
    def test_tighter_absolute_slots_keep_phase_and_skip_missed_slots(self):
        self.assertEqual(
            resources._next_sample_due_ns(3_000_000, 3_100_000, 3_000_000), 6_000_000,
        )
        self.assertEqual(
            resources._next_sample_due_ns(3_000_000, 12_000_000, 3_000_000), 15_000_000,
        )

    def test_nominal_rss_interval_cannot_make_a_late_observation_eligible(self):
        rows = [
            {"scheduled_ns": 0, "rss_elapsed_ns": 2_000_000, "rss_bytes": 25},
            {"scheduled_ns": 3_000_000, "rss_elapsed_ns": 22_000_001, "rss_bytes": 40},
        ]
        result = resources._summarize_samples(rows, 25_000_000, gpu=False, errors=[])
        self.assertEqual(result["maximum_rss_sample_gap_ns"], 20_000_001)
        self.assertEqual(result["observed_peak_rss_bytes"], 40)
        self.assertIsNone(result["peak_rss_bytes"])
        rows[-1]["rss_elapsed_ns"] -= 1
        result = resources._summarize_samples(rows, 25_000_000, gpu=False, errors=[])
        self.assertEqual(result["maximum_rss_sample_gap_ns"], 20_000_000)
        self.assertEqual(result["peak_rss_bytes"], 40)

    def test_invalid_cpu_interval_and_gpu_override_are_rejected_before_launch(self):
        for interval, gpu in ((0, None), (1, None), (9, None), (2.5, None), (3, object())):
            with (
                self.subTest(interval=interval, gpu=gpu is not None),
                patch("resources.subprocess.Popen") as launch,
            ):
                with self.assertRaisesRegex(ValueError, "interval"):
                    resources.measure(
                        [],
                        cwd=Path("."),
                        env={},
                        directory=Path("."),
                        timeout=3,
                        gpu=gpu,
                        rss_sample_interval_ms=interval,
                    )
                launch.assert_not_called()

    @unittest.skipUnless(sys.platform == "linux", "Linux per-thread affinity")
    def test_rss_sampler_uses_its_own_cpu_and_the_job_inherits_the_original_mask(self):
        original = os.sched_getaffinity(0)
        if len(original) < 2:
            self.skipTest("two allowed logical CPUs are needed")
        application_cpu, collector_cpu = sorted(original)[:2]
        job = (
            "import os, time\nprint(sorted(os.sched_getaffinity(0)))\ntime.sleep(0.08)"
        )
        with tempfile.TemporaryDirectory() as scratch:
            directory = Path(scratch)
            sampler_masks = set()

            def sample(session):
                sampler_masks.add(tuple(sorted(os.sched_getaffinity(0))))
                return snapshot(session)

            snapshot = resources._session_memory
            os.sched_setaffinity(0, {application_cpu})
            try:
                with patch("resources._session_memory", side_effect=sample):
                    result = resources.measure(
                        [sys.executable, "-c", job],
                        cwd=directory,
                        env=os.environ.copy(),
                        directory=directory,
                        timeout=3,
                        gpu=None,
                        rss_sampler_cpu=collector_cpu,
                    )
                self.assertEqual(os.sched_getaffinity(0), {application_cpu})
            finally:
                os.sched_setaffinity(0, original)

            self.assertEqual(result["returncode"], 0)
            self.assertEqual(
                json.loads((directory / "stdout.txt").read_text()), [application_cpu]
            )
            self.assertEqual(result["rss_sampler_affinity"], [collector_cpu])
            self.assertEqual(sampler_masks, {(application_cpu,), (collector_cpu,)})
            rows = [
                json.loads(line)
                for line in (directory / "resources.jsonl").read_text().splitlines()
            ]
            self.assertGreater(len(rows), 1)
            self.assertEqual(rows[0]["scheduled_ns"], 0)
            self.assertTrue(all(row["snapshot_thread_cpu_ns"] > 0 for row in rows))
            self.assertTrue(all(row["scheduled_ns"] % 8_000_000 == 0 for row in rows))

    @unittest.skipUnless(sys.platform == "linux", "Linux per-thread affinity")
    def test_refuses_an_rss_sampler_cpu_shared_with_the_job(self):
        with tempfile.TemporaryDirectory() as scratch:
            directory = Path(scratch)
            with self.assertRaisesRegex(ValueError, "outside the common job affinity"):
                resources.measure(
                    [sys.executable, "-c", "raise AssertionError('must not launch')"],
                    cwd=directory,
                    env=os.environ.copy(),
                    directory=directory,
                    timeout=3,
                    gpu=None,
                    rss_sampler_cpu=min(os.sched_getaffinity(0)),
                )
            self.assertFalse((directory / "stdout.txt").exists())

    @unittest.skipUnless(sys.platform == "linux", "Linux per-thread affinity")
    def test_unavailable_rss_sampler_cpu_aborts_before_job_launch(self):
        with (
            tempfile.TemporaryDirectory() as scratch,
            patch(
                "resources.os.sched_setaffinity", side_effect=OSError("cpuset blocked")
            ),
            patch("resources.subprocess.Popen") as launch,
        ):
            directory = Path(scratch)
            outside = max(os.sched_getaffinity(0)) + 1
            with self.assertRaisesRegex(ValueError, "cpuset blocked"):
                resources.measure(
                    [sys.executable, "-c", "raise AssertionError('must not launch')"],
                    cwd=directory,
                    env=os.environ.copy(),
                    directory=directory,
                    timeout=3,
                    gpu=None,
                    rss_sampler_cpu=outside,
                )
            launch.assert_not_called()

    def test_each_trace_keeps_its_own_metric_and_launch_to_exit_cadence(self):
        rows = [
            {"rss_elapsed_ns": 5_000_000, "rss_bytes": 25},
            {"rss_elapsed_ns": 17_000_000, "rss_bytes": 40},
            {"rss_elapsed_ns": 30_000_000, "rss_bytes": 30},
            {"gpu_elapsed_ns": 9_000_000, "gpu_bytes": 4},
            {"gpu_elapsed_ns": 32_000_000, "gpu_bytes": 9},
        ]
        result = resources._summarize_samples(rows, 36_000_000, gpu=True, errors=[])
        self.assertEqual(result["peak_rss_bytes"], 40)
        self.assertEqual(result["maximum_rss_sample_gap_ns"], 13_000_000)
        self.assertIsNone(result["peak_gpu_bytes"])
        self.assertEqual(result["maximum_gpu_sample_gap_ns"], 23_000_000)

    def test_completion_timestamps_apply_the_limit_to_each_counter(self):
        rows = [
            {
                "rss_elapsed_ns": 5_000_000,
                "gpu_elapsed_ns": 6_000_000,
                "rss_bytes": 25,
                "gpu_bytes": 4,
            },
            {
                "rss_elapsed_ns": 20_000_000,
                "gpu_elapsed_ns": 31_000_000,
                "rss_bytes": 40,
                "gpu_bytes": 9,
            },
            {
                "rss_elapsed_ns": 35_000_000,
                "gpu_elapsed_ns": 49_000_000,
                "rss_bytes": 30,
                "gpu_bytes": 7,
            },
        ]
        result = resources._summarize_samples(rows, 40_000_000, gpu=True, errors=[])
        self.assertEqual(result["maximum_rss_sample_gap_ns"], 15_000_000)
        self.assertEqual(result["maximum_gpu_sample_gap_ns"], 25_000_000)
        self.assertEqual(result["maximum_sample_gap_ns"], 25_000_000)
        self.assertEqual(result["peak_rss_bytes"], 40)
        self.assertIsNone(result["peak_gpu_bytes"])
        self.assertEqual(result["observed_peak_gpu_bytes"], 9)

    def test_gaps_at_launch_and_exit_invalidate_late_or_missing_counters(self):
        overdue = 21_000_000
        rows = [
            {
                "rss_elapsed_ns": overdue,
                "gpu_elapsed_ns": overdue,
                "rss_bytes": 5,
                "gpu_bytes": None,
            },
            {
                "rss_elapsed_ns": 42_000_000,
                "gpu_elapsed_ns": 43_000_000,
                "rss_bytes": 99,
                "gpu_bytes": 100,
            },
        ]
        result = resources._summarize_samples(rows, 41_000_001, gpu=True, errors=[])
        self.assertEqual(result["maximum_rss_sample_gap_ns"], overdue)
        self.assertEqual(result["maximum_gpu_sample_gap_ns"], overdue)
        self.assertEqual(result["observed_peak_rss_bytes"], 5)
        self.assertIsNone(result["observed_peak_gpu_bytes"])
        self.assertIsNone(result["peak_rss_bytes"])
        self.assertIsNone(result["peak_gpu_bytes"])

    def test_unknown_device_value_and_empty_session_do_not_qualify_memory(self):
        rows = [
            {
                "rss_elapsed_ns": 1,
                "gpu_elapsed_ns": 2,
                "rss_bytes": 0,
                "gpu_bytes": None,
            },
            {"rss_elapsed_ns": 3, "gpu_elapsed_ns": 4, "rss_bytes": 0, "gpu_bytes": 10},
        ]
        elapsed = 20_000_004
        result = resources._summarize_samples(rows, elapsed, gpu=True, errors=[])
        self.assertEqual(result["maximum_gpu_sample_gap_ns"], 20_000_000)
        self.assertEqual(result["observed_peak_gpu_bytes"], 10)
        self.assertIsNone(result["peak_gpu_bytes"])
        cpu = resources._summarize_samples(rows, 4, gpu=False, errors=[])
        self.assertIsNone(cpu["peak_rss_bytes"])

    @unittest.skipUnless(sys.platform == "linux", "Linux session attribution")
    def test_real_processes_are_screened_by_kernel_session_identity(self):
        with subprocess.Popen(
            [sys.executable, "-c", "import time; time.sleep(2)"],
            start_new_session=True,
        ) as unrelated:
            try:
                with patch("resources.os.getsid", wraps=os.getsid) as session:
                    sampled, rss = resources._session_memory(os.getsid(0))
                self.assertIn(os.getpid(), sampled)
                self.assertNotIn(unrelated.pid, sampled)
                session.assert_any_call(unrelated.pid)
                self.assertGreater(rss, 0)
            finally:
                unrelated.terminate()

    def test_gpu_session_uses_kernel_pid_membership_and_deduplicates_contexts(self):
        collector = object.__new__(resources.NvidiaMemory)
        collector.lib = SimpleNamespace(
            nvmlDeviceGetComputeRunningProcesses_v3=None,
            nvmlDeviceGetGraphicsRunningProcesses_v3=None,
        )
        tables = [
            [
                SimpleNamespace(pid=1, memory_bytes=23),
                SimpleNamespace(pid=2, memory_bytes=99),
            ],
            [
                SimpleNamespace(pid=1, memory_bytes=29),
                SimpleNamespace(pid=3, memory_bytes=12),
            ],
        ]
        with (
            patch.object(collector, "_query", side_effect=tables),
            patch("resources.os.getsid", side_effect=[5, 99, 5, ProcessLookupError()]),
        ):
            pids, used = collector.sample_session(5)
        self.assertEqual(pids, {1})
        self.assertEqual(used, 29)
