"""Linux package integration tests with temporary native program fixtures."""

import io
import json
import subprocess
import tarfile
import tempfile
import unittest
from pathlib import Path, PurePosixPath
from subprocess import CompletedProcess
from unittest.mock import patch

from package_linux import _dependency_report, build_package, file_sha256
from package_native_llvm import NATIVE_DOCS
from package_notices import _registry_notices, verify_notices


NATIVE_BUNDLE = {
    "target": "x86_64-unknown-linux-gnu", "wrapper_version": "23.1.0-4",
    "archive_sha256": "a" * 64, "archive_sidecar_sha256": "b" * 64,
    "installed_content_sha256": "c" * 64, "upstream_commit": "d" * 40,
}
FONT_LICENSES = ("IBMPlexMono-OFL.txt", "SpaceGrotesk-OFL.txt")


def _workspace(root):
    """Create a locked Git fixture with selected and dev-only local crates."""
    source = root / "source"
    source.mkdir()
    (source / "Cargo.toml").write_text(
        '[package]\nname = "pancake"\nversion = "0.1.0"\n'
        'edition = "2024"\nlicense = "GPL-3.0-or-later"\n'
        '[workspace]\nresolver = "3"\nmembers = ["crates/cli", "crates/gui"]\n'
        'exclude = ["vendor/*"]\n'
    )
    (source / "src").mkdir()
    (source / "src/lib.rs").touch()
    (source / "LICENSE").write_text("Fixture first-party notice.\n")
    native = source / "notices/llvm"
    native.mkdir(parents=True)
    native_license = native / "LICENSE.TXT"
    native_license.write_text("Fixture native notice.\n")
    (native / "manifest.json").write_text(json.dumps({
        "schema_version": 1, **NATIVE_BUNDLE, "static_llvm_library_count": 1, "notices": {
            "LICENSE.TXT": {"sha256": file_sha256(native_license), "bytes": native_license.stat().st_size},
        },
    }))
    for name in ("fixture-dep", "fixture-dev"):
        directory = source / "vendor" / f"{name}-1.0.0"
        library = directory / "src"
        library.mkdir(parents=True)
        (library / "lib.rs").touch()
        (directory / "Cargo.toml").write_text(
            f'[package]\nname = "{name}"\nversion = "1.0.0"\n'
            'edition = "2024"\nlicense-file = "legal/terms.txt"\n'
        )
        (directory / "legal").mkdir()
        (directory / "legal/terms.txt").write_text(f"Original {name} license.\n")
        (directory / "NOTICE").write_text(f"Original {name} notice.\n")
        (directory / "LICENSES").mkdir()
        (directory / "LICENSES/additional.txt").write_text(f"Original {name} additional license.\n")
    (source / "vendor/upstream-sources.json").write_text(json.dumps({"packages": [{
        "name": "fixture-dep", "version": "1.0.0", "directory": "fixture-dep-1.0.0",
        "archive_sha256": "a" * 64,
    }]}))
    for component in ("cli", "gui"):
        directory = source / "crates" / component
        (directory / "src").mkdir(parents=True)
        (directory / "src/main.rs").write_text("fn main() {}\n")
        manifest = (f'[package]\nname = "pancake-{component}"\nversion = "0.1.0"\n'
                    'edition = "2024"\nlicense = "GPL-3.0-or-later"\n'
                    '[dependencies]\npancake = { path = "../.." }\n')
        if component == "cli":
            manifest += ('fixture-dep = { path = "../../vendor/fixture-dep-1.0.0" }\n'
                         '[dev-dependencies]\nfixture-dev = { path = "../../vendor/fixture-dev-1.0.0" }\n')
        else:
            fonts = directory / "assets/fonts"
            fonts.mkdir(parents=True)
            for name in FONT_LICENSES:
                (fonts / name).write_text(f"Original {name} font license.\n")
        (directory / "Cargo.toml").write_text(manifest)
    subprocess.run(["cargo", "generate-lockfile", "--offline", "--quiet"], cwd=source, check=True)
    subprocess.run(["git", "init", "--quiet"], cwd=source, check=True)
    subprocess.run(["git", "add", "."], cwd=source, check=True)
    subprocess.run(["git", "-c", "user.name=Fixture", "-c", "user.email=test@localhost",
                    "commit", "--quiet", "-m", "Pin fixture source"], cwd=source, check=True)
    revision = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=source, text=True).strip()
    return source, revision


