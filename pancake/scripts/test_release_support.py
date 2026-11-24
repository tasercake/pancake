"""Tests for the packages assembled by the manual release workflow."""

import json
import os
import subprocess
import tempfile
import unittest
import zipfile
from pathlib import Path
from unittest.mock import patch

from package_notices import file_sha256, stage_notices, verify_notices
from release_support import package_windows, release_version, smoke_stack
from test_package_linux import _workspace


class ReleaseSupportTests(unittest.TestCase):
    """Verify release versions, archived bytes and smoke-test failure handling."""

    def test_versions_must_match_and_use_three_numeric_components(self):
        with tempfile.TemporaryDirectory() as temporary:
            source, _ = _workspace(Path(temporary))
            cli = source / "crates/cli/Cargo.toml"
            gui = source / "crates/gui/Cargo.toml"
            for current, name in ((cli, "pancake-cli"), (gui, "pancake-gui")):
                target = source / "crates" / name
                current.parent.rename(target)
            self.assertEqual(release_version(source), "0.1.0")
            cli = source / "crates/pancake-cli/Cargo.toml"
            cli.write_text(cli.read_text().replace('version = "0.1.0"', 'version = "0.2.0"'))
            with self.assertRaisesRegex(ValueError, "same three-component version"):
                release_version(source)

    def test_windows_zip_contains_both_programs_and_preserves_staged_notices(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            binaries = root / "binaries"
            binaries.mkdir()
            (binaries / "pancake-cli.exe").write_bytes(b"CLI bytes")
            (binaries / "pancake-gui.exe").write_bytes(b"desktop bytes")
            payload = root / "payload"
            notices = payload / "dependency-notices"
            notices.mkdir(parents=True)
            (notices / "inventory.json").write_text("{}")
            archive = root / "pancake-win.zip"
            package_windows(binaries, payload, archive)
            with zipfile.ZipFile(archive) as package:
                self.assertIsNone(package.testzip())
                self.assertEqual(package.read("pancake-cli.exe"), b"CLI bytes")
                self.assertEqual(package.read("pancake-gui.exe"), b"desktop bytes")
                self.assertEqual(package.read("dependency-notices/inventory.json"), b"{}")

    def test_notice_staging_records_explicit_target_and_features(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source, _ = _workspace(root)
            with (source / "Cargo.toml").open("a") as manifest:
                manifest.write('\n[features]\nfixture = []\n')
            subprocess.run(["git", "-c", "user.name=Fixture", "-c", "user.email=test@localhost",
                            "commit", "-qam", "Declare fixture feature"], cwd=source, check=True)
            revision = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=source, text=True).strip()
            receipt = stage_notices(root / "payload", source, revision, file_sha256(source / "Cargo.lock"),
                                    target="x86_64-pc-windows-msvc", features="pancake/fixture",
                                    directory=Path("dependency-notices"))
            verify_notices(root / "payload", receipt)
            inventory = json.loads((root / "payload" / receipt["file"]).read_text())
            self.assertEqual(inventory["target"], "x86_64-pc-windows-msvc")
            self.assertEqual(inventory["features"], "pancake/fixture")

    def test_macos_release_linker_uses_the_static_zstd_at_the_same_link_position(self):
        linker = Path(__file__).with_name("macos-release-linker.sh")
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            driver = root / "xcrun"
            driver.write_text('#!/bin/sh\nprintf "%s\\n" "$@"\n')
            driver.chmod(0o755)
            archive = root / "static zstd" / "libzstd.a"
            archive.parent.mkdir()
            archive.touch()
            env = {**os.environ, "PATH": os.pathsep.join((temporary, os.environ["PATH"])),
                   "PANCAKE_ZSTD_ARCHIVE": str(archive)}
            args = ["-arch", "arm64", "-lz", "-lzstd", "-lc++"]
            output = subprocess.check_output(["bash", linker, *args], text=True, env=env)
            self.assertEqual(output.splitlines(), ["clang", *args[:3], str(archive), args[-1]])

    @patch("release_support.subprocess.run")
    def test_cli_success_without_tiff_fails_smoke_check(self, run):
        with self.assertRaisesRegex(ValueError, "did not write a TIFF"):
            smoke_stack(Path("pancake-cli"), Path("fixtures"))
        self.assertEqual(run.call_args.args[0][1:3], ["--cpu", "--input"])
