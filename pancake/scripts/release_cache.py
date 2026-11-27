"""Address release build outputs by the dependency graph and source contents."""

import argparse
import hashlib
import os
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
VENDOR = "pancake/vendor"
GRAPH = (
    VENDOR, ".gitattributes", "pancake/.gitattributes", "pancake/Cargo.lock",
    "pancake/Cargo.toml", "pancake/.cargo", "pancake/rust-toolchain.toml",
    "pancake/crates/pancake-cli/Cargo.toml", "pancake/crates/pancake-gui/Cargo.toml",
    "pancake/notices/llvm/manifest.json", "pancake/scripts/ci-linux-native-deps.sh",
    "pancake/scripts/macos-release-linker.sh", "pancake/scripts/package_macos.py",
)
WORKSPACE = (
    "pancake/Cargo.lock", "pancake/Cargo.toml", "pancake/src", "pancake/build.rs", "pancake/crates",
    "assets", "pancake/lightroom",
)
CHECKS = (
    "pancake/tests", "pancake/.config", "pancake/clippy.toml", "pancake/deny.toml",
    "pancake/dependency-policy.json", "pancake/scripts/check.sh", "pancake/scripts/dependency_policy.py",
)
SOURCE_MTIME = 946684800  # 2000-01-01 UTC


def tracked_tree(root, revision, paths):
    """Read names, modes and object identities for the selected Git tree entries."""
    return subprocess.check_output(["git", "ls-tree", "-rz", revision, "--", *paths], cwd=root)


def dependency_keys(root, dispatched, *, checks=False):
    """Use the dispatched graph for shared dependencies and the candidate for workspace outputs."""
    graph = tracked_tree(root, dispatched, GRAPH)
    candidate = tracked_tree(root, "HEAD", (*WORKSPACE, *CHECKS) if checks else WORKSPACE)
    return hashlib.sha256(graph).hexdigest(), hashlib.sha256(candidate).hexdigest()


def restore_mtimes(root, *, exact, checks=False, llvm_data=None):
    """Let Cargo reuse path inputs whose tracked contents match the restored cache."""
    paths = (VENDOR,)
    if exact:
        paths += (*WORKSPACE, *CHECKS) if checks else WORKSPACE
    for record in filter(None, tracked_tree(root, "HEAD", paths).split(b"\0")):
        metadata, name = record.split(b"\t", maxsplit=1)
        if metadata.startswith((b"100644 ", b"100755 ")):
            path = root / name.decode("utf-8")
            if not path.is_symlink():
                os.utime(path, (SOURCE_MTIME, SOURCE_MTIME))
    if llvm_data is not None:
        # The bundle build script creates and tracks this marker on a cold run.
        for marker in llvm_data.glob("tracel-llvm-*/.tracel-llvm-installed"):
            if marker.is_file() and not marker.is_symlink():
                os.utime(marker, (SOURCE_MTIME, SOURCE_MTIME))


def main():
    """Print Actions outputs or restore mtimes for a verified key hit."""
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    keys = commands.add_parser("keys")
    keys.add_argument("dispatched")
    keys.add_argument("--checks", action="store_true")
    restore = commands.add_parser("restore-mtimes")
    restore.add_argument("--exact", action="store_true")
    restore.add_argument("--checks", action="store_true")
    restore.add_argument("--llvm-data", type=Path)
    args = parser.parse_args()
    if args.command == "keys":
        graph, inputs = dependency_keys(ROOT, args.dispatched, checks=args.checks)
        print(f"graph={graph}")
        print(f"inputs={inputs}")
    else:
        restore_mtimes(ROOT, exact=args.exact, checks=args.checks, llvm_data=args.llvm_data)


if __name__ == "__main__":
    main()
