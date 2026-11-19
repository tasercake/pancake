"""Prove the selected Tracel LLVM archives are present in an existing Linux release build."""

import argparse
from collections import Counter
import hashlib
import json
from pathlib import Path
import re
import subprocess
import sys
import tomllib

from package_notices import file_sha256

BINARIES = ("pancake-cli", "pancake-gui")
NATIVE_DOCS = Path("usr/share/doc/pancake/native-llvm")
NOTICE_SOURCE = Path(__file__).resolve().parents[1] / "notices/llvm"
GIT_OBJECT = re.compile(r"[0-9a-f]{40}\Z")
SHA256 = re.compile(r"[0-9a-f]{64}\Z")
ARCHIVE_NAME = re.compile(r"libLLVM[A-Za-z0-9_]+\.a")
BUNDLE_KEYS = ("target", "wrapper_version", "archive_sha256", "archive_sidecar_sha256",
               "installed_content_sha256", "upstream_commit")
RECEIPT_KEYS = {*BUNDLE_KEYS, "schema_version", "source_commit", "cargo_lock_sha256", "binaries",
                "link_output_sha256", "static_llvm_inputs", "native_rlib"}


def static_llvm_inputs(build_output, library_directory):
    """Resolve the static LLVM archives recorded in an existing Cargo link trace."""
    directory = library_directory.resolve(strict=True)
    paths = []
    libraries = []
    for line in build_output.read_text(encoding="utf-8").splitlines():
        if not line.startswith("cargo:"):
            continue
        directive, _, value = line.removeprefix("cargo:").removeprefix(":").partition("=")
        if directive == "rustc-link-search" and value.startswith("native="):
            paths.append(Path(value.removeprefix("native=")).resolve())
        if directive == "rustc-link-lib" and re.search(r"(?:^|=)(?:lib)?LLVM", value):
            matched = re.fullmatch(r"static=(LLVM[A-Za-z0-9_]+)", value)
            if matched is None:
                raise ValueError("LLVM link directive is not a recognized static archive")
            libraries.append(matched.group(1))
    if paths.count(directory) != 1:
        raise ValueError("Cargo LLVM link search path differs from the verified installation")
    if not libraries or len(libraries) != len(set(libraries)):
        raise ValueError("Missing or duplicate static LLVM link directives")
    inputs = [directory / f"lib{name}.a" for name in libraries]
    for archive in inputs:
        if not archive.is_file() or archive.is_symlink():
            raise ValueError(f"Missing regular LLVM archive {archive.name}")
        if any(path != directory and (path / archive.name).exists() for path in paths):
            raise ValueError(f"Ambiguous LLVM archive in native search paths: {archive.name}")
    return inputs


def _archive_objects(path):
    """Hash GNU ar member bytes, excluding only ar symbol and filename indexes."""
    objects = Counter()
    limit = path.stat().st_size
    with path.open("rb") as stream:
        if stream.read(8) != b"!<arch>\n":
            raise ValueError(f"Malformed GNU archive: {path.name}")
        while stream.tell() < limit:
            header = stream.read(60)
            try:
                size = int(header[48:58])
            except ValueError as error:
                raise ValueError(f"Malformed member size in {path.name}") from error
            if len(header) != 60 or header[58:] != b"`\n" or size < 0 or size + size % 2 > limit - stream.tell():
                raise ValueError(f"Malformed member header in {path.name}")
            name = header[:16].strip()
            if name.startswith(b"#1/"):
                raise ValueError(f"Non-GNU extended archive member in {path.name}")
            if name in {b"/", b"//", b"/SYM64/"}:
                stream.seek(size, 1)
            else:
                digest = hashlib.sha256()
                remaining = size
                while remaining:
                    block = stream.read(min(remaining, 1 << 20))
                    if not block:
                        raise ValueError(f"Malformed member payload in {path.name}")
                    digest.update(block)
                    remaining -= len(block)
                objects[digest.hexdigest()] += 1
            if size % 2:
                stream.read(1)
    return objects


def confirm_archive_members(libraries, rlib):
    """Verify every selected native object is bundled, preserving multiplicity."""
    expected = Counter()
    for library in libraries:
        expected.update(_archive_objects(library))
    if not expected or expected - _archive_objects(rlib):
        raise ValueError("The CubeCL rlib does not contain the exact selected native LLVM objects")
    return {
        "rlib_sha256": file_sha256(rlib),
        "rlib_bytes": rlib.stat().st_size,
        "matched_native_objects": expected.total(),
        "distinct_native_object_digests": len(expected),
    }


