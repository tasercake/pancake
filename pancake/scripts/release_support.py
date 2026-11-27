"""Prepare and smoke-test the packages built by the manual release workflow."""

import argparse
import re
import shutil
import subprocess
import tarfile
import tempfile
import tomllib
from pathlib import Path

from package_notices import ROOTS, file_sha256, stage_notices, verify_notices

SOURCE = Path(__file__).resolve().parents[1]
VERSION_PATTERN = r"(?:0|[1-9][0-9]*)(?:\.(?:0|[1-9][0-9]*)){2}(?:-rc\.[1-9][0-9]*)?"


def debian_version(version):
    """Keep Debian release candidates ordered before the corresponding stable package."""
    return version.replace("-rc.", "~rc.")


def release_version(source):
    """Require the core and both executables to agree on the release version."""
    manifests = [source / "Cargo.toml", *(source / "crates" / name / "Cargo.toml" for name in ROOTS)]
    versions = set()
    for manifest in manifests:
        with manifest.open("rb") as stream:
            versions.add(tomllib.load(stream)["package"]["version"])
    version = next(iter(versions))
    if len(versions) != 1 or re.fullmatch(VERSION_PATTERN, version) is None:
        raise ValueError("Core, CLI and GUI must have the same three-component version, optionally with an rc suffix")
    return version


def package_windows(binaries, payload, dist, version):
    """Publish the built Windows executables directly, with a separate notices archive."""
    suffix = f"{version}-windows-x64"
    for name in ROOTS:
        shutil.copy2(binaries / f"{name}.exe", dist / f"{name}-{suffix}.exe")
    with tarfile.open(dist / f"pancake-notices-{suffix}.tar.gz", "x:gz") as archive:
        for name in ("LICENSE", "dependency-notices", "native-llvm"):
            archive.add(payload / name, arcname=name)


def smoke_stack(executable, frames):
    """Run a CPU stack from the distributed CLI and verify it produced a TIFF."""
    with tempfile.TemporaryDirectory(prefix="pancake-smoke-") as temporary:
        output = Path(temporary) / "stacked.tif"
        subprocess.run(
            [executable, "--cpu", "--input", frames / "focus-near.png", frames / "focus-far.png",
             "--output", output],
            check=True, timeout=60,
        )
        if output.is_file():
            with output.open("rb") as stream:
                if stream.read(4) in (b"II*\0", b"MM\0*"):
                    return
        raise ValueError("Pancake CLI did not write a TIFF")


def main():
    """Expose the shared release operations to GitHub Actions."""
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    commands.add_parser("version")
    notices = commands.add_parser("notices")
    notices.add_argument("payload", type=Path)
    notices.add_argument("target")
    notices.add_argument("--features")
    windows = commands.add_parser("windows-assets")
    windows.add_argument("binaries", type=Path)
    windows.add_argument("payload", type=Path)
    windows.add_argument("dist", type=Path)
    windows.add_argument("version")
    smoke = commands.add_parser("smoke")
    smoke.add_argument("executable", type=Path)
    args = parser.parse_args()
    if args.command == "version":
        print(release_version(SOURCE))
    elif args.command == "notices":
        revision = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=SOURCE, text=True).strip()
        lock_digest = file_sha256(SOURCE / "Cargo.lock")
        receipt = stage_notices(args.payload, SOURCE, revision, lock_digest, target=args.target,
                                features=args.features, directory=Path("dependency-notices"))
        verify_notices(args.payload, receipt)
        shutil.copy2(SOURCE / "LICENSE", args.payload)
        native = args.payload / "native-llvm"
        native.mkdir()
        for name in ("LICENSE.TXT", "BLAKE3-LICENSE"):
            shutil.copy2(SOURCE / "notices/llvm" / name, native)
    elif args.command == "windows-assets":
        package_windows(args.binaries, args.payload, args.dist, args.version)
    else:
        smoke_stack(args.executable.resolve(strict=True), SOURCE.parent / "assets/quickstart")


if __name__ == "__main__":
    main()
