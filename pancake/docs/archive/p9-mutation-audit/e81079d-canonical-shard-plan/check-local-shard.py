"""Select and reconcile one canonical native Linux CPU mutation shard."""

import argparse
from collections import Counter
import hashlib
import json
from pathlib import Path
import sys
import tempfile

SHARDS = 16
COST_MS = {"pancake": 22_875, "pancake-gui": 18_019, "pancake-cli": 10_658}
GUARD_SHA = "368f6c57a083ed3b6c18dd9d35a696677278f1136555712f9cb413b5a56c93cc"
TARGET_SHA = "d1180f38b292c12c077049872a7c8ce68a7517a643fcaecd71b07acdc7878f9e"
VERSION = "27.1.0"


def sha(data):
    """Return the SHA-256 of the supplied bytes."""
    return hashlib.sha256(data).hexdigest()


def load_guard(path):
    """Use only the verifier bytes reviewed at edc1d26."""
    raw = path.read_bytes()
    if sha(raw) != GUARD_SHA:
        raise ValueError("reviewed verifier digest differs")
    guard = {"__name__": "reviewed_guard"}
    exec(compile(raw, str(path), "exec"), guard)
    return guard


def plan(roster, identity):
    """Select the smallest full partition, with historical cost and index tie breaks."""
    data = roster.read_bytes()
    full = json.loads(data)
    identities = [identity(row) for row in full]
    if len(full) < SHARDS or len(set(identities)) != len(full):
        raise ValueError("expected a full roster with 16 nonempty partitions and unique identities")
    partitions = [full[index::SHARDS] for index in range(SHARDS)]
    scores = [(len(rows), sum(COST_MS[row["package"]] for row in rows), index)
              for index, rows in enumerate(partitions)]
    count, cost_ms, index = min(scores)
    selected = partitions[index]
    report = {
        "full_count": len(full),
        "full_sha256": sha(data),
        "partition": f"{index}/{SHARDS}",
        "partition_count": count,
        "partition_counts": [len(rows) for rows in partitions],
        "ordered_identity_sha256": sha(
            "\n".join(identity(row) for row in selected).encode("utf-8")
        ),
        "by_package": dict(Counter(row["package"] for row in selected)),
        "historical_mean_variant_projection_ms": cost_ms,
    }
    return report, selected


def check_list(attempt, report, expected, guard):
    """Bind the plan receipt and independent native listing to the full roster."""
    if guard["read_json"](attempt / "selection.json") != report:
        raise ValueError("selection receipt differs from the independently retained full roster")
    listed = guard["read_json"](attempt / "list.json")
    identity = guard["mutation_id"]
    if [identity(row) for row in listed] != [identity(row) for row in expected]:
        raise ValueError("independent native listing differs from the complete canonical partition")
    return listed


def verify(attempt, listed, guard):
    """Require terminal status, native execution of the entire list, and reviewed phase checks."""
    for marker in ("native-exit.status", "log-writer.status"):
        if (attempt / marker).read_bytes() != b"0\n":
            raise ValueError(f"{marker}: producer or log writer failed")
    read = guard["read_json"]
    if read(attempt / "mutants.out/mutants.json") != listed:
        raise ValueError("native execution roster differs from the independent canonical listing")
    outcomes = read(attempt / "mutants.out/outcomes.json")
    if outcomes != read(attempt / "outcomes.json"):
        raise ValueError("copied outcomes differ from the authoritative native outcomes")
    targets = (attempt / "cpu-targets.txt").read_bytes()
    if sha(targets) != TARGET_SHA:
        raise ValueError("CPU selector receipt differs from the exact suite")
    with tempfile.TemporaryDirectory(prefix="pancake-local-shard-verifier-") as temporary:
        selected = Path(temporary) / "canonical-partition.json"
        selected.write_text(json.dumps(listed), encoding="utf-8")
        counts = guard["verify_shards"](selected, [attempt])
    guard["verify_native_phases"](attempt, outcomes, targets.decode("utf-8"), VERSION)
    return counts


def main():
    """Reconcile evidence without running Cargo, native tests or a mutator."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("mode", choices=("plan", "check-list", "verify"))
    parser.add_argument("reviewed_guard", type=Path)
    parser.add_argument("full_roster", type=Path)
    parser.add_argument("attempt", nargs="?", type=Path)
    args = parser.parse_args()
    try:
        guard = load_guard(args.reviewed_guard)
        report, expected = plan(args.full_roster, guard["mutation_id"])
        if args.mode != "plan":
            if args.attempt is None:
                raise ValueError("an attempt directory is required")
            listed = check_list(args.attempt, report, expected, guard)
            if args.mode == "verify":
                report["native_outcomes"] = verify(args.attempt, listed, guard)
        print(json.dumps(report, sort_keys=True, indent=2))
    except (OSError, ValueError, KeyError, TypeError) as error:
        print(f"Local mutation shard evidence: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
