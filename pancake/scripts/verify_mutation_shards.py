"""Verify local round-robin cargo-mutants artifact coverage across shards."""

import argparse
from collections import Counter
import json
from pathlib import Path
import sys

IDENTITY = ("file", "function", "genre", "name", "package", "replacement", "span")
RESULTS = {"CaughtMutant": "caught", "Unviable": "unviable"}


def read_json(path):
    """Load a native cargo-mutants JSON artifact."""
    return json.loads(path.read_text(encoding="utf-8"))


def mutation_id(row):
    """Compare the native mutant identity, excluding list-only diff decoration."""
    return json.dumps({field: row[field] for field in IDENTITY}, sort_keys=True)


def verify_shards(roster, shards):
    """Require each known mutant once and no failed test outcomes or baselines."""
    full = [mutation_id(row) for row in read_json(roster)]
    if not full or len(set(full)) != len(full) or not shards:
        raise ValueError("Expected a nonempty full roster and shards with no duplicate roster identity")
    counts = {"caught": 0, "unviable": 0}
    versions = set()
    for index, shard in enumerate(shards):
        expected = full[index::len(shards)]
        listed = [mutation_id(row) for row in read_json(shard / "list.json")]
        if listed != expected:
            raise ValueError(f"{shard}: partition differs from round-robin shard {index}/{len(shards)}")
        report = read_json(shard / "outcomes.json")
        versions.add(report["cargo_mutants_version"])
        outcomes = report["outcomes"]
        baselines = [row["summary"] for row in outcomes if row["scenario"] == "Baseline"]
        if baselines != ["Success"]:
            raise ValueError(f"{shard}: expected one successful baseline")
        observed = Counter()
        for row in outcomes:
            scenario = row["scenario"]
            if scenario == "Baseline":
                continue
            variant = scenario["Mutant"]
            status = row["summary"]
            if status not in RESULTS:
                raise ValueError(f"{shard}: {status} for {variant['name']}")
            observed[mutation_id(variant)] += 1
            counts[RESULTS[status]] += 1
        if observed != Counter(expected):
            raise ValueError(f"{shard}: missing, duplicate or unexpected mutant in outcome roster")
    if len(versions) != 1:
        raise ValueError("Shard tool versions differ")
    return counts


def main(argv=None):
    """Check a full JSON listing and numbered shard directories in index order."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("roster", type=Path, help="Independent full cargo mutants --list --json output")
    parser.add_argument("shards", type=Path, nargs="+", help="Directories with list.json and outcomes.json, in index order")
    args = parser.parse_args(argv)
    try:
        print(json.dumps(verify_shards(args.roster, args.shards), sort_keys=True))
    except (OSError, ValueError, KeyError, TypeError) as error:
        print(f"Mutation shard evidence: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
