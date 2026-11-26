"""Build and verify a private, ad-hoc signed macOS ARM64 Pancake package."""

import argparse
import json
import os
import platform
import plistlib
import re
import shutil
import subprocess
import tempfile
import tomllib
from pathlib import Path, PurePosixPath

from package_notices import GUI_FONT_NOTICES, file_sha256


TARGET = "aarch64-apple-darwin"
BINARIES = ("pancake-cli", "pancake-gui")
SYSTEM_LOCATIONS = ("/System/Library/Frameworks/", "/usr/lib/")
BUILD_OVERRIDES = (
    "DEVELOPER_DIR", "SDKROOT", "MACOSX_DEPLOYMENT_TARGET",
    "CARGO_TARGET_AARCH64_APPLE_DARWIN_LINKER", "PANCAKE_ZSTD_ARCHIVE",
)


def _apple(source, *command):
    """Run the exact native tool command and retain its combined output."""
    completed = subprocess.run(command, cwd=source, stdout=subprocess.PIPE,
                               stderr=subprocess.STDOUT, text=True, check=False)
    if completed.returncode:
        raise ValueError(f"{command[0]} exited {completed.returncode}\n{completed.stdout}")
    return completed.stdout


def _source_identity(source, owned_scratch=None):
    """Require clean source before and after preparing the package."""
    def git(*command, cwd=source):
        return subprocess.check_output(["git", *command], cwd=cwd, text=True).strip()

    repository = Path(git("rev-parse", "--show-toplevel")).resolve()
    scope = ()
    if owned_scratch is not None and owned_scratch.is_relative_to(repository):
        relative = owned_scratch.relative_to(repository).as_posix()
        scope = ("--", ".", f":(exclude,literal){relative}")
    if git("status", "--porcelain=v1", "--untracked-files=all", *scope, cwd=repository):
        raise ValueError("Package from a clean Git worktree")
    return {
        "source_commit": git("rev-parse", "HEAD"),
        "source_tree": git("rev-parse", "HEAD^{tree}"),
        "cargo_lock_sha256": file_sha256(source / "Cargo.lock"),
    }


def _bundle_metadata(source, identifier, build_number):
    """Read the shared release version and create the reviewed bundle fields."""
    versions = set()
    for name in BINARIES:
        with (source / "crates" / name / "Cargo.toml").open("rb") as manifest:
            versions.add(tomllib.load(manifest)["package"]["version"])
    if len(versions) != 1:
        raise ValueError("The CLI and GUI release versions differ")
    version = versions.pop()
    if re.fullmatch(r"[0-9]+\.[0-9]+\.[0-9]+", version) is None:
        raise ValueError("Use a three-component Cargo release version for the app bundle")
    return {
        "CFBundleDevelopmentRegion": "en",
        "CFBundleDisplayName": "Pancake",
        "CFBundleExecutable": "pancake-gui",
        "CFBundleIdentifier": identifier,
        "CFBundleInfoDictionaryVersion": "6.0",
        "CFBundleName": "Pancake",
        "CFBundlePackageType": "APPL",
        "CFBundleShortVersionString": version,
        "CFBundleVersion": build_number,
    }


def _inspect_binary(source, binary):
    """Require a single ARM64 slice and dependencies available from macOS."""
    architecture = _apple(source, "lipo", "-archs", binary).strip()
    if architecture != "arm64":
        raise ValueError(f"Expected a single arm64 slice for {binary.name}; found {architecture}")
    output = _apple(source, "otool", "-L", binary)
    lines = output.splitlines()
    if lines[:1] != [f"{binary}:"] or len(lines) < 2:
        raise ValueError(f"Missing dependency inspection for {binary.name}")
    dependencies = []
    for entry in lines[1:]:
        dependency, separator, _version = entry.strip().partition(" (")
        if (not separator or not dependency.startswith(SYSTEM_LOCATIONS)
                or ".." in PurePosixPath(dependency).parts):
            raise ValueError(f"Unexpected macOS dependency for {binary.name}: {dependency}")
        dependencies.append(dependency)
    return {"architecture": architecture, "dependencies": dependencies, "otool_libraries": output}


def _sign_and_verify(source, executable, identifier, *, bundle=False):
    """Sign the staged copy and verify the disclosed ad-hoc signing identity."""
    explicit = () if bundle else ("--identifier", identifier)
    _apple(source, "codesign", "--force", "--sign", "-", "--timestamp=none", *explicit, executable)
    verified = _apple(source, "codesign", "--verify", "--strict", "--verbose=2", executable)
    display = _apple(source, "codesign", "--display", "--verbose=4", executable)
    details = set(display.splitlines())
    if f"Identifier={identifier}" not in details or "Signature=adhoc" not in details:
        raise ValueError(f"Unexpected ad-hoc signature identity for {executable.name}")
    return {"verification": verified, "display": display}


