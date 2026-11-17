#!/usr/bin/env python3
"""Select explicit CPU or physical-GPU integration targets without silent omissions."""

import json
from pathlib import Path
import re
import sys


def main():
    """Validate all first-party integration targets before emitting one profile."""
    try:
        metadata = json.loads(Path(sys.argv[1]).read_text())
        family = sys.argv[2]
        if family not in {"cpu", "gpu"}:
            raise ValueError("profile must be cpu or gpu")
        members = set(metadata["workspace_members"])
        targets = [target["name"] for package in metadata["packages"] if package["id"] in members
                   for target in package["targets"] if target["kind"] == ["test"]]
        unknown = [name for name in targets if not re.fullmatch(r"(?:cpu|gpu)_[A-Za-z0-9_-]+", name)]
        if unknown:
            raise ValueError(f"unclassified integration targets: {', '.join(sorted(unknown))}")
        selected = sorted({name for name in targets if name.startswith(family + "_")})
        if family == "gpu" and not selected:
            raise ValueError("no physical-GPU integration target exists")
        for name in selected:
            print(name)
    except (OSError, ValueError, KeyError, IndexError, TypeError) as error:
        print(f"Assurance targets: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