def _tree_digest(prefix):
    """Reproduce the pinned bundler checksum before its empty install sentinel."""
    sentinel = prefix / ".tracel-llvm-installed"
    if not sentinel.is_file() or sentinel.is_symlink() or sentinel.stat().st_size != 0:
        raise ValueError("Invalid Tracel LLVM installation sentinel")
    paths = sorted((path.relative_to(prefix).as_posix(), path)
                   for path in prefix.rglob("*")
                   if path != sentinel and path.is_file() and not path.is_symlink())
    digest = hashlib.sha256()
    for relative, path in paths:
        digest.update(f"{relative}\n{path.stat().st_size}\n".encode())
        with path.open("rb") as stream:
            while block := stream.read(1 << 20):
                digest.update(block)
    return digest.hexdigest()


def _file_identity(path):
    return {"sha256": file_sha256(path), "bytes": path.stat().st_size}


def _manifest(directory):
    """Load the committed native-bundle identity and exact available notice bytes."""
    manifest = json.loads((directory / "manifest.json").read_bytes())
    if manifest.get("schema_version") != 1 or not manifest.get("notices"):
        raise ValueError("Malformed pinned LLVM notice manifest")
    for filename, entry in manifest["notices"].items():
        if Path(filename).name != filename or _file_identity(directory / filename) != {
                "sha256": entry["sha256"], "bytes": entry["bytes"]}:
            raise ValueError("Pinned LLVM notice file differs from manifest")
    return manifest


def _locked_wrapper(lock, pins):
    matches = [entry for entry in tomllib.loads(lock.read_text())["package"]
               if entry["name"] == "tracel-llvm-bundler"]
    if (len(matches) != 1 or matches[0]["version"] != pins["wrapper_version"]
            or matches[0].get("checksum") != pins["wrapper_crate_sha256"]):
        raise ValueError("Tracel LLVM wrapper identity differs from the reviewed native bundle")


def capture_build_provenance(*, source, release, build_output, rlib, archive, sidecar, installed, notices):
    """Validate existing build inputs. Record the final binaries from the same Cargo target."""
    manifest = _manifest(notices)
    release = release.resolve(strict=True)
    crate_build = release / "build/cubecl-llvm"
    if not all(path.resolve(strict=True).is_relative_to(crate_build) for path in (build_output, rlib)):
        raise ValueError("LLVM link output and rlib must come from this release target directory")
    git = ["git", "-C", str(source)]
    dirty = subprocess.check_output([*git, "status", "--porcelain=v1", "--untracked-files=all", "--", "."],
                                    text=True)
    if dirty:
        raise ValueError("Commit Pancake source changes before recording native provenance")
    revision = subprocess.check_output([*git, "rev-parse", "HEAD"], text=True).strip()
    lock = source / "Cargo.lock"
    _locked_wrapper(lock, manifest)
    checksums = json.loads(sidecar.read_text(encoding="utf-8-sig"))
    version, release_number = manifest["wrapper_version"].rsplit("-", 1)
    if (file_sha256(sidecar) != manifest["archive_sidecar_sha256"]
            or checksums.get("archive_sha256") != manifest["archive_sha256"]
            or checksums.get("content_sha256") != manifest["installed_content_sha256"]
            or checksums.get("platform") != manifest["platform"]
            or checksums.get("version") != version or checksums.get("release_number") != release_number):
        raise ValueError("Tracel LLVM checksum sidecar differs from the pinned release")
    if file_sha256(archive) != manifest["archive_sha256"] or _tree_digest(installed) != manifest["installed_content_sha256"]:
        raise ValueError("Tracel LLVM archive or installed content differs from the pinned release")
    vcs = (installed / "include/llvm/Support/VCSRevision.h").read_text().splitlines()
    if (f'#define LLVM_REVISION R"({manifest["upstream_commit"]})"' not in vcs
            or f'#define LLVM_REPOSITORY R"({manifest["upstream_repository"]}.git)"' not in vcs
            or file_sha256(installed / "include/llvm/Support/LICENSE.TXT")
            != manifest["notices"]["Support-NOTICE.TXT"]["sha256"]):
        raise ValueError("LLVM upstream revision or bundled notice differs from the pinned release")
    libraries = static_llvm_inputs(build_output, installed / "lib")
    if len(libraries) != manifest["static_llvm_library_count"]:
        raise ValueError("Static LLVM link input count differs from the reviewed release")
    native = confirm_archive_members(libraries, rlib)
    return {
        "schema_version": 1,
        "source_commit": revision,
        "cargo_lock_sha256": file_sha256(lock),
        "binaries": {name: _file_identity(release / name) for name in BINARIES},
        "target": manifest["target"],
        "wrapper_version": manifest["wrapper_version"],
        "archive_sha256": manifest["archive_sha256"],
        "archive_sidecar_sha256": manifest["archive_sidecar_sha256"],
        "installed_content_sha256": manifest["installed_content_sha256"],
        "upstream_commit": manifest["upstream_commit"],
        "link_output_sha256": file_sha256(build_output),
        "static_llvm_inputs": [{"name": library.name, **_file_identity(library)} for library in libraries],
        "native_rlib": native,
    }