def build_package(source, output, identifier, build_number, *, target_dir=None):
    """Build, stage, and verify one private package on native Apple Silicon."""
    if platform.system() != "Darwin" or platform.machine() != "arm64":
        raise ValueError("Package on an Apple Silicon macOS host running an arm64 process")
    if re.fullmatch(r"[A-Za-z0-9-]+(?:\.[A-Za-z0-9-]+)+", identifier) is None:
        raise ValueError("Supply a valid reverse-domain bundle identifier")
    if re.fullmatch(r"[1-9][0-9]{0,3}", build_number) is None:
        raise ValueError("Supply a positive macOS build number from 1 through 9999")
    if output.exists() or output.is_symlink():
        raise FileExistsError(f"Package output already exists: {output}")
    source = source.resolve(strict=True)
    output = output.resolve()
    if target_dir is not None:
        target_dir = target_dir.resolve()
        if target_dir.is_relative_to(output):
            raise ValueError("Keep the Cargo target directory outside the package output")
    identity = _source_identity(source)
    metadata = _bundle_metadata(source, identifier, build_number)
    with (source / "rust-toolchain.toml").open("rb") as pin:
        toolchain = "+" + tomllib.load(pin)["toolchain"]["channel"]
    output.parent.mkdir(parents=True, exist_ok=True)

    with tempfile.TemporaryDirectory(prefix=".pancake-macos-", dir=output.parent) as scratch:
        temporary = Path(scratch)
        build_dir = temporary / "target" if target_dir is None else target_dir
        payload = temporary / "package"
        app = payload / "Pancake.app"
        contents = app / "Contents"
        gui_dir = contents / "MacOS"
        gui_dir.mkdir(parents=True)
        build = ["cargo", toolchain, "build", "--locked", "--release", "--workspace", "--bins",
                 "--target", TARGET, "--features", "cubecl/metal-native", "--target-dir", str(build_dir)]
        sdk = ("xcrun", "--no-cache", "--sdk", "macosx")
        provenance = {
            "cargo": _apple(source, "cargo", toolchain, "--version"),
            "rustc": _apple(source, "rustc", toolchain, "-vV"),
            "macos": _apple(source, "sw_vers"),
            "developer_directory": _apple(source, "xcode-select", "--print-path").strip(),
            "sdk_version": _apple(source, *sdk, "--show-sdk-version").strip(),
            "sdk_path": _apple(source, *sdk, "--show-sdk-path").strip(),
            "clang": _apple(source, *sdk, "clang", "--version").strip(),
            "sdk_linker_path": _apple(source, *sdk, "--find", "ld").strip(),
        }
        overrides = {name: os.environ[name] for name in BUILD_OVERRIDES if name in os.environ}
        (payload / "build.log").write_text(_apple(source, *build), encoding="utf-8")
        release = build_dir / TARGET / "release"
        plist = contents / "Info.plist"
        plist.write_bytes(plistlib.dumps(metadata, fmt=plistlib.FMT_XML))
        plist_check = _apple(source, "plutil", "-lint", plist)
        executables = {}
        for name, directory in zip(BINARIES, (payload, gui_dir), strict=True):
            original, staged = release / name, directory / name
            digest = file_sha256(original)
            shutil.copyfile(original, staged)
            staged.chmod(0o755)
            if file_sha256(staged) != digest:
                raise ValueError(f"Release binary changed while copying {name}")
            executables[name] = {
                "file": staged.relative_to(payload).as_posix(),
                "source_sha256": digest,
                **_inspect_binary(source, staged),
            }
        # Keep the notices with the app when it is moved out of the ZIP.
        font_notices = contents / "Resources/font-notices"
        font_notices.mkdir(parents=True)
        for relative in GUI_FONT_NOTICES:
            original = source / "crates/pancake-gui" / relative
            staged = font_notices / relative.name
            shutil.copyfile(original, staged)
            if file_sha256(staged) != file_sha256(original):
                raise ValueError(f"Font notice changed while copying {relative.name}")
        executables["pancake-cli"]["signature"] = _sign_and_verify(
            source, payload / "pancake-cli", f"{identifier}.cli",
        )
        executables["pancake-gui"]["signature"] = _sign_and_verify(source, app, identifier, bundle=True)
        if plistlib.loads(plist.read_bytes()) != metadata:
            raise ValueError("Info.plist changed during packaging")
        if _source_identity(source, temporary) != identity:
            raise ValueError("Source revision or Cargo.lock changed during packaging")
        receipt = {
            **identity,
            "target": TARGET,
            "build_command": build,
            "build_overrides": overrides,
            "tool_versions": provenance,
            "bundle_identifier": identifier,
            "bundle_metadata": metadata,
            "plist_verification": plist_check,
            "signing_mode": "ad-hoc",
            "executables": executables,
            "files_sha256": {
                item.relative_to(payload).as_posix(): file_sha256(item)
                for item in sorted(payload.rglob("*")) if item.is_file()
            },
        }
        (payload / "package-receipt.json").write_text(json.dumps(receipt, indent=2) + "\n", encoding="utf-8")

        # Claim the output only after every audit passes. mkdir fails even if
        # another process created an empty destination during the build.
        output.mkdir()
        try:
            for item in payload.iterdir():
                item.rename(output / item.name)
        except OSError:
            shutil.rmtree(output)
            raise


def main(argv=None):
    """Expose the local qualification recipe without distribution settings."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--bundle-identifier", required=True)
    parser.add_argument("--build-number", required=True)
    parser.add_argument("--target-dir", type=Path,
                        help="Reuse a Cargo target directory for this checkout (default: a temporary directory)")
    options = parser.parse_args(argv)
    try:
        build_package(Path(__file__).resolve().parents[1], options.output,
                      options.bundle_identifier, options.build_number, target_dir=options.target_dir)
    except (ValueError, OSError, subprocess.CalledProcessError) as error:
        parser.exit(1, f"{error}\n")


if __name__ == "__main__":
    main()
