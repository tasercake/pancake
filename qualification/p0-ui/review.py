"""Extend exact root reviews only by this profile's three reviewed parent edges."""

import hashlib
import json
from pathlib import Path
import sys


def merge(base_path, review_path, output_path):
    """Preserve root policy and reject an unreviewed change to its source ledger."""
    raw = base_path.read_bytes()
    base = json.loads(raw)
    review = json.loads(review_path.read_text())
    if hashlib.sha256(raw).hexdigest() != review["base_ledger_sha256"]:
        raise ValueError("Root dependency ledger changed; review the UI extension again")
    for addition in review["parent_additions"]:
        entries = [entry for entry in base["duplicate_exceptions"]
                   if (entry["name"], entry["version"]) == (addition["name"], addition["version"])]
        if len(entries) != 1:
            raise ValueError(f"Missing exact base review: {addition}")
        entries[0]["parents"].append(addition["parent"])
        entries[0]["reason"] += " UI profile: " + addition["reason"]
    output_path.parent.mkdir(parents=True, exist_ok=True)
    output_path.write_text(json.dumps(base, indent=2) + "\n")


if __name__ == "__main__":
    merge(*(Path(argument) for argument in sys.argv[1:]))
