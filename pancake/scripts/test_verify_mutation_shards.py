"""Exercise cross-shard accounting with archived cargo-mutants outcomes."""

import gzip
import json
from pathlib import Path
import tempfile
import unittest

from verify_mutation_shards import verify_shards

DOCS = Path(__file__).resolve().parents[1] / "docs"
P0 = DOCS / "p0-close-evidence/closure-mutation-outcomes.json.gz"
P1 = DOCS / "p1-assurance-evidence/cli-config/mutation-outcomes.json.gz"


class MutationShardTests(unittest.TestCase):
    """A complete local aggregate carries every expected final outcome."""

    def setUp(self):
        self.scratch = tempfile.TemporaryDirectory()
        self.addCleanup(self.scratch.cleanup)
        self.root = Path(self.scratch.name)

    def capture(self, archive=P0):
        """Split archived scenario payloads into two fixture-only partitions."""
        data = json.loads(gzip.decompress(archive.read_bytes()))
        baseline = next(row for row in data["outcomes"] if row["scenario"] == "Baseline")
        outcomes = [row for row in data["outcomes"] if row["scenario"] != "Baseline"]
        listing = [{"diff": "fixture metadata", **row["scenario"]["Mutant"]} for row in outcomes]
        paths = [self.root / str(index) for index in range(2)]
        self.write(self.root / "full.json", listing)
        for index, path in enumerate(paths):
            path.mkdir()
            self.write(path / "list.json", listing[index::2])
            self.write(path / "outcomes.json", {**data, "outcomes": [baseline, *outcomes[index::2]]})
        return self.root / "full.json", paths

    def write(self, path, data):
        """Rewrite one test-only synthetic input."""
        path.write_text(json.dumps(data), encoding="utf-8")

    def test_disjoint_complete_archived_schema(self):
        roster, paths = self.capture()
        self.assertEqual(verify_shards(roster, paths), {"caught": 79, "unviable": 0})

    def test_compiler_unviable_is_preserved_as_a_distinct_class(self):
        roster, paths = self.capture(P1)
        self.assertEqual(verify_shards(roster, paths), {"caught": 85, "unviable": 3})

    def test_missing_shard_cannot_supply_full_roster(self):
        roster, paths = self.capture()
        with self.assertRaisesRegex(ValueError, "partition"):
            verify_shards(roster, paths[:1])

    def test_wrong_expected_partition_is_rejected(self):
        roster, paths = self.capture()
        with self.assertRaisesRegex(ValueError, "partition"):
            verify_shards(roster, list(reversed(paths)))

    def test_missing_and_duplicate_outcomes_are_rejected(self):
        roster, paths = self.capture()
        path = paths[0] / "outcomes.json"
        original = json.loads(path.read_text())
        for rows in (original["outcomes"][:-1], [*original["outcomes"], original["outcomes"][-1]]):
            with self.subTest(count=len(rows)):
                self.write(path, {**original, "outcomes": rows})
                with self.assertRaisesRegex(ValueError, "outcome roster"):
                    verify_shards(roster, paths)

    def test_survivor_and_timeout_are_rejected(self):
        roster, paths = self.capture()
        path = paths[0] / "outcomes.json"
        data = json.loads(path.read_text())
        for status in ("MissedMutant", "Timeout"):
            with self.subTest(status=status):
                data["outcomes"][1]["summary"] = status
                self.write(path, data)
                with self.assertRaisesRegex(ValueError, status):
                    verify_shards(roster, paths)

    def test_missing_or_failed_baseline_is_rejected(self):
        roster, paths = self.capture()
        path = paths[1] / "outcomes.json"
        data = json.loads(path.read_text())
        self.write(path, {**data, "outcomes": data["outcomes"][1:]})
        with self.assertRaisesRegex(ValueError, "baseline"):
            verify_shards(roster, paths)
        data["outcomes"][0]["summary"] = "Failure"
        self.write(path, data)
        with self.assertRaisesRegex(ValueError, "baseline"):
            verify_shards(roster, paths)

    def test_duplicate_full_identity_is_rejected(self):
        roster, paths = self.capture()
        listing = json.loads(roster.read_text())
        self.write(roster, [*listing, {**listing[0], "diff": "another output decoration"}])
        with self.assertRaisesRegex(ValueError, "duplicate"):
            verify_shards(roster, paths)

    def test_inconsistent_pinned_tool_versions_are_rejected(self):
        roster, paths = self.capture()
        path = paths[1] / "outcomes.json"
        data = json.loads(path.read_text())
        self.write(path, {**data, "cargo_mutants_version": "other"})
        with self.assertRaisesRegex(ValueError, "tool version"):
            verify_shards(roster, paths)
