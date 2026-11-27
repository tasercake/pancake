"""Build a local amd64 Debian package from already-built Pancake release binaries."""

import argparse
import json
import os
import platform
import re
import shutil
import subprocess
import sys
import tempfile
import tomllib
from pathlib import Path

from package_native_llvm import stage_native_llvm_notices, verify_native_llvm_notices
from package_notices import file_sha256, stage_notices, verify_notices
from release_support import debian_version

BINARIES = ("pancake-cli", "pancake-gui")
# Slint/winit and Vulkan can load these at runtime without an ELF NEEDED entry.
SESSION_DEPENDS = (
    "at-spi2-core",
    "libfontconfig1",
    "libvulkan1",
    "libwayland-client0",
    "libxcursor1",
    "libxi6",
    "libx11-6",
    "libx11-xcb1",
    "libxcb1",
    "libxkbcommon0",
    "libxkbcommon-x11-0",
    "xdg-utils",
)


def _run(*args, cwd=None):
    """Collect reproducible native-tool diagnostics in the C locale."""
    return subprocess.run(args, cwd=cwd, capture_output=True, check=True, text=True,
                          env={**os.environ, "LC_ALL": "C"})


def _elf_header(path):
    """Require a directly executable x86-64 ELF file."""
    if not path.is_file() or not os.access(path, os.X_OK):
        raise ValueError(f"Missing native Linux executable: {path}")
    with path.open("rb") as stream:
        if stream.read(4) != b"\x7fELF":
            raise ValueError(f"Expected a native Linux executable: {path}")
    header = _run("readelf", "--wide", "--file-header", path).stdout
    if not ("Advanced Micro Devices X86-64" in header and
            re.search(r"^\s*Entry point address:\s*0x[0-9a-f]*[1-9a-f][0-9a-f]*$", header, re.MULTILINE) and
            re.search(r"^\s*Type:\s*(DYN|EXEC)\b", header, re.MULTILINE)):
        raise ValueError(f"Expected a native Linux x86-64 executable: {path}")
    return header


def _dependency_report(staging, binaries, control):
    """Resolve actual ELF dependencies in the package build distribution."""
    debian = staging / "debian"
    debian.mkdir(exist_ok=True)
    (debian / "control").write_text(f"Source: pancake\nSection: graphics\nPriority: optional\n\n{control}",
                                    encoding="utf-8")
    result = _run("dpkg-shlibdeps", "-O", *(f"-e{path}" for path in binaries), cwd=staging)
    linked = result.stdout.strip().removeprefix("shlibs:Depends=")
    if not linked or linked == result.stdout.strip():
        raise ValueError("dpkg-shlibdeps did not resolve native dependencies")
    dependencies = linked.split(", ")
    provided = {dependency.partition(" ")[0] for dependency in dependencies if "|" not in dependency}
    dependencies.extend(dependency for dependency in SESSION_DEPENDS if dependency not in provided)
    return ", ".join(dependencies), result.stderr


