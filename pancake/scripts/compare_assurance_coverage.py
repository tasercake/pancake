#!/usr/bin/env python3
"""Require complete CPU coverage and all host regions exercised by native tests."""

import json
from pathlib import Path
import sys


def covered_regions(report, root):
    """Keep exact function/region identities in authored src/ and build.rs files."""
    result = set()
    for data in report["data"]:
        for function in data["functions"]:
            for region in function["regions"]:
                filename = Path(function["filenames"][region[5]]).resolve()
                if not filename.is_relative_to(root):
                    continue
                relative = filename.relative_to(root)
                crate_path = (Path(*relative.parts[2:])
                              if relative.parts[0] == "crates" and len(relative.parts) > 2
                              else relative)
                if crate_path.is_relative_to("tests"):
                    continue
                if not (crate_path.is_relative_to("src") or crate_path == Path("build.rs")):
                    raise ValueError(f"unclassified source layout requires review: {filename}")
                if region[7] == 0 and region[4] > 0:
                    result.add((function["name"], relative.as_posix(), *region[:4]))
    return result


def main():
    """Reject missing CPU source even if filtered CPU summary appears complete."""
    try:
        root = Path(sys.argv[1]).resolve()
        cpu, native = (json.loads(Path(path).read_text()) for path in sys.argv[2:4])
        if not cpu["data"] or not native["data"]:
            raise ValueError("empty coverage report")
        for label, report in (("CPU", cpu), ("native", native)):
            for data in report["data"]:
                for kind in ("lines", "functions", "regions", "branches"):
                    total = data["totals"][kind]
                    if total["count"] != total["covered"]:
                        raise ValueError(f"uncovered {label} {kind}")
        cpu_regions, native_regions = covered_regions(cpu, root), covered_regions(native, root)
        if not cpu_regions or not native_regions:
            raise ValueError("no production source regions in coverage report")
        missing = native_regions - cpu_regions
        if missing:
            sample = sorted({f"{item[1]}:{item[2]}" for item in missing})[:5]
            raise ValueError(f"native host regions absent from CPU coverage: {', '.join(sample)}")
        print(f"CPU assurance: {len(cpu_regions)} covered host regions; native source universe retained.")
    except (OSError, ValueError, KeyError, IndexError, TypeError) as error:
        print(f"CPU assurance: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