class LinuxPackageTests(unittest.TestCase):
    """Exercise dpkg build/extract and retain the bytes passed in by the caller."""

    def test_package_keeps_native_binary_bytes_and_collects_dependencies(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            release = root / "release"
            release.mkdir()
            source = root / "program.c"
            source.write_text('#include <stdio.h>\nint main(void) { puts("package fixture"); return 0; }\n')
            for name in ("pancake-cli", "pancake-gui"):
                subprocess.run(["cc", "-o", release / name, source], check=True)
            output = root / "package"
            source_root, revision = _workspace(root)
            icon = Path(__file__).resolve().parents[2] / "assets/pancake-stacker-icon.png"
            (root / "assets").mkdir()
            (root / "assets" / icon.name).write_bytes(icon.read_bytes())
            lock_digest = file_sha256(source_root / "Cargo.lock")
            native_receipt = root / "native-build.json"
            native_receipt.write_text(json.dumps({
                "schema_version": 1, **NATIVE_BUNDLE, "source_commit": revision, "cargo_lock_sha256": lock_digest,
                "binaries": {path.name: {"sha256": file_sha256(path), "bytes": path.stat().st_size}
                             for path in release.iterdir()},
                "link_output_sha256": "e" * 64,
                "static_llvm_inputs": [{"name": "libLLVMCore.a", "sha256": "f" * 64, "bytes": 12}],
                "native_rlib": {"rlib_sha256": "a" * 64, "rlib_bytes": 30,
                                "matched_native_objects": 2, "distinct_native_object_digests": 2},
            }))
            build_package(release, output, "0.1.0", "Pancake fixture <test@localhost>", revision, lock_digest,
                          source_root=source_root, native_llvm_receipt=native_receipt)
            receipt = json.loads((output / "package-receipt.json").read_text())
            self.assertEqual(receipt["source_commit"], revision)
            artifact = output / receipt["archive"]["file"]
            self.assertEqual(receipt["archive"]["sha256"], file_sha256(artifact))
            members = subprocess.check_output(["ar", "t", artifact], text=True).splitlines()
            self.assertEqual(members, ["debian-binary", "control.tar.zst", "data.tar.zst"])
            subprocess.run(["dpkg-deb", "--extract", artifact, root / "unpack"], check=True)
            share = root / "unpack/usr/share"
            launcher = (share / "applications/pancake.desktop").read_text().splitlines()
            self.assertIn("Name=Pancake Stacker", launcher)
            self.assertIn("Icon=pancake", launcher)
            self.assertEqual(file_sha256(share / "pixmaps/pancake.png"), file_sha256(icon))
            for name in ("pancake-cli", "pancake-gui"):
                self.assertEqual(file_sha256(release / name), file_sha256(root / "unpack/usr/bin" / name))
            launched = subprocess.check_output([root / "unpack/usr/bin/pancake-cli"], text=True, timeout=5)
            self.assertEqual(launched, "package fixture\n")
            self.assertIn("libc6", receipt["depends"])
            self.assertIn("libvulkan1", receipt["depends"])
            inventory = root / "unpack/usr/share/doc/pancake/dependency-notices/inventory.json"
            self.assertTrue(inventory.is_file(), "Debian archive dropped the dependency notice inventory")
            self.assertEqual(receipt["dependency_notices"]["sha256"], file_sha256(inventory))
            manifest = json.loads(inventory.read_text())
            self.assertEqual(manifest["source_commit"], revision)
            self.assertEqual(manifest["cargo_lock_sha256"], lock_digest)
            items = {package["name"]: package for package in manifest["packages"]}
            self.assertEqual(set(items), {"pancake", "pancake-cli", "pancake-gui", "fixture-dep"})
            shared = items["pancake"]["notices"]
            self.assertEqual(len(shared), 1)
            self.assertEqual((inventory.parent / shared[0]["file"]).read_bytes(),
                             (source_root / "LICENSE").read_bytes())
            self.assertEqual(items["pancake-cli"]["status"], "covered_by_shared_project_notice")
            self.assertEqual(items["pancake-cli"]["shared_project_license"], shared[0])
            self.assertFalse(items["pancake-cli"]["notices"])
            self.assertEqual(items["pancake-gui"]["status"], "present")
            self.assertEqual(items["pancake-gui"]["shared_project_license"], shared[0])
            gui_notices = {item["file"] for item in items["pancake-gui"]["notices"]}
            for name in FONT_LICENSES:
                relative = Path("assets/fonts") / name
                packed_path = Path("pancake-gui-0.1.0") / relative
                self.assertIn(packed_path.as_posix(), gui_notices)
                self.assertEqual((inventory.parent / packed_path).read_bytes(),
                                 (source_root / "crates/gui" / relative).read_bytes())
            self.assertEqual(set(receipt["native_llvm"]), {
                (NATIVE_DOCS / name).as_posix() for name in ("LICENSE.TXT", "manifest.json", "build-provenance.json")
            })
            for path, expected in receipt["native_llvm"].items():
                extracted = root / "unpack" / path
                self.assertEqual(file_sha256(extracted), expected["sha256"])
                self.assertEqual(extracted.stat().st_size, expected["bytes"])
            packed_native_receipt = root / "unpack" / NATIVE_DOCS / "build-provenance.json"
            self.assertEqual(json.loads(packed_native_receipt.read_text()), json.loads(native_receipt.read_text()))
            self.assertEqual(items["fixture-dep"]["provenance"]["commit"], revision)
            self.assertEqual(items["fixture-dep"]["provenance"]["upstream_patch"]["archive_sha256"], "a" * 64)
            for name in ("NOTICE", "legal/terms.txt", "LICENSES/additional.txt"):
                relative = Path("fixture-dep-1.0.0") / name
                self.assertEqual((inventory.parent / relative).read_bytes(),
                                 (source_root / "vendor" / relative).read_bytes())
            (inventory.parent / "fixture-dep-1.0.0/NOTICE").write_text("tampered")
            with self.assertRaisesRegex(ValueError, "Packaged dependency notice changed"):
                verify_notices(root / "unpack", receipt["dependency_notices"])
            launcher = root / "unpack/usr/share/applications/pancake.desktop"
            self.assertIn("Exec=pancake-gui %F", launcher.read_text())
            self.assertIn("Shared library: [libc.so.6]", receipt["binaries"]["pancake-cli"]["dynamic"])

    def test_direct_version_constraint_replaces_duplicate_runtime_dependency(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            versioned = "libfontconfig1 (>= 2.12.6)"
            with patch("package_linux._run", return_value=CompletedProcess(
                args=[], returncode=0, stdout=f"shlibs:Depends=libc6 (>= 2.39), {versioned}\n", stderr=""
            )):
                dependencies, _ = _dependency_report(root, (), "Package: pancake\n")
            requirements = dependencies.split(", ")
            self.assertIn(versioned, requirements)
            self.assertNotIn("libfontconfig1", requirements)

    def test_registry_notices_use_the_locked_archive(self):
        with tempfile.TemporaryDirectory() as directory:
            registry = Path(directory) / "registry"
            index = "index-fixture"
            source = registry / "src" / index / "fixture-registry-1.2.3"
            source.mkdir(parents=True)
            archive = registry / "cache" / index / "fixture-registry-1.2.3.crate"
            archive.parent.mkdir(parents=True)
            entries = {
                "LICENSES/LicenseRef-Fictional.txt": b"upstream declared notice",
                "source-license/terms.txt": b"Cargo license-file",
                "../NOTICE": b"out of crate scope",
                "src/lib.rs": b"not notice text",
            }
            with tarfile.open(archive, "w:gz") as output:
                for name, content in entries.items():
                    member = tarfile.TarInfo(f"fixture-registry-1.2.3/{name}")
                    member.size = len(content)
                    output.addfile(member, io.BytesIO(content))
            package = {"name": "fixture-registry", "version": "1.2.3",
                       "manifest_path": str(source / "Cargo.toml"), "license_file": "source-license/terms.txt"}
            checksum = file_sha256(archive)
            provenance, status, notices = _registry_notices(package, {"checksum": checksum})
            self.assertEqual(status, "present")
            self.assertEqual(provenance["locked_archive_sha256"], checksum)
            self.assertEqual(notices, {PurePosixPath(name): entries[name] for name in
                                       ("LICENSES/LicenseRef-Fictional.txt", "source-license/terms.txt")})
            _, status, notices = _registry_notices(package, {"checksum": "a" * 64})
            self.assertEqual(status, "archive_checksum_mismatch")
            self.assertEqual(notices, {})

    def test_a_shell_script_cannot_be_packaged_as_a_native_release_binary(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            release = root / "release"
            release.mkdir()
            for name in ("pancake-cli", "pancake-gui"):
                script = release / name
                script.write_text("#!/bin/sh\nexit 0\n")
                script.chmod(0o755)
            output = root / "package"
            with self.assertRaisesRegex(ValueError, "native Linux executable"):
                build_package(release, output, "0.1.0", "Pancake fixture <test@localhost>", "a" * 40, "b" * 64,
                              source_root=root)
            self.assertFalse(output.exists())


if __name__ == "__main__":
    unittest.main()