def build_package(release, output, version, maintainer, revision, lock_digest, *, source_root,
                  native_llvm_receipt=None, archive_version=None):
    """Stage, extract and verify the package before publishing local artifacts."""
    if sys.platform != "linux" or platform.machine() != "x86_64":
        raise ValueError("Build the Debian package on a Linux x86-64 package host")
    if output.exists():
        raise ValueError(f"Package output already exists: {output}")
    if not re.fullmatch(r"[0-9A-Za-z.+~\-]+", version):
        raise ValueError(f"Invalid package version: {version}")
    _run("dpkg", "--validate-version", version)
    if not re.fullmatch(r"[^<>\r\n]+ <[^<>\s]+@[^<>\s]+>", maintainer):
        raise ValueError("Maintainer must use the Debian name <email> format")
    if not (re.fullmatch(r"[0-9a-f]{40}", revision) and re.fullmatch(r"[0-9a-f]{64}", lock_digest)):
        raise ValueError("Missing source revision or Cargo.lock digest")
    release = release.resolve(strict=True)
    output = output.resolve()
    headers = {name: _elf_header(release / name) for name in BINARIES}
    control = (f"Package: pancake\nVersion: {version}\nArchitecture: amd64\n"
               f"Maintainer: {maintainer}\nSection: graphics\nPriority: optional\n"
               "Description: Focus stacking and exposure fusion\n")
    output.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix=".pancake-package-", dir=output.parent) as scratch:
        staging = Path(scratch)
        payload = staging / "debian" / "pancake"
        bindir = payload / "usr" / "bin"
        bindir.mkdir(parents=True)
        audit = {}
        for name in BINARIES:
            source, binary = release / name, bindir / name
            shutil.copyfile(source, binary)
            binary.chmod(0o755)
            checksum = file_sha256(binary)
            if checksum != file_sha256(source):
                raise ValueError(f"Release binary changed while packaging: {source}")
            audit[name] = {"sha256": checksum, "bytes": binary.stat().st_size, "elf_header": headers[name],
                           "dynamic": _run("readelf", "--wide", "--dynamic", binary).stdout,
                           "symbols": _run("readelf", "--wide", "--version-info", binary).stdout}
        dependencies, diagnostics = _dependency_report(staging, [bindir / name for name in BINARIES], control)
        applications = payload / "usr" / "share" / "applications"
        applications.mkdir(parents=True)
        (applications / "pancake.desktop").write_text(
            "[Desktop Entry]\nType=Application\nName=Pancake\n"
            "Exec=pancake-gui %F\nCategories=Graphics;Photography;\n", encoding="utf-8")
        notices = stage_notices(payload, source_root, revision, lock_digest)
        native_notices = None
        if native_llvm_receipt is not None:
            identities = {name: {key: record[key] for key in ("sha256", "bytes")} for name, record in audit.items()}
            native_notices = stage_native_llvm_notices(payload, source_root / "notices/llvm",
                                                      native_llvm_receipt, revision, lock_digest, identities)
        metadata = payload / "DEBIAN"
        metadata.mkdir()
        (metadata / "control").write_text(f"{control}Depends: {dependencies}\n", encoding="utf-8")
        archive_name = f"pancake_{archive_version or version}_amd64.deb"
        archive = staging / archive_name
        _run("dpkg-deb", "--root-owner-group", "--build", payload, archive)
        extracted = staging / "verify"
        _run("dpkg-deb", "--extract", archive, extracted)
        verify_notices(extracted, notices)
        if native_notices is not None:
            verify_native_llvm_notices(extracted, native_notices)
        for name, expected in audit.items():
            binary = extracted / "usr" / "bin" / name
            if not os.access(binary, os.X_OK) or file_sha256(binary) != expected["sha256"]:
                raise ValueError(f"Packaged binary differs from release: {name}")
        receipt = {"schema_version": 1, "source_commit": revision, "cargo_lock_sha256": lock_digest,
                   "platform": platform.platform(), "dpkg_deb": _run("dpkg-deb", "--version").stdout.splitlines()[0],
                   "depends": dependencies, "shlibdeps_diagnostics": diagnostics, "binaries": audit,
                   "dependency_notices": notices,
                   "archive": {"file": archive_name, "sha256": file_sha256(archive), "bytes": archive.stat().st_size}}
        if native_notices is not None:
            receipt["native_llvm"] = native_notices
        output.mkdir()
        archive.rename(output / archive_name)
        (output / "package-receipt.json").write_text(json.dumps(receipt, indent=2) + "\n", encoding="utf-8")


def main(argv=None):
    """Package the selected locked build from this source checkout."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--release-dir", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--maintainer", required=True)
    parser.add_argument("--native-llvm-receipt", type=Path,
                        help="Include native LLVM notices and exact release-build evidence")
    args = parser.parse_args(argv)
    pancake = Path(__file__).resolve().parents[1]
    try:
        manifest = tomllib.loads((pancake / "Cargo.toml").read_text(encoding="utf-8"))
        dirty = _run("git", "-C", pancake, "status", "--porcelain=v1", "--untracked-files=all", "--", ".").stdout
        if dirty:
            raise ValueError(f"Commit Pancake source changes before packaging:\n{dirty}")
        revision = _run("git", "-C", pancake, "rev-parse", "HEAD").stdout.strip()
        version = manifest["package"]["version"]
        build_package(args.release_dir, args.output, debian_version(version), args.maintainer,
                      revision, file_sha256(pancake / "Cargo.lock"), source_root=pancake,
                      native_llvm_receipt=args.native_llvm_receipt, archive_version=version)
        print(args.output / "package-receipt.json")
        return 0
    except subprocess.CalledProcessError as error:
        print(f"Linux package: {error}\n{error.stderr}", file=sys.stderr)
    except (OSError, ValueError, KeyError) as error:
        print(f"Linux package: {error}", file=sys.stderr)
    return 1


if __name__ == "__main__":
    sys.exit(main())
