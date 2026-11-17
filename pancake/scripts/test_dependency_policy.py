"""Exercise dependency policy exceptions through their actual command boundary."""

import copy
import datetime
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest


POLICY = '''[graph]
all-features = true
[advisories]
unmaintained = "all"
unsound = "all"
ignore = []
[bans]
multiple-versions = "deny"
wildcards = "deny"
skip = []
skip-tree = []
[sources]
unknown-registry = "deny"
unknown-git = "deny"
allow-registry = ["https://github.com/rust-lang/crates.io-index"]
allow-git = []
'''


class DependencyPolicyTests(unittest.TestCase):
    """Reject policy broadening while permitting reviewed exact dependency versions."""

    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)
        self.today = datetime.datetime.now(datetime.timezone.utc).date()
        self.metadata = {
            "packages": [
                {"id": "old", "name": "utility", "version": "1.0.0", "license": None},
                {"id": "new", "name": "utility", "version": "2.0.0", "license": "MIT"},
                {"id": "parent", "name": "legacy-parent", "version": "3.0.0",
                 "license": "Apache-2.0", "dependencies": [
                     {"name": "utility", "req": "^1", "kind": None, "target": None}]},
            ],
            "resolve": {"nodes": [
                {"id": "parent", "deps": [{"pkg": "old", "dep_kinds": [
                    {"kind": None, "target": None}]}]}]},
        }
        self.entry = {
            "name": "utility", "version": "1.0.0",
            "reason": "Private legacy parent API requires the older semver generation.",
            "parents": [{"name": "legacy-parent", "version": "3.0.0", "requirement": "^1"}],
            "reviewed_on": self.today.isoformat(),
            "expires_on": (self.today + datetime.timedelta(days=30)).isoformat(),
        }
        self.ledger = {"schema_version": 1, "duplicate_exceptions": [self.entry],
                       "advisory_exceptions": []}

    def run_policy(self, *, ledger=None, metadata=None, policy=POLICY):
        (self.root / "deny.toml").write_text(policy)
        (self.root / "ledger.json").write_text(json.dumps(self.ledger if ledger is None else ledger))
        (self.root / "metadata.json").write_text(json.dumps(self.metadata if metadata is None else metadata))
        (self.root / "clippy.toml").write_text('msrv = "1.100.0"\n')
        return subprocess.run([
            sys.executable, str(Path(__file__).with_name("dependency_policy.py")),
            "--config", str(self.root / "deny.toml"),
            "--ledger", str(self.root / "ledger.json"),
            "--metadata", str(self.root / "metadata.json"),
            "--effective-config", str(self.root / "effective.toml"),
            "--license-inventory", str(self.root / "licenses.json"),
            "--clippy-config", str(self.root / "clippy.toml"),
            "--effective-clippy-dir", str(self.root / "clippy"),
        ], text=True, capture_output=True, check=False)

    def test_exact_review_generates_skip_and_keeps_unknown_license_inventory(self):
        result = self.run_policy()
        self.assertEqual(result.returncode, 0, result.stderr)
        config = (self.root / "effective.toml").read_text()
        self.assertIn('version = "=1.0.0"', config)
        inventory = json.loads((self.root / "licenses.json").read_text())
        self.assertFalse(inventory["license_gate_enforced"])
        self.assertIn({"name": "utility", "version": "1.0.0", "license": None}, inventory["packages"])

    def test_reviewed_but_absent_version_does_not_create_unused_skip(self):
        result = self.run_policy(metadata={"packages": [], "resolve": {"nodes": []}})
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertNotIn('version = "=1.0.0"', (self.root / "effective.toml").read_text())

    def test_expired_review_is_rejected_even_when_dependency_absent(self):
        ledger = copy.deepcopy(self.ledger)
        ledger["duplicate_exceptions"][0]["reviewed_on"] = (self.today - datetime.timedelta(days=31)).isoformat()
        ledger["duplicate_exceptions"][0]["expires_on"] = (self.today - datetime.timedelta(days=1)).isoformat()
        result = self.run_policy(ledger=ledger)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("expired", result.stderr)

    def test_version_ranges_are_rejected(self):
        ledger = copy.deepcopy(self.ledger)
        ledger["duplicate_exceptions"][0]["version"] = "^1"
        result = self.run_policy(ledger=ledger)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("exact version", result.stderr)

    def test_shared_boundary_crates_cannot_receive_duplicate_exceptions(self):
        for name in ("wgpu", "wgpu-hal", "naga", "raw-window-handle", "cubecl-core"):
            with self.subTest(name=name):
                ledger = copy.deepcopy(self.ledger)
                ledger["duplicate_exceptions"][0]["name"] = name
                result = self.run_policy(ledger=ledger)
                self.assertNotEqual(result.returncode, 0)
                self.assertIn("protected", result.stderr)

    def test_active_review_requires_resolved_parent_evidence(self):
        metadata = copy.deepcopy(self.metadata)
        metadata["resolve"] = {"nodes": []}
        result = self.run_policy(metadata=metadata)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("parent evidence", result.stderr)

    def test_renamed_dependency_records_only_its_matching_requirement(self):
        metadata = copy.deepcopy(self.metadata)
        metadata["packages"][2]["dependencies"] = [
            {"name": "utility", "rename": "old_utility", "req": "^1"},
            {"name": "utility", "rename": "new_utility", "req": "^2"},
        ]
        metadata["resolve"]["nodes"][0]["deps"][0]["name"] = "old_utility"
        result = self.run_policy(metadata=metadata)
        self.assertEqual(result.returncode, 0, result.stderr)

    def test_library_target_name_can_differ_from_package_name(self):
        metadata = copy.deepcopy(self.metadata)
        metadata["packages"][0]["targets"] = [{"name": "utility_sys", "kind": ["lib"]}]
        metadata["resolve"]["nodes"][0]["deps"][0]["name"] = "utility_sys"
        result = self.run_policy(metadata=metadata)
        self.assertEqual(result.returncode, 0, result.stderr)

    def clippy_ledger(self):
        """Review both exact versions before adding a Clippy name exception."""
        ledger = copy.deepcopy(self.ledger)
        old = ledger["duplicate_exceptions"][0]
        old.update(clippy_allow=True, cargo_deny_skip=True)
        new = copy.deepcopy(old)
        new.update(version="2.0.0", cargo_deny_skip=False)
        ledger["duplicate_exceptions"].append(new)
        metadata = copy.deepcopy(self.metadata)
        metadata["resolve"]["nodes"][0]["deps"].append({"pkg": "new"})
        return ledger, metadata

    def test_clippy_name_exception_requires_both_exact_version_reviews(self):
        ledger, metadata = self.clippy_ledger()
        result = self.run_policy(ledger=ledger, metadata=metadata)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn('allowed-duplicate-crates = ["utility"]',
                      (self.root / "clippy/clippy.toml").read_text())

    def test_clippy_exception_cannot_hide_new_version(self):
        ledger, metadata = self.clippy_ledger()
        metadata["packages"][1]["version"] = "2.1.0"
        result = self.run_policy(ledger=ledger, metadata=metadata)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("unreviewed Clippy duplicate version", result.stderr)

    def test_new_parent_path_requires_new_review(self):
        metadata = copy.deepcopy(self.metadata)
        metadata["packages"][2]["version"] = "4.0.0"
        result = self.run_policy(metadata=metadata)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("parent", result.stderr)

    def test_skip_tree_and_manual_skips_are_rejected(self):
        for old, new in (
            ('skip-tree = []', 'skip-tree = [{ name = "utility" }]'),
            ('skip = []', 'skip = [{ name = "utility", version = "=1.0.0" }]'),
        ):
            with self.subTest(new=new):
                result = self.run_policy(policy=POLICY.replace(old, new))
                self.assertNotEqual(result.returncode, 0)
                self.assertIn("ledger", result.stderr)

    def test_relaxed_advisory_source_or_license_gate_is_rejected(self):
        for policy in (
            POLICY.replace('unmaintained = "all"', 'unmaintained = "none"'),
            POLICY.replace('unknown-git = "deny"', 'unknown-git = "allow"'),
            POLICY + '\n[licenses]\nallow = ["MIT"]\n',
        ):
            with self.subTest(policy=policy):
                result = self.run_policy(policy=policy)
                self.assertNotEqual(result.returncode, 0)

    def test_missing_reason_and_permanent_review_are_rejected(self):
        for update in ({"reason": ""}, {"expires_on": (self.today + datetime.timedelta(days=365)).isoformat()}):
            with self.subTest(update=update):
                ledger = copy.deepcopy(self.ledger)
                ledger["duplicate_exceptions"][0].update(update)
                self.assertNotEqual(self.run_policy(ledger=ledger).returncode, 0)

    def test_advisory_ignore_cannot_cover_an_unreviewed_package_version(self):
        metadata = copy.deepcopy(self.metadata)
        metadata["packages"][0].update(name="ttf-parser", version="0.25.1")
        metadata["packages"][1].update(name="ttf-parser", version="0.26.0")
        metadata["packages"][2]["dependencies"][0].update(name="ttf-parser", req="^0.25")
        entry = copy.deepcopy(self.entry)
        entry.update(name="ttf-parser", version="0.25.1", id="RUSTSEC-2026-0192",
                     exposure_review="Parses system titlebar fonts at runtime; not untrusted photo payload.")
        entry["parents"][0]["requirement"] = "^0.25"
        ledger = {"schema_version": 1, "duplicate_exceptions": [], "advisory_exceptions": [entry]}
        result = self.run_policy(ledger=ledger, metadata=metadata)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("unreviewed version", result.stderr)

    def test_arbitrary_security_advisory_cannot_be_ignored(self):
        ledger = copy.deepcopy(self.ledger)
        entry = copy.deepcopy(self.entry)
        entry.update(id="RUSTSEC-2099-9999", exposure_review="Unreviewed issue.")
        ledger["advisory_exceptions"].append(entry)
        result = self.run_policy(ledger=ledger)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("reviewed unmaintained", result.stderr)

    def test_chrono_maintenance_review_is_exact_and_does_not_cover_other_versions(self):
        metadata = copy.deepcopy(self.metadata)
        metadata["packages"][0].update(name="chrono", version="0.4.45")
        metadata["packages"][2].update(name="i-slint-core", version="1.19.0")
        metadata["packages"][2]["dependencies"][0].update(name="chrono", req="^0.4")
        entry = copy.deepcopy(self.entry)
        entry.update(name="chrono", version="0.4.45", id="RUSTSEC-2026-0543",
                     exposure_review="Runtime Slint date text/format parsing and local clock/timezone access.")
        entry["parents"] = [{"name": "i-slint-core", "version": "1.19.0", "requirement": "^0.4"}]
        ledger = {"schema_version": 1, "duplicate_exceptions": [], "advisory_exceptions": [entry]}
        result = self.run_policy(ledger=ledger, metadata=metadata)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn('id = "RUSTSEC-2026-0543"', (self.root / "effective.toml").read_text())

        metadata["packages"][1].update(name="chrono", version="0.4.46")
        result = self.run_policy(ledger=ledger, metadata=metadata)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("unreviewed version", result.stderr)


if __name__ == "__main__":
    unittest.main()
