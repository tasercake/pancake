"""Verify a release draft's assets before letting it become public."""

import unittest

from release_publish import missing_assets


class ReleasePublishTests(unittest.TestCase):
    """Reject a draft that belongs to a different package build."""

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
