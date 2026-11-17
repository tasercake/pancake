"""Ensure a CPU-only target selection cannot hide native host source regions."""

import copy
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest


class AssuranceCoverageTests(unittest.TestCase):
    """Check independent CPU coverage and the complete native source universe."""

    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)
        self.report = {"data": [{"totals": {
            key: {"count": 1, "covered": 1} for key in ("lines", "functions", "regions", "branches")},
            "functions": [{"name": "shared_launch", "filenames": [str(self.root / "src/lib.rs")],
                           "regions": [[1, 1, 2, 1, 1, 0, 0, 0]]}]}]}

    def check(self, cpu, native):
        for name, report in (("cpu", cpu), ("native", native)):
            (self.root / (name + ".json")).write_text(json.dumps(report))
        return subprocess.run([
            sys.executable, str(Path(__file__).with_name("compare_assurance_coverage.py")),
            str(self.root), str(self.root / "cpu.json"), str(self.root / "native.json")],
            text=True, capture_output=True, check=False)

    def test_same_covered_source_passes(self):
        result = self.check(self.report, self.report)
        self.assertEqual(result.returncode, 0, result.stderr)

    def test_native_only_host_region_fails_even_when_cpu_totals_are_complete(self):
        native = copy.deepcopy(self.report)
        native["data"][0]["functions"][0]["regions"].append([4, 1, 5, 1, 1, 0, 0, 0])
        result = self.check(self.report, native)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("native host regions absent", result.stderr)

    def test_uncovered_cpu_branch_fails(self):
        cpu = copy.deepcopy(self.report)
        cpu["data"][0]["totals"]["branches"]["covered"] = 0
        result = self.check(cpu, self.report)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("uncovered CPU branches", result.stderr)

    def test_empty_source_universe_cannot_pass(self):
        report = copy.deepcopy(self.report)
        report["data"][0]["functions"] = []
        result = self.check(report, report)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("no production source", result.stderr)

    def test_uncovered_native_branch_still_fails(self):
        native = copy.deepcopy(self.report)
        native["data"][0]["totals"]["branches"]["covered"] = 0
        result = self.check(self.report, native)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("uncovered native branches", result.stderr)

    def test_new_production_source_layout_requires_review(self):
        native = copy.deepcopy(self.report)
        native["data"][0]["functions"][0]["filenames"] = [str(self.root / "host.rs")]
        result = self.check(self.report, native)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("unclassified source", result.stderr)


if __name__ == "__main__":
    unittest.main()
