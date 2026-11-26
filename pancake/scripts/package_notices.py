"""Archive original notices and source provenance for locked Pancake release graphs."""

import hashlib
import json
import re
import subprocess
import tarfile
import tomllib
from collections import Counter
from pathlib import Path, PurePosixPath

TARGET = "x86_64-unknown-linux-gnu"
ROOTS = ("pancake-cli", "pancake-gui")
DOC_DIRECTORY = Path("usr/share/doc/pancake/dependency-notices")
GUI_FONT_NOTICES = tuple(PurePosixPath("assets/fonts") / name
                         for name in ("IBMPlexMono-OFL.txt", "SpaceGrotesk-OFL.txt"))
NOTICE_NAME = re.compile(
    r"^(?:licen[cs]es?|licensing|unlicense|copying|copyright|notices?|authors|patents|"
    r"third[-_ ]party[-_ ](?:licenses?|notices?))"
    r"(?:$|[._ -])",
    re.IGNORECASE,
)


def file_sha256(path):
    """Stream a file digest without materializing a binary or source archive."""
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def _command(source, *args):
    """Read bytes from the exact source's local Cargo or Git checkout."""
    return subprocess.run(args, cwd=source, capture_output=True, check=True).stdout


def _validate_source(source, revision, lock_digest):
    """Ensure the packing source and graph inputs agree with the named revision."""
    actual = _command(source, "git", "rev-parse", "HEAD").decode().strip()
    if actual != revision or file_sha256(source / "Cargo.lock") != lock_digest:
        raise ValueError("Notice source revision or Cargo.lock differs from the package identity")
    dirty = _command(source, "git", "status", "--porcelain=v1", "--untracked-files=all", "--",
                     "Cargo.toml", "Cargo.lock", ".cargo", "rust-toolchain.toml", "crates", "vendor", "LICENSE", "notices")
    if dirty:
        raise ValueError("Commit Cargo graph and vendored source changes before packaging notices")
    return Path(_command(source, "git", "rev-parse", "--show-toplevel").decode().strip())


def _production_packages(source, target, features):
    """Select the packages used to build the CLI and GUI using Cargo's build graph."""
    tree = ["cargo", "tree", "--locked", "--offline", "--target", target,
            "--edges", "normal,build", "--prefix", "none", "--format", "{p}"]
    feature_args = ("--features", features) if features else ()
    if features:
        tree.extend(("--workspace", *feature_args))
    else:
        for name in ROOTS:
            tree.extend(("-p", name))
    selected = set()
    for line in _command(source, *tree).decode().splitlines():
        if not line:
            continue
        match = re.match(r"^([A-Za-z0-9_-]+) v([^ ]+)(?: |$)", line)
        if match is None:
            raise ValueError(f"Unrecognized Cargo package in release graph: {line}")
        selected.add(match.groups())
    # Build dependencies can select host-only packages during a cross-build.
    compiler = _command(source, "rustc", "-vV").decode().splitlines()
    host = next(line.removeprefix("host: ") for line in compiler if line.startswith("host: "))
    platform_args = tuple(arg for platform in dict.fromkeys((target, host))
                          for arg in ("--filter-platform", platform))
    metadata = _command(source, "cargo", "metadata", "--locked", "--offline", "--format-version", "1",
                        *platform_args, *feature_args)
    by_id = {}
    for package in json.loads(metadata)["packages"]:
        key = package["name"], package["version"]
        if key in selected:
            if key in by_id:
                raise ValueError(f"Ambiguous Cargo source record for {key[0]} {key[1]}")
            by_id[key] = package
    if by_id.keys() != selected or not all(any(name == root for name, _ in selected) for root in ROOTS):
        raise ValueError("Locked Cargo metadata differs from the release package graph")
    return tree, [by_id[key] for key in sorted(selected)]


def _notice_path(path, explicit):
    """Identify source notice files and Cargo's explicitly declared license-file."""
    return (
        not path.is_absolute() and ".." not in path.parts
        and (str(path) == explicit or NOTICE_NAME.match(path.name) is not None
             or any(part.casefold() in {"licenses", "licences"} for part in path.parts[:-1]))
    )


def _registry_notices(package, lock_entry):
    """Read notice bytes directly from the Cargo.lock-verified registry archive."""
    name = f"{package['name']}-{package['version']}"
    crate = Path(package["manifest_path"]).parent
    archive = crate.parents[2] / "cache" / crate.parent.name / f"{name}.crate"
    expected = lock_entry.get("checksum")
    provenance = {"archive": archive.name, "locked_archive_sha256": expected}
    if not archive.is_file():
        return provenance, "archive_missing", {}
    actual = file_sha256(archive)
    if actual != expected:
        return {**provenance, "actual_archive_sha256": actual}, "archive_checksum_mismatch", {}
    files = {}
    with tarfile.open(archive, "r:gz") as source:
        for member in source:
            path = PurePosixPath(member.name)
            if not member.isfile() or path.is_absolute() or len(path.parts) < 2 or path.parts[0] != name:
                continue
            relative = path.relative_to(name)
            if _notice_path(relative, package["license_file"]):
                if relative in files:
                    raise ValueError(f"Duplicate notice path in {archive.name}: {relative}")
                with source.extractfile(member) as stream:
                    files[relative] = stream.read()
    return provenance, "present" if files else "no_notice_in_source", files


