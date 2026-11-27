"""Verify a release draft's assets before letting it become public."""

import unittest

from release_publish import missing_assets, release_notes


class ReleasePublishTests(unittest.TestCase):
    """Reject a draft that belongs to a different package build."""

    def test_download_table_precedes_changelog_and_links_every_asset(self):
        notes = release_notes("owner/repo", "a" * 40, "0.2.0-rc.1", "## Full changelog\n")
        download = "https://github.com/owner/repo/releases/download/v0.2.0-rc.1/"
        self.assertTrue(notes.startswith("## Downloads\n\n| Platform | Download |\n| --- | --- |"))
        table, changelog = notes.split("## Full changelog", 1)
        for platform in ("macOS (Apple Silicon)", "Windows x64", "Ubuntu 24.04 x86-64"):
            self.assertIn(f"| {platform} |", table)
        for filename in (
            "pancake-0.2.0-rc.1-macos-arm64.dmg", "pancake-gui-0.2.0-rc.1-windows-x64.exe",
            "pancake-cli-0.2.0-rc.1-windows-x64.exe", "pancake-notices-0.2.0-rc.1-windows-x64.tar.gz",
            "pancake_0.2.0-rc.1_amd64.deb", "SHA256SUMS.txt",
        ):
            self.assertIn(f"]({download}{filename})", table)
        self.assertIn("## Installation", changelog)

    def test_draft_uploads_only_missing_assets_and_checks_uploaded_digests(self):
        expected = {"first.deb": "sha256:good", "SHA256SUMS.txt": "sha256:checksums"}
        assets = [{"name": "first.deb", "state": "uploaded", "digest": "sha256:good"}]
        self.assertEqual(missing_assets(expected, assets), ["SHA256SUMS.txt"])
        assets[0]["digest"] = "sha256:bad"
        with self.assertRaisesRegex(ValueError, "digest"):
            missing_assets(expected, assets)

    def test_unexpected_or_duplicate_draft_assets_are_refused(self):
        asset = {"name": "unexpected.deb", "state": "uploaded", "digest": "sha256:good"}
        with self.assertRaisesRegex(ValueError, "Unexpected"):
            missing_assets({}, [asset])
        with self.assertRaisesRegex(ValueError, "Unexpected"):
            missing_assets({asset["name"]: asset["digest"]}, [asset, asset])
