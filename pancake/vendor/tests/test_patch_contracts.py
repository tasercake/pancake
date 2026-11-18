"""Regression contracts for narrow, pinned third-party dependency patches."""

import hashlib
import json
from pathlib import Path
import tomllib
import unittest


VENDOR = Path(__file__).resolve().parents[1]
CUBECL_PACKAGES = ("core", "std", "cpp", "hip", "spirv")


class DependencyPatchContracts(unittest.TestCase):
    """Keep actual dependency identities visible to Cargo and advisory tools."""

    def test_cubecl_uses_maintained_proc_macro_under_existing_import_name(self):
        for package in CUBECL_PACKAGES:
            with self.subTest(package=package):
                path = VENDOR / f"cubecl-{package}-0.11.4" / "Cargo.toml"
                manifest = tomllib.loads(path.read_text())
                dependency = manifest["dependencies"]["paste"]
                self.assertEqual(dependency.get("package"), "pastey")
                self.assertEqual(dependency["version"], "=0.2.3")

    def test_spin_on_has_no_retired_pinning_dependency(self):
        path = VENDOR / "spin_on-0.1.1" / "Cargo.toml"
        manifest = tomllib.loads(path.read_text())
        self.assertNotIn("pin-utils", manifest.get("dependencies", {}))

    def test_vendor_records_exact_original_crate_identities(self):
        provenance = json.loads((VENDOR / "upstream-sources.json").read_text())
        expected = {f"cubecl-{part}" for part in CUBECL_PACKAGES} | {
            "spin_on", "cubecl-macros", "rowan", "oneshot",
        }
        self.assertEqual({p["name"] for p in provenance["packages"]}, expected)
        for package in provenance["packages"]:
            with self.subTest(package=package["name"]):
                manifest = tomllib.loads(
                    (VENDOR / package["directory"] / "Cargo.toml").read_text()
                )
                self.assertEqual(manifest["package"]["name"], package["name"])
                self.assertEqual(manifest["package"]["version"], package["version"])
                self.assertRegex(package["archive_sha256"], r"^[0-9a-f]{64}$")

    def test_oneshot_workspace_resolves_the_reviewed_checkout(self):
        root = VENDOR.parent
        manifest = tomllib.loads((root / "Cargo.toml").read_text())
        self.assertEqual(
            manifest["patch"]["crates-io"].get("oneshot"),
            {"path": "vendor/oneshot-0.2.1"},
        )
        lock = tomllib.loads((root / "Cargo.lock").read_text())
        entries = [package for package in lock["package"] if package["name"] == "oneshot"]
        self.assertEqual(entries, [{"name": "oneshot", "version": "0.2.1"}])

    def test_modified_files_match_reviewed_hashes(self):
        provenance = json.loads((VENDOR / "upstream-sources.json").read_text())
        for package in provenance["packages"]:
            for changed in package["modified_files"]:
                with self.subTest(package=package["name"], path=changed["path"]):
                    path = VENDOR / package["directory"] / changed["path"]
                    self.assertEqual(
                        hashlib.sha256(path.read_bytes()).hexdigest(),
                        changed["patched_sha256"],
                    )


if __name__ == "__main__":
    unittest.main()