def _local_notices(package, source, repository, revision, vendor_map):
    """Read original notice blobs from the named committed source tree."""
    crate = Path(package["manifest_path"]).parent.resolve()
    if not crate.is_relative_to(source):
        return {}, "external_path_source", {}
    relative = crate.relative_to(repository).as_posix()
    vendored = crate.is_relative_to(source / "vendor")
    listing = _command(repository, "git", "ls-tree", "-r", "-z", revision, "--", relative)
    font_notices = set(GUI_FONT_NOTICES) if package["name"] == "pancake-gui" else set()
    files = {}
    for entry in filter(None, listing.split(b"\0")):
        metadata, name = entry.split(b"\t", 1)
        mode, _, blob = metadata.decode().split()
        path = PurePosixPath(name.decode()).relative_to(relative)
        font_notice = path in font_notices
        # The workspace root also contains sibling crates and the complete vendor tree.
        if (not vendored and len(path.parts) > 1 and path.parts[0].casefold() not in {"licenses", "licences"}
                and not font_notice and str(path) != package["license_file"]):
            continue
        if mode in {"100644", "100755"} and (font_notice or _notice_path(path, package["license_file"])):
            files[path] = _command(repository, "git", "cat-file", "blob", blob)
    missing = font_notices - files.keys()
    if missing:
        raise ValueError(f"Missing bundled font notices: {', '.join(str(path) for path in sorted(missing))}")
    provenance = {"commit": revision, "directory": relative}
    if vendored:
        provenance["upstream_patch"] = vendor_map.get((package["name"], package["version"], crate.name))
    return provenance, "present" if files else "no_notice_in_source", files


def bind_shared_project_license(entries):
    """Reference one physical Pancake license from the matching launcher rows."""
    project = next((entry for entry in entries
                    if entry["name"] == "pancake" and entry["source"] is None
                    and entry["declared_license"] == "GPL-3.0-or-later"), None)
    if project is None:
        return
    shared = next((notice for notice in project["notices"]
                   if len(PurePosixPath(notice["file"]).parts) == 2
                   and PurePosixPath(notice["file"]).name == "LICENSE"), None)
    if shared is None:
        return
    for entry in entries:
        if (entry["name"] in ROOTS and entry["source"] is None
                and entry["declared_license"] == project["declared_license"]):
            entry["shared_project_license"] = shared
            if entry["status"] == "no_notice_in_source":
                entry["status"] = "covered_by_shared_project_notice"


def stage_notices(payload, source, revision, lock_digest, *, target=TARGET, features=None,
                  directory=DOC_DIRECTORY):
    """Copy all discovered notices and write an informational inventory."""
    source = source.resolve(strict=True)
    repository = _validate_source(source, revision, lock_digest)
    graph_command, packages = _production_packages(source, target, features)
    locked = {(entry["name"], entry["version"]): entry
              for entry in tomllib.loads((source / "Cargo.lock").read_text())["package"]}
    vendor_path = source / "vendor/upstream-sources.json"
    vendor_map = {}
    if vendor_path.is_file():
        pinned = json.loads(vendor_path.read_text())["packages"]
        vendor_map = {(entry["name"], entry["version"], entry["directory"]): entry for entry in pinned}
    destination = payload / directory
    destination.mkdir(parents=True)
    entries = []
    for package in packages:
        key = package["name"], package["version"]
        record = locked.get(key)
        origin = package["source"]
        if record is None or record.get("source") != origin:
            raise ValueError(f"Missing matching Cargo.lock source for {key[0]} {key[1]}")
        if origin and origin.startswith("registry+"):
            provenance, status, files = _registry_notices(package, record)
        elif origin is None:
            provenance, status, files = _local_notices(package, source, repository, revision, vendor_map)
        else:
            provenance, status, files = {}, "unsupported_source", {}
        pkg_path = Path(f"{key[0]}-{key[1]}")
        notices = []
        for relative, content in sorted(files.items()):
            archive_path = pkg_path / relative
            output = destination / archive_path
            output.parent.mkdir(parents=True, exist_ok=True)
            output.write_bytes(content)
            notices.append({"file": archive_path.as_posix(), "bytes": len(content),
                            "sha256": hashlib.sha256(content).hexdigest()})
        entries.append({"name": key[0], "version": key[1], "source": origin,
                        "declared_license": package["license"], "declared_license_file": package["license_file"],
                        "status": status, "provenance": provenance, "notices": notices})

    bind_shared_project_license(entries)
    inventory = destination / "inventory.json"
    counts = dict(Counter(entry["status"] for entry in entries))
    inventory.write_text(json.dumps({
        "schema_version": 1,
        "source_commit": revision,
        "cargo_lock_sha256": lock_digest,
        "target": target,
        "roots": ROOTS,
        "features": features or "workspace defaults",
        "cargo_version": _command(source, "cargo", "--version").decode().strip(),
        "graph_command": graph_command,
        "selected_packages": len(entries),
        "counts": counts,
        "packages": entries,
    }, indent=2) + "\n", encoding="utf-8")
    return {"file": inventory.relative_to(payload).as_posix(), "sha256": file_sha256(inventory),
            "packages": len(entries), "status_counts": counts,
            "files": sum(len(entry["notices"]) for entry in entries)}


def verify_notices(extracted, receipt):
    """Confirm the inventory and every archived notice survived staging or extraction."""
    path = extracted / receipt["file"]
    if file_sha256(path) != receipt["sha256"]:
        raise ValueError("Packaged dependency notice inventory changed")
    packages = json.loads(path.read_text())["packages"]
    physical = {notice["file"]: notice for package in packages for notice in package["notices"]}
    for package in packages:
        shared = package.get("shared_project_license")
        if shared is not None and physical.get(shared["file"]) != shared:
            raise ValueError("Packaged shared project license reference changed")
        for notice in package["notices"]:
            actual = path.parent / notice["file"]
            if actual.stat().st_size != notice["bytes"] or file_sha256(actual) != notice["sha256"]:
                raise ValueError(f"Packaged dependency notice changed: {notice['file']}")
