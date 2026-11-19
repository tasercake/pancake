"""Fast artifact-verification tests with small local GNU archives."""

import hashlib
import json
import tempfile
import unittest
from pathlib import Path

from package_native_llvm import (
    NATIVE_DOCS,
    confirm_archive_members,
    stage_native_llvm_notices,
    static_llvm_inputs,
    verify_native_llvm_notices,
)
from package_notices import bind_shared_project_license


def write_ar(path, files):
    """Write a GNU-style regular archive for the read-only verifier."""
    with path.open("wb") as output:
        output.write(b"!<arch>\n")
        for name, data in files:
            header = f"{name + '/':16}{0:12}{0:6}{0:6}{'100644':8}{len(data):10}`\n"
            output.write(header.encode("ascii") + data)
            if len(data) % 2:
                output.write(b"\n")


class PackageNativeLlvmTests(unittest.TestCase):
    def test_link_search_selects_installed_static_objects(self):
        with tempfile.TemporaryDirectory() as location:
            root = Path(location)
            libdir = root / "bundle/lib"
            libdir.mkdir(parents=True)
            output = root / "output"
            library = libdir / "libLLVMSupport.a"
            write_ar(library, [("support.o", b"verified")])
            output.write_text(
                f"cargo:rustc-link-search=native={libdir}\n"
                "cargo:rustc-link-lib=static=LLVMSupport\n"
                "cargo:rustc-link-lib=dylib=m\n"
            )
            self.assertEqual(static_llvm_inputs(output, libdir), [library])

            output.write_text(
                f"cargo:rustc-link-search=native={root / 'stale/lib'}\n"
                "cargo:rustc-link-lib=static=LLVMSupport\n"
            )
            with self.assertRaisesRegex(ValueError, "search path"):
                static_llvm_inputs(output, libdir)

    def test_duplicate_or_dynamic_llvm_directives_fail(self):
        with tempfile.TemporaryDirectory() as location:
            root = Path(location)
            root_lib = root / "lib"
            root_lib.mkdir()
            write_ar(root_lib / "libLLVMCore.a", [("core.o", b"exact")])
            output = root / "output"
            for directives in (
                "cargo:rustc-link-lib=static=LLVMCore\ncargo:rustc-link-lib=static=LLVMCore\n",
                "cargo:rustc-link-lib=static=LLVMCore\ncargo:rustc-link-lib=dylib=LLVMCore\n",
                "cargo:rustc-link-lib=static=LLVMCore\ncargo:rustc-link-lib=LLVMCore\n",
            ):
                output.write_text(f"cargo:rustc-link-search=native={root_lib}\n" + directives)
                with self.assertRaises(ValueError):
                    static_llvm_inputs(output, root_lib)

    def test_rlib_identity_checks_each_native_member_and_multiplicity(self):
        with tempfile.TemporaryDirectory() as location:
            root = Path(location)
            library = root / "libLLVMCore.a"
            rlib = root / "libcubecl_llvm.rlib"
            write_ar(library, [("core.o", b"exact"), ("core2.o", b"exact")])
            write_ar(rlib, [("core.o", b"exact"), ("core2.o", b"exact"), ("lib.rmeta", b"Rust")])
            result = confirm_archive_members([library], rlib)
            self.assertEqual(result["matched_native_objects"], 2)
            self.assertEqual(result["rlib_sha256"], hashlib.sha256(rlib.read_bytes()).hexdigest())
            write_ar(rlib, [("core.o", b"exact"), ("core2.o", b"stale")])
            with self.assertRaisesRegex(ValueError, "native LLVM objects"):
                confirm_archive_members([library], rlib)
            rlib.write_bytes(b"!<arch>\ntruncated")
            with self.assertRaisesRegex(ValueError, "Malformed"):
                confirm_archive_members([library], rlib)

    def test_project_licenses_share_the_single_committed_package_copy(self):
        shared = {"file": "pancake-0.1.0/LICENSE", "bytes": 35149, "sha256": "a" * 64}
        rows = [
            {"name": name, "source": None, "declared_license": "GPL-3.0-or-later",
             "notices": [shared] if name == "pancake" else [],
             "status": "present" if name == "pancake" else "no_notice_in_source"}
            for name in ("pancake", "pancake-cli", "pancake-gui")
        ]
        bind_shared_project_license(rows)
        self.assertEqual(sum(len(row["notices"]) for row in rows), 1)
        for row in rows[1:]:
            self.assertEqual(row["shared_project_license"], shared)
            self.assertEqual(row["status"], "covered_by_shared_project_notice")

    def test_staging_requires_matching_well_formed_release_evidence(self):
        revision = "a" * 40
        lock = "b" * 64
        binaries = {name: {"sha256": "c" * 64, "bytes": 10} for name in ("pancake-cli", "pancake-gui")}
        with tempfile.TemporaryDirectory() as location:
            root = Path(location)
            source = root / "source"
            source.mkdir()
            text = b"Fixture notice\n"
            (source / "LICENSE.TXT").write_bytes(text)
            identity = {
                "target": "x86_64-unknown-linux-gnu", "wrapper_version": "23.1.0-4",
                "archive_sha256": "a" * 64, "archive_sidecar_sha256": "b" * 64,
                "installed_content_sha256": "c" * 64, "upstream_commit": "d" * 40,
            }
            pins = {"schema_version": 1, **identity, "static_llvm_library_count": 1, "notices": {
                "LICENSE.TXT": {"bytes": len(text), "sha256": hashlib.sha256(text).hexdigest()},
            }}
            (source / "manifest.json").write_text(json.dumps(pins))
            record = {
                "schema_version": 1, **identity, "source_commit": revision, "cargo_lock_sha256": lock,
                "binaries": binaries, "link_output_sha256": "e" * 64,
                "static_llvm_inputs": [{"name": "libLLVMCore.a", "sha256": "f" * 64, "bytes": 12}],
                "native_rlib": {"rlib_sha256": "a" * 64, "rlib_bytes": 30,
                                "matched_native_objects": 2, "distinct_native_object_digests": 2},
            }
            receipt = root / "build.json"
            receipt.write_text(json.dumps(record))
            stale = {**binaries, "pancake-cli": {"sha256": "f" * 64, "bytes": 10}}
            with self.assertRaisesRegex(ValueError, "identities differ"):
                stage_native_llvm_notices(root / "payload", source, receipt, revision, lock, stale)
            invalid = {**record, "static_llvm_inputs": [{**record["static_llvm_inputs"][0], "sha256": []}]}
            for rejected in (invalid, {**record, "unrecognized_field": "unreviewed text"}):
                receipt.write_text(json.dumps(rejected))
                with self.assertRaisesRegex(ValueError, "[Mm]alformed"):
                    stage_native_llvm_notices(root / "payload", source, receipt, revision, lock, binaries)
            self.assertFalse((root / "payload").exists())

            receipt.write_text(json.dumps(record))
            files = stage_native_llvm_notices(root / "payload", source, receipt, revision, lock, binaries)
            self.assertEqual(len(files), 3)
            verify_native_llvm_notices(root / "payload", files)
            (root / "payload" / NATIVE_DOCS / "LICENSE.TXT").write_bytes(b"Wrong")
            with self.assertRaisesRegex(ValueError, "changed"):
                verify_native_llvm_notices(root / "payload", files)
