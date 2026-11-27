"""Release history, version allocation, and Copilot input/output contracts."""

import json
import subprocess
import tempfile
import unittest
from pathlib import Path

from release_plan import (
    Change, PublishedRelease, allocate_version, baseline, inventory, notes,
    read_advice, release_prompt, update_manifests,
)


class ReleasePlanTests(unittest.TestCase):
    """Check release behavior without publishing or invoking Copilot."""

    def test_stable_changelog_includes_prereleases_and_ignores_draft(self):
        stable = PublishedRelease("v0.1.0", "base", False)
        preview = PublishedRelease("v0.2.0-rc.1", "preview", True)
        self.assertEqual(baseline([preview, stable], "prerelease"), preview)
        self.assertEqual(baseline([preview, stable], "stable"), stable)
        self.assertIsNone(baseline([preview], "stable"))

    def test_legacy_release_and_prerelease_numbers_are_never_reused(self):
        legacy = PublishedRelease("v0.1.0", "base", True)
        self.assertEqual(allocate_version([legacy], {"v0.1.0"}, "minor", "prerelease"), "0.2.0-rc.1")
        first = PublishedRelease("v0.2.0-rc.1", "preview", True)
        self.assertEqual(
            allocate_version([first, legacy], {"v0.2.0-rc.1", "v0.2.0-rc.2"}, "patch", "prerelease"),
            "0.2.0-rc.3",
        )
        self.assertEqual(allocate_version([first, legacy], {"v0.1.0"}, "patch", "stable"), "0.2.0")
        self.assertEqual(allocate_version([legacy], {"v0.1.1"}, "patch", "stable"), "0.1.2")

    def test_breaking_changes_preserve_initial_development_and_bump_stable_major(self):
        initial = PublishedRelease("v0.3.0", "base", False)
        stable = PublishedRelease("v1.3.0", "base", False)
        self.assertEqual(allocate_version([initial], set(), "major", "stable"), "0.4.0")
        self.assertEqual(allocate_version([stable], set(), "major", "stable"), "2.0.0")

    def test_copilot_advice_cannot_invent_references_or_include_markup(self):
        first = Change("a" * 40, "Fix reading (#14)", "", True)
        self.assertIsNone(read_advice('{"bump":"0.9.0","highlights":[]}', [first]))
        for revision, summary in (
            ("z" * 12, "Read images"), ("a" * 12, "[link](https://bad.example)"), ("a" * 12, "Read\nimages"),
        ):
            result = json.dumps({"bump": "minor", "highlights": [{"change": revision, "summary": summary}]})
            self.assertIsNone(read_advice(result, [first]))
        advice = read_advice('{"bump":"patch","highlights":[{"change":"aaaaaaaaaaaa","summary":"Read images"}]}',
                             [first])
        self.assertEqual(advice, ("patch", [(first, "Read images")]))

    def test_notes_include_every_change_even_if_copilot_skips_it(self):
        product = Change("a" * 40, "Fix reading (#14)", "", True)
        companion = Change("b" * 40, "BokehForge [maintenance] @someone", "", False)
        result = notes("owner/repo", "a" * 40, None, [product, companion], [(product, "Read images")])
        self.assertIn("Read images", result)
        self.assertIn("https://github.com/owner/repo/pull/14", result)
        self.assertIn("https://github.com/owner/repo/commit/" + "b" * 40, result)
        self.assertIn("Other repository changes", result)
        self.assertIn("&#64;someone", result)
        self.assertIn("## Full changelog", result)

    def test_prompt_is_bounded_but_uses_product_history_for_the_bump(self):
        product = Change("a" * 40, "Fix reading", "details", True)
        old = Change("b" * 40, "Add stacking", "", True)
        companion = Change("c" * 40, "BokehForge", "huge " * 9000, False)
        result = release_prompt([product, companion] * 100 + [old], [product])
        self.assertLess(len(result), 16_000)
        self.assertIn('"version_scope": true', result)
        self.assertIn("details", result)

    def test_inventory_uses_first_parent_and_resolves_release_commit_to_its_source(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)

            def git(*args):
                return subprocess.check_output(
                    ["git", "-c", "user.name=Fixture", "-c", "user.email=fixture@example.net", *args],
                    cwd=root, text=True,
                ).strip()

            def commit(path, subject):
                file = root / path
                file.parent.mkdir(parents=True, exist_ok=True)
                file.write_text(subject)
                git("add", path)
                git("commit", "-qm", subject)
                return git("rev-parse", "HEAD")

            git("init", "-q", "-b", "main")
            base = commit("pancake/foo", "Initial product")
            git("tag", "v0.1.0")
            source = commit("pancake/foo", "Improve stacking (#12)")
            git("switch", "-q", "-c", "candidate")
            commit("pancake/metadata", "Release Pancake v0.2.0-rc.1")
            git("tag", "v0.2.0-rc.1")
            git("switch", "-q", "main")
            final = commit("bokehforge/tool", "Improve review")
            git("tag", "v0.2.0-rc.2")
            releases = [
                {"tag_name": "v0.1.0", "draft": False, "prerelease": True},
                {"tag_name": "v0.2.0-rc.1", "draft": False, "prerelease": True},
                {"tag_name": "v0.2.0-rc.2", "draft": True, "prerelease": True},
            ]
            changes, published, occupied = inventory(root, final, releases)
            self.assertEqual([change.revision for change in changes], [final, source, base])
            self.assertEqual([change.product for change in changes], [False, True, True])
            self.assertEqual(published, [PublishedRelease("v0.2.0-rc.1", source, True),
                                         PublishedRelease("v0.1.0", base, True)])
            self.assertIn("v0.2.0-rc.2", occupied)
            with self.assertRaisesRegex(ValueError, "already ships newer source"):
                inventory(root, base, releases)
            inventory(root, base, releases, build_only=True)

    def test_version_update_changes_only_workspace_versions_and_exact_requirements(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            for name in ("pancake", "pancake-cli", "pancake-gui"):
                parent = root if name == "pancake" else root / "crates" / name
                parent.mkdir(parents=True, exist_ok=True)
                content = f'[package]\nname = "{name}"\nversion = "0.1.0"\n'
                if name != "pancake":
                    content += '\n[dependencies]\npancake = { version = "=0.1.0", path = "../.." }\n'
                content += '\n[package.metadata.test]\nversion = "0.1.0"\n'
                (parent / "Cargo.toml").write_text(content)
            update_manifests(root, "0.2.0-rc.1")
            for path in root.rglob("Cargo.toml"):
                self.assertIn('version = "0.2.0-rc.1"', path.read_text())
                self.assertIn('[package.metadata.test]\nversion = "0.1.0"', path.read_text())
                if path.parent != root:
                    self.assertIn('version = "=0.2.0-rc.1"', path.read_text())


if __name__ == "__main__":
    unittest.main()
