#!/usr/bin/env python3
"""Route integration targets after validating the entire workspace inventory."""

import json
from pathlib import Path
import re
import sys


def main():
    """Validate all first-party integration targets before emitting one profile."""
    try:
        source, family = sys.argv[1:3]
        metadata = json.load(sys.stdin) if source == "-" else json.loads(Path(source).read_text())
        if family not in {"cpu", "gpu", "headless", "gui"}:
            raise ValueError("profile must be cpu, gpu, headless or gui")
        members = set(metadata["workspace_members"])
        targets = [(package["name"], target["name"])
                   for package in metadata["packages"] if package["id"] in members
                   for target in package["targets"] if target["kind"] == ["test"]]
        unknown = [name for _, name in targets
                   if not re.fullmatch(r"(?:cpu|gpu)_[A-Za-z0-9_-]+", name)]
        if unknown:
            raise ValueError(f"unclassified integration targets: {', '.join(sorted(unknown))}")
        if family == "headless":
            targets = [(package, name) for package, name in targets if package != "pancake-gui"]
        elif family == "gui":
            targets = [(package, name) for package, name in targets if package == "pancake-gui"]
        prefix = "gpu_" if family == "gpu" else "cpu_"
        selected = sorted({name for _, name in targets if name.startswith(prefix)})
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