def _matches(pattern, value):
    return isinstance(value, str) and pattern.fullmatch(value) is not None


def validate_build_provenance(record, pins, revision, lock_sha256, binaries):
    """Bind well-formed native evidence to the exact binaries being packaged."""
    if not isinstance(record, dict) or set(record) != RECEIPT_KEYS:
        raise ValueError("Malformed native LLVM receipt")
    if (record["schema_version"] != 1 or not _matches(GIT_OBJECT, revision)
            or not _matches(SHA256, lock_sha256) or record["source_commit"] != revision
            or record.get("cargo_lock_sha256") != lock_sha256 or record.get("binaries") != binaries):
        raise ValueError("Native LLVM receipt and package source, lock, or binary identities differ")
    for key in BUNDLE_KEYS:
        if record.get(key) != pins.get(key) or key not in pins:
            raise ValueError(f"Native LLVM receipt {key} differs from the reviewed bundle")
    inputs = record.get("static_llvm_inputs")
    if not isinstance(inputs, list) or len(inputs) != pins["static_llvm_library_count"]:
        raise ValueError("Native LLVM receipt has the wrong static input count")
    if (not all(isinstance(entry, dict) and set(entry) == {"name", "sha256", "bytes"}
                and _matches(ARCHIVE_NAME, entry["name"]) and _matches(SHA256, entry["sha256"])
                and type(entry["bytes"]) is int
                and entry["bytes"] > 0 for entry in inputs)
            or len({entry["name"] for entry in inputs}) != len(inputs)):
        raise ValueError("Malformed native LLVM static input evidence")
    rlib = record.get("native_rlib")
    if (not isinstance(rlib, dict)
            or set(rlib) != {"rlib_sha256", "rlib_bytes", "matched_native_objects", "distinct_native_object_digests"}
            or not _matches(SHA256, rlib["rlib_sha256"])
            or type(rlib["rlib_bytes"]) is not int or rlib["rlib_bytes"] <= 0
            or type(rlib["matched_native_objects"]) is not int or rlib["matched_native_objects"] < len(inputs)
            or type(rlib["distinct_native_object_digests"]) is not int
            or not 0 < rlib["distinct_native_object_digests"] <= rlib["matched_native_objects"]
            or not _matches(SHA256, record["link_output_sha256"])):
        raise ValueError("Incomplete CubeCL rlib consumption evidence")


def stage_native_llvm_notices(payload, notice_source, provenance, revision, lock_sha256, binaries):
    """Stage source-pinned notices and an independently gathered build receipt."""
    pins = _manifest(notice_source)
    record = json.loads(provenance.read_bytes())
    validate_build_provenance(record, pins, revision, lock_sha256, binaries)
    destination = payload / NATIVE_DOCS
    destination.mkdir(parents=True)
    files = {}
    for name in sorted((*pins["notices"], "manifest.json")):
        output = destination / name
        output.write_bytes((notice_source / name).read_bytes())
        files[output.relative_to(payload).as_posix()] = _file_identity(output)
    output = destination / "build-provenance.json"
    output.write_text(json.dumps(record, indent=2) + "\n", encoding="utf-8")
    files[output.relative_to(payload).as_posix()] = _file_identity(output)
    return files


def verify_native_llvm_notices(extracted, files):
    """Recheck every staged native notice and provenance file after extraction."""
    for path, expected in files.items():
        if _file_identity(extracted / path) != expected:
            raise ValueError(f"Packaged native LLVM file changed: {path}")


def main(argv=None):
    """Audit existing local artifacts and emit an explicit release identity."""
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ("source", "release", "build-output", "rlib", "archive", "sidecar", "installed"):
        parser.add_argument("--" + name, required=True, type=Path)
    args = parser.parse_args(argv)
    try:
        print(json.dumps(capture_build_provenance(**vars(args), notices=NOTICE_SOURCE), indent=2))
    except (OSError, ValueError, TypeError, KeyError, subprocess.CalledProcessError) as error:
        print(f"Native LLVM provenance: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
