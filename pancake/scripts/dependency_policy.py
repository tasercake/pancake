#!/usr/bin/env python3
"""Generate narrowly reviewed cargo-deny exceptions and a non-gating license inventory.

Cargo-deny cannot bind advisory ignores to package versions or expire reviews.
This build-time tool supplies those checks before advisories/bans/sources run.
It never changes source manifests, lockfiles, or the reviewed ledger.
"""

import argparse
import datetime
import json
from pathlib import Path
import re
import sys
import tomllib


REVIEWED_UNMAINTAINED = {
    "RUSTSEC-2026-0192": "ttf-parser",
    "RUSTSEC-2026-0370": "zvariant",
    "RUSTSEC-2026-0371": "zbus_names",
    "RUSTSEC-2026-0372": "zvariant_derive",
}
EXACT_VERSION = re.compile(r"\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?\Z")
MAX_REVIEW_DAYS = 90


def protected(name):
    """Identify shared GPU/window ABI boundaries that may not be duplicated."""
    return name in {"wgpu", "naga", "raw-window-handle", "cubecl"} or name.startswith(("wgpu-", "cubecl-"))


def require(condition, message):
    """Return a recoverable policy diagnostic for an unmet invariant."""
    if not condition:
        raise ValueError(message)


def validate_base(config):
    """Keep global advisory, duplicate, source, and explicit-review defaults strict."""
    require("licenses" not in config, "license gate must be removed; inventory is informational")
    advisories, bans, sources = (config.get(key, {}) for key in ("advisories", "bans", "sources"))
    require(advisories.get("unmaintained") == "all" and advisories.get("unsound") == "all",
            "global advisory policy must remain all")
    require(not advisories.get("ignore"), "advisory exceptions must come from the reviewed ledger")
    require(bans.get("multiple-versions") == "deny" and bans.get("wildcards") == "deny",
            "duplicate/wildcard defaults must remain deny")
    require(not bans.get("skip") and not bans.get("skip-tree"),
            "all skips must come from the reviewed exact-version ledger; no skip-tree")
    require(sources.get("unknown-registry") == "deny" and sources.get("unknown-git") == "deny",
            "source defaults must remain deny")
    require(sources.get("allow-registry") == ["https://github.com/rust-lang/crates.io-index"]
            and not sources.get("allow-git"), "unreviewed source allowances are forbidden")


def parent_records(package, metadata):
    """Collect immediate resolved parents and their declared package requirements."""
    packages = {item["id"]: item for item in metadata["packages"]}
    library_names = {target["name"].replace("-", "_") for target in package.get("targets", [])
                     if set(target["kind"]) & {"lib", "rlib", "proc-macro", "dylib", "staticlib", "cdylib"}}
    if not library_names:
        library_names = {package["name"].replace("-", "_")}
    parents = set()
    for node in (metadata.get("resolve") or {}).get("nodes", []):
        if any(edge["pkg"] == package["id"] for edge in node.get("deps", [])):
            parent = packages[node["id"]]
            aliases = {edge.get("name") for edge in node.get("deps", [])
                       if edge["pkg"] == package["id"]}
            requirements = {dep["req"] for dep in parent.get("dependencies", [])
                            if dep["name"] == package["name"]
                            and (None in aliases or (dep.get("rename") is not None
                                 and dep["rename"].replace("-", "_") in aliases)
                                 or (dep.get("rename") is None and bool(library_names & aliases)))}
            require(requirements, f"parent requirement missing for {package['name']}")
            parents.update((parent["name"], parent["version"], requirement) for requirement in requirements)
    return parents


def validate_entry(entry, today):
    """Validate an exact, time-bounded, explained package review."""
    require(isinstance(entry.get("name"), str) and entry["name"], "review requires package name")
    require(isinstance(entry.get("version"), str) and EXACT_VERSION.fullmatch(entry["version"]),
            "review requires exact version, not a range or wildcard")
    require(isinstance(entry.get("reason"), str) and len(entry["reason"].strip()) >= 20,
            "review requires a concrete reason")
    require(isinstance(entry.get("parents"), list) and entry["parents"], "review requires parent constraints")
    for parent in entry["parents"]:
        require(all(isinstance(parent.get(key), str) and parent[key].strip()
                    for key in ("name", "version", "requirement")), "incomplete parent review")
        require(EXACT_VERSION.fullmatch(parent["version"]), "parent requires exact version")
    reviewed = datetime.date.fromisoformat(entry["reviewed_on"])
    expires = datetime.date.fromisoformat(entry["expires_on"])
    require(reviewed <= today <= expires, f"review expired or future-dated: {entry['name']} {entry['version']}")
    require(0 < (expires - reviewed).days <= MAX_REVIEW_DAYS, "review must expire within 90 days")


