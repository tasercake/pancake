"""Exercise the file identities used by the release build cache."""

import os
import subprocess
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from release_cache import dependency_keys, restore_mtimes


class ReleaseCacheTests(unittest.TestCase):
    """Keep version-only candidates and source edits on the correct cache paths."""

    def setUp(self):
        self.root = Path(self.enterContext(tempfile.TemporaryDirectory()))
        self.git("init", "--quiet")
        self.write("pancake/Cargo.lock", 'version = "0.1.0"\n')
        self.write("pancake/crates/pancake-gui/Cargo.toml", 'version = "0.1.0"\n')
        self.write("pancake/src/lib.rs", "pub fn work() {}\n")
        self.write("pancake/vendor/patch/src/lib.rs", "pub fn patched() {}\n")
        self.write("pancake/docs/example.md", "Documentation\n")
        self.source = self.commit()

    def git(self, *args):
        return subprocess.check_output(
            ["git", "-c", "user.name=Fixture", "-c", "user.email=test@localhost",
             "-c", "commit.gpgSign=false", *args],
            cwd=self.root, text=True,
        ).strip()

    def write(self, name, content):
        path = self.root / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(content)

    def commit(self):
        self.git("add", "-A")
        self.git("commit", "--quiet", "-m", "Fixture")
        return self.git("rev-parse", "HEAD")

    def test_candidate_versions_reuse_dependencies_and_keep_separate_binaries(self):
        original_graph, original_inputs = dependency_keys(self.root, self.source)
        self.write("pancake/Cargo.lock", 'version = "0.2.0-rc.2"\n')
        self.write("pancake/crates/pancake-gui/Cargo.toml", 'version = "0.2.0-rc.2"\n')
        self.commit()
        graph, inputs = dependency_keys(self.root, self.source)
        self.assertEqual(graph, original_graph)
        self.assertNotEqual(inputs, original_inputs)
        self.assertNotEqual(dependency_keys(self.root, "HEAD")[0], original_graph)

    def test_documentation_reuses_targets_and_renamed_build_inputs_do_not(self):
        original = dependency_keys(self.root, self.source)
        self.write("pancake/docs/example.md", "New documentation\n")
        self.commit()
        self.assertEqual(dependency_keys(self.root, self.source), original)
        self.git("mv", "pancake/src/lib.rs", "pancake/src/moved.rs")
        self.commit()
        graph, inputs = dependency_keys(self.root, self.source)
        self.assertEqual(graph, original[0])
        self.assertNotEqual(inputs, original[1])

    def test_tests_change_check_cache_and_leave_release_cache_intact(self):
        packages = dependency_keys(self.root, self.source)
        checks = dependency_keys(self.root, self.source, checks=True)
        self.write("pancake/tests/smoke.rs", "#[test] fn smoke() {}\n")
        self.commit()
        self.assertEqual(dependency_keys(self.root, self.source), packages)
        self.assertNotEqual(dependency_keys(self.root, self.source, checks=True), checks)

    def test_embedded_external_assets_invalidate_release_binaries(self):
        original = dependency_keys(self.root, self.source)
        for name in ("assets/pancake-stacker-wordmark.png", "pancake/lightroom/Pancake.lrplugin/Info.lua"):
            with self.subTest(name=name):
                self.write(name, "New embedded content\n")
                self.commit()
                graph, inputs = dependency_keys(self.root, self.source)
                self.assertEqual(graph, original[0])
                self.assertNotEqual(inputs, original[1])
                original = graph, inputs

    def test_mtime_reset_only_covers_files_validated_by_the_matching_key(self):
        vendor = self.root / "pancake/vendor/patch/src/lib.rs"
        workspace = self.root / "pancake/src/lib.rs"
        docs = self.root / "pancake/docs/example.md"
        previous = [path.stat().st_mtime for path in (vendor, workspace, docs)]
        restore_mtimes(self.root, exact=False)
        self.assertLess(vendor.stat().st_mtime, previous[0])
        self.assertEqual(workspace.stat().st_mtime, previous[1])
        restore_mtimes(self.root, exact=True)
        self.assertLess(workspace.stat().st_mtime, previous[1])
        self.assertEqual(docs.stat().st_mtime, previous[2])

    def test_cache_restore_works_without_a_no_follow_utime_option(self):
        vendor = self.root / "pancake/vendor/patch/src/lib.rs"
        previous = vendor.stat().st_mtime
        native_utime = os.utime

        def portable_utime(path, times):
            native_utime(path, times)

        with patch("release_cache.os.utime", portable_utime):
            restore_mtimes(self.root, exact=True)
        self.assertLess(vendor.stat().st_mtime, previous)
