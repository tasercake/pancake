"""Verify assurance target routing against real Cargo metadata."""

import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest


class AssuranceTargetTests(unittest.TestCase):
    """Every integration target must declare CPU or physical-GPU requirements."""

    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)
        (self.root / "src").mkdir()
        (self.root / "src/lib.rs").write_text("//! Target-routing fixture.\n")
        (self.root / "tests").mkdir()
        (self.root / "Cargo.toml").write_text('[package]\nname="assurance-fixture"\nversion="0.1.0"\nedition="2024"\n[workspace]\n')
        for name in ("cpu_kernel_contract", "gpu_kernel_contract"):
            (self.root / "tests" / (name + ".rs")).write_text("//! Target-routing fixture.\n")

    def route(self, family):
        result = subprocess.run(["cargo", "metadata", "--offline", "--format-version", "1",
                                 "--manifest-path", str(self.root / "Cargo.toml")],
                                text=True, capture_output=True, check=False)
        self.assertEqual(result.returncode, 0, result.stderr)
        (self.root / "metadata.json").write_text(result.stdout)
        return subprocess.run([sys.executable, str(Path(__file__).with_name("assurance_targets.py")),
                               str(self.root / "metadata.json"), family],
                              text=True, capture_output=True, check=False)

    def test_cpu_and_gpu_routes_are_disjoint_and_complete(self):
        cpu, gpu = self.route("cpu"), self.route("gpu")
        self.assertEqual(cpu.returncode, 0, cpu.stderr)
        self.assertEqual(gpu.returncode, 0, gpu.stderr)
        self.assertEqual(cpu.stdout.splitlines(), ["cpu_kernel_contract"])
        self.assertEqual(gpu.stdout.splitlines(), ["gpu_kernel_contract"])

    def add_gui_member(self):
        gui = self.root / "gui"
        (gui / "src").mkdir(parents=True)
        (gui / "src/main.rs").write_text("fn main() {}\n")
        (gui / "tests").mkdir()
        (gui / "tests/cpu_desktop.rs").write_text("//! Desktop target.\n")
        (gui / "Cargo.toml").write_text(
            '[package]\nname="pancake-gui"\nversion="0.1.0"\nedition="2024"\n')
        with (self.root / "Cargo.toml").open("a") as manifest:
            manifest.write('members=["gui"]\n')

    def test_headless_and_gui_split_the_cpu_targets_by_package(self):
        self.add_gui_member()
        expected = {
            "headless": ["cpu_kernel_contract"],
            "gui": ["cpu_desktop"],
            "cpu": ["cpu_desktop", "cpu_kernel_contract"],
            "gpu": ["gpu_kernel_contract"],
        }
        for family, targets in expected.items():
            with self.subTest(family=family):
                result = self.route(family)
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertEqual(result.stdout.splitlines(), targets)

    def test_headless_verifies_the_gui_inventory_before_excluding_it(self):
        self.add_gui_member()
        (self.root / "gui/tests/unclassified.rs").write_text("//! New target.\n")
        result = self.route("headless")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("unclassified integration targets: unclassified", result.stderr)

    def test_unclassified_integration_target_fails_closed(self):
        (self.root / "tests/unclassified.rs").write_text("//! Unclassified fixture.\n")
        result = self.route("cpu")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("unclassified", result.stderr)

    def test_gpu_profile_cannot_pass_without_a_hardware_target(self):
        (self.root / "tests/gpu_kernel_contract.rs").unlink()
        result = self.route("gpu")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("no physical-GPU", result.stderr)


if __name__ == "__main__":
    unittest.main()