def reviewed_skips(config, ledger, metadata, today):
    """Generate only active exact exceptions; reject unreviewed parent/version expansion."""
    validate_base(config)
    require(ledger.get("schema_version") == 1, "unsupported review ledger schema")
    packages = metadata["packages"]
    for name in {package["name"] for package in packages if protected(package["name"])}:
        require(len({package["version"] for package in packages if package["name"] == name}) == 1,
                f"protected shared boundary has multiple versions: {name}")
    seen = set()
    for kind, field in (("duplicate", "duplicate_exceptions"), ("advisory", "advisory_exceptions")):
        entries = ledger.get(field)
        require(isinstance(entries, list), f"ledger requires {field}")
        for entry in entries:
            validate_entry(entry, today)
            identity = (kind, entry.get("id"), entry["name"], entry["version"])
            require(identity not in seen, "duplicate ledger review")
            seen.add(identity)
            if kind == "duplicate":
                require(not protected(entry["name"]), f"protected crate cannot receive skip: {entry['name']}")
            else:
                require(REVIEWED_UNMAINTAINED.get(entry.get("id")) == entry["name"],
                        "only specifically reviewed unmaintained advisory IDs may be ignored")
                require(isinstance(entry.get("exposure_review"), str) and len(entry["exposure_review"].strip()) >= 20,
                        "advisory requires runtime input-exposure review")
            active = [package for package in packages
                      if (package["name"], package["version"]) == (entry["name"], entry["version"])]
            if not active:
                continue
            reviewed_parents = {(parent["name"], parent["version"], parent["requirement"])
                                for parent in entry["parents"]}
            for package in active:
                actual_parents = parent_records(package, metadata)
                require(actual_parents, f"resolved parent evidence missing for {entry['name']} {entry['version']}")
                require(actual_parents <= reviewed_parents,
                        f"unreviewed parent path for {entry['name']} {entry['version']}")
            if kind == "duplicate" and entry.get("cargo_deny_skip", True):
                config["bans"]["skip"].append({"name": entry["name"], "version": "=" + entry["version"],
                                               "reason": entry["reason"]})
            elif kind == "advisory":
                # Cargo-deny ignores IDs globally. Additional versions therefore
                # need explicit reviews too, even when one exact version is active.
                versions = {package["version"] for package in packages if package["name"] == entry["name"]}
                approved = {item["version"] for item in entries
                            if item.get("id") == entry["id"] and item.get("name") == entry["name"]}
                require(versions <= approved, f"advisory would cover unreviewed version of {entry['name']}")
                item = {"id": entry["id"], "reason": entry["reason"]}
                if item not in config["advisories"]["ignore"]:
                    config["advisories"]["ignore"].append(item)
    return config


def reviewed_clippy_config(config, ledger, metadata):
    """Allow duplicate names only after every active version has an exact review."""
    require(not config.get("allowed-duplicate-crates"), "Clippy exceptions must come from the reviewed ledger")
    entries = [entry for entry in ledger["duplicate_exceptions"] if entry.get("clippy_allow")]
    names = []
    for name in sorted({entry["name"] for entry in entries}):
        versions = {package["version"] for package in metadata["packages"] if package["name"] == name}
        if len(versions) < 2:
            continue
        approved = {entry["version"] for entry in entries if entry["name"] == name}
        require(versions <= approved, f"unreviewed Clippy duplicate version: {name}")
        names.append(name)
    config["allowed-duplicate-crates"] = names
    return "".join(f"{key} = {toml_value(value)}\n" for key, value in config.items())


def toml_value(value):
    """Serialize only the scalar/list/inline-table types used by deny configuration."""
    if isinstance(value, bool):
        return "true" if value else "false"
    if isinstance(value, (str, int, float)):
        return json.dumps(value, ensure_ascii=True)
    if isinstance(value, list):
        return "[" + ", ".join(toml_value(item) for item in value) + "]"
    if isinstance(value, dict):
        return "{ " + ", ".join(f"{key} = {toml_value(item)}" for key, item in value.items()) + " }"
    raise ValueError("unsupported deny configuration value")


def write_json(path, value):
    """Write generated policy evidence outside tracked source configuration."""
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=2, ensure_ascii=False) + "\n")


def main():
    """Read exact reviews, emit informational licenses, and validate effective policy."""
    parser = argparse.ArgumentParser(description=__doc__)
    for option in ("config", "ledger", "metadata", "effective-config", "license-inventory", "clippy-config", "effective-clippy-dir"):
        parser.add_argument("--" + option, type=Path, required=True)
    args = parser.parse_args()
    try:
        config = tomllib.loads(args.config.read_text())
        ledger = json.loads(args.ledger.read_text())
        metadata = json.loads(args.metadata.read_text())
        write_json(args.license_inventory, {
            "license_gate_enforced": False,
            "purpose": "Informational package license declarations; no allowlist or legal approval claim.",
            "packages": [{key: package.get(key) for key in ("name", "version", "license")}
                         for package in sorted(metadata["packages"], key=lambda item: (item["name"], item["version"]))],
        })
        config = reviewed_skips(config, ledger, metadata, datetime.datetime.now(datetime.timezone.utc).date())
        output = "# Generated from strict policy and expiring exact-version reviews.\n"
        for section, values in config.items():
            output += f"\n[{section}]\n"
            output += "".join(f"{key} = {toml_value(value)}\n" for key, value in values.items())
        tomllib.loads(output)
        args.effective_config.parent.mkdir(parents=True, exist_ok=True)
        args.effective_config.write_text(output)
        clippy = reviewed_clippy_config(tomllib.loads(args.clippy_config.read_text()), ledger, metadata)
        args.effective_clippy_dir.mkdir(parents=True, exist_ok=True)
        (args.effective_clippy_dir / "clippy.toml").write_text(clippy)
    except (OSError, ValueError, KeyError, TypeError) as error:
        print(f"Dependency policy: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
