"""Verify a release draft's assets before letting it become public."""

import contextlib
import hashlib
import io
import json
import os
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from release_publish import missing_assets, package_names, publish, release_notes, upload_asset


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

    def test_upload_posts_raw_binary_to_github_asset_url(self):
        with tempfile.TemporaryDirectory() as directory:
            asset = Path(directory) / "package with space.zip"
            asset.write_bytes(b"\x00\xff")
            response = {"name": asset.name, "state": "uploaded", "digest": "sha256:fixture"}

            def send(request, *, timeout):
                self.assertEqual(request.full_url, "https://uploads.github.com/draft/assets?name=package+with+space.zip")
                self.assertEqual(request.get_method(), "POST")
                self.assertEqual(request.get_header("Content-length"), "2")
                self.assertEqual(request.get_header("Content-type"), "application/octet-stream")
                self.assertEqual(request.get_header("Authorization"), "Bearer fixture")
                self.assertEqual(request.data.read(), b"\x00\xff")
                self.assertGreater(timeout, 0)
                return io.BytesIO(json.dumps(response).encode())

            with (patch.dict(os.environ, {"GH_TOKEN": "fixture"}),
                  patch("release_publish.urlopen", side_effect=send) as send_request):
                self.assertEqual(
                    upload_asset("https://uploads.github.com/draft/assets{?name,label}", asset), response,
                )
                with self.assertRaisesRegex(ValueError, "unexpected release asset upload URL"):
                    upload_asset("https://example.test/draft/assets{?name,label}", asset)
                send_request.assert_called_once()

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


class PublishFlowTests(unittest.TestCase):
    """Exercise the release transaction against GitHub response fixtures."""

    def setUp(self):
        self.root = Path(self.enterContext(tempfile.TemporaryDirectory()))
        self.repository = "owner/repo"
        self.version = "0.2.0-rc.1"
        self.tag = f"v{self.version}"
        self.source = "a" * 40
        self.endpoint = f"repos/{self.repository}/releases"
        self.upload_url = f"https://uploads.github.com/{self.endpoint}/42/assets{{?name,label}}"
        self.draft = {
            "id": 42, "tag_name": self.tag, "target_commitish": self.source,
            "prerelease": True, "draft": True, "assets": [],
            "upload_url": self.upload_url, "html_url": f"https://github.com/{self.repository}/releases/tag/draft",
        }
        self.public_url = f"https://github.com/{self.repository}/releases/tag/{self.tag}"
        self.releases = []
        self.bad_digest = False
        self.calls = []
        self.dist = self.root / "dist"
        self.dist.mkdir()
        (self.root / "changelog.md").write_text("## Full changelog\n")
        self.packages = package_names(self.version)
        for name in self.packages:
            (self.dist / name).write_text(name)
        self.enterContext(patch.dict(os.environ, {"GH_REPO": self.repository}))
        self.enterContext(patch("release_publish.release_version", return_value=self.version))
        self.git = self.enterContext(patch("release_publish.git", side_effect=self.git_response))
        self.enterContext(patch("release_publish.github", side_effect=self.github_response))

    def git_response(self, *args):
        if args == ("rev-parse", "HEAD"):
            return self.source
        if args[:3] == ("ls-remote", "--tags", "origin"):
            return f"{self.source}\trefs/tags/{self.tag}" if self.releases else ""
        if args[:2] == ("push", "origin"):
            return ""
        self.fail(f"Unexpected git command: {args}")

    def github_response(self, *args, payload=None):
        self.calls.append((args, payload))
        if args == ("api", "--paginate", "--slurp", f"{self.endpoint}?per_page=100"):
            # The list remains stale even after a successful create.
            return json.dumps([self.releases])
        if args == ("api", "--method", "POST", self.endpoint):
            self.assertEqual(payload["tag_name"], self.tag)
            self.assertEqual(payload["target_commitish"], self.source)
            self.assertTrue(payload["draft"])
            self.assertTrue(payload["prerelease"])
            return json.dumps(self.draft)
        if args == ("api", f"{self.endpoint}/42/assets?per_page=100"):
            self.assertTrue(self.releases)
            return json.dumps(self.draft["assets"])
        if args == ("api", "--method", "PATCH", f"{self.endpoint}/42"):
            self.assertFalse(payload["draft"])
            self.assertEqual(payload["make_latest"], "false")
            self.assertTrue(payload["body"].startswith("## Downloads"))
            return json.dumps({**self.draft, "draft": False, "html_url": self.public_url})
        self.fail(f"Unexpected GitHub command: {args}")

    def verify_upload(self, url, path):
        self.assertEqual(url, self.upload_url)
        digest = hashlib.sha256(path.read_bytes()).hexdigest() if not self.bad_digest else "incorrect"
        self.calls.append((("upload", path.name), None))
        return {"name": path.name, "state": "uploaded", "digest": f"sha256:{digest}"}

    def test_new_draft_is_published_from_create_response_when_list_stays_stale(self):
        with (patch("release_publish.upload_asset", self.verify_upload),
              contextlib.redirect_stdout(io.StringIO()) as output):
            publish(self.dist, self.root, prerelease=True)
        self.assertEqual(output.getvalue().strip(), self.public_url)
        self.assertEqual([args[:3] for args, _ in self.calls if args[0] == "api"],
                         [("api", "--paginate", "--slurp"), ("api", "--method", "POST"),
                          ("api", "--method", "PATCH")])
        self.assertEqual([args[1] for args, _ in self.calls if args[0] == "upload"],
                         [*self.packages, "SHA256SUMS.txt"])
        self.git.assert_any_call("push", "origin", f"{self.source}:refs/tags/{self.tag}")

    def test_resume_checks_existing_assets_and_uploads_only_missing_files(self):
        first = self.packages[0]
        digest = hashlib.sha256((self.dist / first).read_bytes()).hexdigest()
        self.draft["assets"] = [{"name": first, "state": "uploaded", "digest": f"sha256:{digest}"}]
        self.releases.append(self.draft)
        with (patch("release_publish.upload_asset", self.verify_upload),
              contextlib.redirect_stdout(io.StringIO())):
            publish(self.dist, self.root, prerelease=True)
        self.assertEqual([args[1] for args, _ in self.calls if args[0] == "upload"],
                         [*self.packages[1:], "SHA256SUMS.txt"])
        self.assertFalse(any(args[:3] == ("api", "--method", "POST") for args, _ in self.calls))

    def test_bad_upload_digest_leaves_release_unpublished(self):
        self.bad_digest = True
        with patch("release_publish.upload_asset", self.verify_upload):
            with self.assertRaisesRegex(ValueError, "asset digest"):
                publish(self.dist, self.root, prerelease=True)
        self.assertFalse(any(args[:3] == ("api", "--method", "PATCH") for args, _ in self.calls))
