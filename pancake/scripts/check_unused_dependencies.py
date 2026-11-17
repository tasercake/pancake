#!/usr/bin/env python3
"""Run pinned cargo-machete on every first-party workspace manifest, fail closed."""

import json
from pathlib import Path
import subprocess
import sys


def main():
    """Avoid vendor recursion and reject cargo-machete 0.9.2 metadata errors."""
    try:
        metadata = json.loads(Path(sys.argv[1]).read_text())
        packages = {package["id"]: package for package in metadata["packages"]}
        members = metadata["workspace_members"]
        if not members:
            raise ValueError("workspace has no first-party members")
        for member in members:
            manifest = Path(packages[member]["manifest_path"])
            if not manifest.is_file() or manifest.name != "Cargo.toml":
                raise ValueError(f"workspace manifest missing: {manifest}")
            # Passing exact manifest files is supported by the pinned tool's
            # WalkBuilder path input and excludes unrelated nested vendor crates.
            result = subprocess.run(["cargo", "machete", "--with-metadata", str(manifest)],
                                    capture_output=True, text=True, check=False)
            print(result.stdout, end="")
            print(result.stderr, end="", file=sys.stderr)
            if result.returncode or "error when handling " in result.stderr:
                raise ValueError(f"unused-dependency analysis failed: {manifest}")
    except (OSError, ValueError, KeyError, IndexError, TypeError) as error:
        print(f"Unused dependencies: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
