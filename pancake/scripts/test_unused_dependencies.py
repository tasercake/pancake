"""Exercise the pinned unused-dependency tool with real Cargo manifests."""

import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest


class UnusedDependencyTests(unittest.TestCase):
    """Check first-party scope and fail-closed handling of tool metadata errors."""

    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)
        (self.root / "src").mkdir()
        (self.root / "src/lib.rs").write_text("//! Fixture crate.\n")
        (self.root / "Cargo.toml").write_text(
            '[package]\nname = "policy-fixture"\nversion = "0.1.0"\nedition = "2024"\n[workspace]\n')
        self.metadata = {"workspace_members": ["fixture"], "packages": [
            {"id": "fixture", "manifest_path": str(self.root / "Cargo.toml")}]}
        self.environment = dict(os.environ)
        self.environment["PATH"] = str(Path(__file__).resolve().parents[1] / "target/tools/bin") + os.pathsep + self.environment["PATH"]

    def run_check(self):
        metadata = self.root / "metadata.json"
        metadata.write_text(json.dumps(self.metadata))
        return subprocess.run([
            sys.executable, str(Path(__file__).with_name("check_unused_dependencies.py")),
            str(metadata),
        ], capture_output=True, text=True, check=False, env=self.environment)

    def test_vendor_manifests_are_not_first_party_members(self):
        vendor = self.root / "vendor/broken"
        vendor.mkdir(parents=True)
        (vendor / "Cargo.toml").write_text("malformed manifest")
        result = self.run_check()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertNotIn("error when handling", result.stderr)

    def test_cargo_machete_metadata_error_cannot_pass_with_exit_zero(self):
        (self.root / "Cargo.toml").write_text("malformed manifest")
        result = self.run_check()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("error when handling", result.stderr)


if __name__ == "__main__":
    unittest.main()
