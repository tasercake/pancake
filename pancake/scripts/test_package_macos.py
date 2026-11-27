"""Portable tests of the private Apple package staging and verification contract."""

import json
import plistlib
import subprocess
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from package_macos import _bundle_metadata, build_package, file_sha256


IDENTIFIER = "test.example.pancake"
SYSTEM_DEPENDENCY = "/usr/lib/libSystem.B.dylib"
BUILD = (
    "cargo", "+1.100.0", "build", "--locked", "--release", "--workspace", "--bins",
    "--target", "aarch64-apple-darwin", "--features", "cubecl/metal-native",
)
BINARIES = ("pancake-cli", "pancake-gui")
FONT_LICENSES = ("IBMPlexMono-OFL.txt", "SpaceGrotesk-OFL.txt")


class MacPackageTests(unittest.TestCase):
    """Exercise real staging and Git provenance, with an explicit Apple-tool fixture."""

    def setUp(self):
        self.root = Path(self.enterContext(tempfile.TemporaryDirectory()))
        self.source = self.root / "source"
        self.output = self.root / "private-package"
        self.source.mkdir()
        (self.source / "Cargo.lock").write_text("# fixture locked graph\n")
        (self.source / "rust-toolchain.toml").write_text('[toolchain]\nchannel = "1.100.0"\n')
        for name in BINARIES:
            crate = self.source / "crates" / name
            crate.mkdir(parents=True)
            (crate / "Cargo.toml").write_text(f'[package]\nname = "{name}"\nversion = "0.1.0"\n')
        fonts = self.source / "crates/pancake-gui/assets/fonts"
        fonts.mkdir(parents=True)
        for name in FONT_LICENSES:
            (fonts / name).write_text(f"Original {name} font license.\n")
        self.git("init", "--quiet")
        self.git("add", ".")
        self.git("-c", "user.name=Fixture", "-c", "user.email=test@localhost",
                 "-c", "commit.gpgSign=false", "commit", "--quiet", "-m", "fixture")
        self.commands = []
        self.architecture = "arm64"
        self.dependency = SYSTEM_DEPENDENCY
        self.failure = ""
        self.identity = IDENTIFIER
        self.enterContext(patch("package_macos.platform.system", return_value="Darwin"))
        self.enterContext(patch("package_macos.platform.machine", return_value="arm64"))
        self.enterContext(patch("package_macos._apple", side_effect=self.apple))

    def test_rc_uses_numeric_bundle_version(self):
        for name in BINARIES:
            manifest = self.source / "crates" / name / "Cargo.toml"
            manifest.write_text(manifest.read_text().replace("0.1.0", "0.2.0-rc.1"))
        metadata = _bundle_metadata(self.source, IDENTIFIER, "17")
        self.assertEqual(metadata["CFBundleShortVersionString"], "0.2.0")
        self.assertEqual(metadata["CFBundleVersion"], "17")

    def git(self, *command):
        return subprocess.check_output(["git", *command], cwd=self.source, text=True).strip()

    def apple(self, source, *command):
        """Supply representative native command results while exercising staging on disk."""
        self.assertEqual(source, self.source)
        argv = tuple(map(str, command))
        self.commands.append(argv)
        if argv[:len(BUILD)] == BUILD:
            self.assertEqual(argv[-2], "--target-dir")
            release = Path(argv[-1]) / "aarch64-apple-darwin" / "release"
            release.mkdir(parents=True, exist_ok=True)
            for name in BINARIES:
                binary = release / name
                if not binary.exists():
                    binary.write_bytes(name.encode())
                    binary.chmod(0o755)
            if self.failure == "collision":
                self.output.mkdir()
                (self.output / "untouched").write_text("user file")
            return "locked build output\n"
        if argv[0] in {"cargo", "rustc", "sw_vers"}:
            return f"{argv[0]} fixture version\n"
        if argv == ("xcode-select", "--print-path"):
            return "/Applications/Xcode.app/Contents/Developer\n"
        if argv[:4] == ("xcrun", "--no-cache", "--sdk", "macosx"):
            return {
                ("--show-sdk-version",): "26.4\n",
                ("--show-sdk-path",): "/test/MacOSX.sdk\n",
                ("clang", "--version"): "Apple clang fixture version\n",
                ("--find", "ld"): "/test/apple/ld\n",
            }[argv[4:]]
        if argv[:2] == ("lipo", "-archs"):
            return self.architecture + "\n"
        if argv[:2] == ("otool", "-L"):
            return f"{argv[-1]}:\n\t{self.dependency} (compatibility version 1.0.0, current version 1.0.0)\n"
        if argv[:2] == ("plutil", "-lint"):
            with Path(argv[-1]).open("rb") as plist:
                self.assertEqual(plistlib.load(plist)["CFBundleIdentifier"], IDENTIFIER)
            return "Info.plist: OK\n"
        if argv[:2] == ("codesign", "--force"):
            signed = Path(argv[-1])
            if signed.suffix == ".app":
                for name in FONT_LICENSES:
                    self.assertEqual((signed / "Contents/Resources/font-notices" / name).read_bytes(),
                                     (self.source / "crates/pancake-gui/assets/fonts" / name).read_bytes())
                signatures = signed / "Contents" / "_CodeSignature"
                signatures.mkdir()
                (signatures / "CodeResources").write_bytes(b"resources")
                if self.failure == "plist":
                    (signed / "Contents" / "Info.plist").write_bytes(plistlib.dumps({}))
                signed = signed / "Contents" / "MacOS" / "pancake-gui"
            with signed.open("ab") as binary:
                binary.write(b" signed")
            return ""
        if argv[:3] == ("codesign", "--verify", "--strict"):
            if self.failure == "signature":
                raise ValueError("invalid signature")
            return "verified\n"
        if argv[:2] == ("codesign", "--display"):
            identity = self.identity if argv[-1].endswith(".app") else f"{IDENTIFIER}.cli"
            return f"Identifier={identity}\nSignature=adhoc\n"
        self.fail(f"Unexpected native command {argv}")

    def package(self):
        return build_package(self.source, self.output, IDENTIFIER, "17")

    def test_stages_matching_cli_and_audited_app(self):
        with patch.dict("os.environ", {
            "MACOSX_DEPLOYMENT_TARGET": "15.0",
            "CARGO_TARGET_AARCH64_APPLE_DARWIN_LINKER": "/test/custom-linker",
            "UNRELATED_FIXTURE": "discard this value",
        }):
            self.package()
        app = self.output / "Pancake.app"
        plist_path = app / "Contents" / "Info.plist"
        plist = plistlib.loads(plist_path.read_bytes())
        self.assertEqual(plist["CFBundleExecutable"], "pancake-gui")
        self.assertEqual(plist["CFBundleIdentifier"], IDENTIFIER)
        self.assertEqual(plist["CFBundlePackageType"], "APPL")
        self.assertEqual(plist["CFBundleShortVersionString"], "0.1.0")
        self.assertEqual(plist["CFBundleVersion"], "17")
        self.assertNotIn("LSMinimumSystemVersion", plist)
        self.assertNotIn("NSAppleEventsUsageDescription", plist)
        receipt = json.loads((self.output / "package-receipt.json").read_bytes())
        self.assertEqual(receipt["source_commit"], self.git("rev-parse", "HEAD"))
        self.assertEqual(receipt["source_tree"], self.git("rev-parse", "HEAD^{tree}"))
        self.assertEqual(receipt["cargo_lock_sha256"], file_sha256(self.source / "Cargo.lock"))
        self.assertEqual(tuple(receipt["build_command"][:len(BUILD)]), BUILD)
        self.assertEqual(receipt["bundle_identifier"], IDENTIFIER)
        self.assertEqual(receipt["signing_mode"], "ad-hoc")
        tools = receipt["tool_versions"]
        self.assertEqual(tools["developer_directory"], "/Applications/Xcode.app/Contents/Developer")
        self.assertEqual(tools["sdk_version"], "26.4")
        self.assertEqual(tools["sdk_path"], "/test/MacOSX.sdk")
        self.assertEqual(tools["clang"], "Apple clang fixture version")
        self.assertEqual(tools["sdk_linker_path"], "/test/apple/ld")
        overrides = receipt["build_overrides"]
        self.assertEqual(overrides["MACOSX_DEPLOYMENT_TARGET"], "15.0")
        self.assertEqual(overrides["CARGO_TARGET_AARCH64_APPLE_DARWIN_LINKER"], "/test/custom-linker")
        self.assertNotIn("UNRELATED_FIXTURE", overrides)
        self.assertEqual(receipt["files_sha256"]["Pancake.app/Contents/Info.plist"], file_sha256(plist_path))
        self.assertIn("Pancake.app/Contents/_CodeSignature/CodeResources", receipt["files_sha256"])
        for name in FONT_LICENSES:
            relative = f"Pancake.app/Contents/Resources/font-notices/{name}"
            self.assertEqual(receipt["files_sha256"][relative], file_sha256(self.output / relative))
        for name, path in zip(BINARIES, ("pancake-cli", "Pancake.app/Contents/MacOS/pancake-gui"), strict=True):
            binary = self.output / path
            self.assertEqual(binary.read_bytes(), f"{name} signed".encode())
            self.assertEqual(binary.stat().st_mode & 0o777, 0o755)
            self.assertEqual(receipt["executables"][name]["architecture"], "arm64")
            self.assertEqual(receipt["executables"][name]["dependencies"], [SYSTEM_DEPENDENCY])
            self.assertNotEqual(receipt["executables"][name]["source_sha256"], file_sha256(binary))
            self.assertEqual(receipt["files_sha256"][path], file_sha256(binary))
        self.assertEqual((self.output / "build.log").read_text(), "locked build output\n")
        self.assertEqual(sum(cmd[:3] == ("codesign", "--verify", "--strict") for cmd in self.commands), 2)

    def test_reuses_a_cargo_target_without_signing_its_binaries(self):
        target = self.root / "cargo-target"
        release = target / "aarch64-apple-darwin" / "release"
        outputs = (self.output, self.root / "another-package")
        for output in outputs:
            build_package(self.source, output, IDENTIFIER, "17", target_dir=target)
            receipt = json.loads((output / "package-receipt.json").read_bytes())
            self.assertEqual(receipt["build_command"], [*BUILD, "--target-dir", str(target)])
            for name, item in receipt["executables"].items():
                binary = release / name
                self.assertEqual(binary.read_bytes(), name.encode())
                self.assertEqual(item["source_sha256"], file_sha256(binary))
                self.assertEqual((output / item["file"]).read_bytes(), f"{name} signed".encode())
        self.assertEqual(set(self.root.iterdir()), {self.source, target, *outputs})

    def test_refuses_a_target_within_the_package_output(self):
        with self.assertRaisesRegex(ValueError, "outside the package output"):
            build_package(self.source, self.output, IDENTIFIER, "17", target_dir=self.output / "build")
        self.assertFalse(self.output.exists())
        self.assertFalse(self.commands)

    def test_fails_closed_on_wrong_architecture_dependency_and_signature(self):
        for field, value, message in (
            ("architecture", "x86_64 arm64", "arm64"),
            ("dependency", "@rpath/libMissing.dylib", "dependency"),
            ("dependency", "/usr/lib/../local/libBrew.dylib", "dependency"),
            ("failure", "signature", "invalid signature"),
            ("failure", "plist", "Info.plist changed"),
            ("identity", "test.example.different", "signature identity"),
        ):
            with self.subTest(field=field, value=value):
                before = getattr(self, field)
                setattr(self, field, value)
                try:
                    with self.assertRaisesRegex(ValueError, message):
                        self.package()
                finally:
                    setattr(self, field, before)
                self.assertFalse(self.output.exists())
                self.assertEqual(set(self.root.iterdir()), {self.source})

    def test_refuses_source_copy_mismatch_before_signing(self):
        def corrupt_copy(source, destination):
            destination.write_bytes(source.read_bytes() + b" changed")

        with patch("package_macos.shutil.copyfile", side_effect=corrupt_copy):
            with self.assertRaisesRegex(ValueError, "Release binary changed"):
                self.package()
        self.assertFalse(self.output.exists())
        self.assertTrue(all(command[0] != "codesign" for command in self.commands))

    def test_leaves_an_existing_or_concurrently_created_output_untouched(self):
        self.output.mkdir()
        original = self.output / "untouched"
        original.write_text("user file")
        with self.assertRaises(FileExistsError):
            self.package()
        self.assertFalse(self.commands)
        original.unlink()
        self.output.rmdir()
        self.failure = "collision"
        with self.assertRaises(FileExistsError):
            self.package()
        self.assertEqual(original.read_text(), "user file")
        self.assertEqual(list(self.output.iterdir()), [original])

    def test_accepts_a_new_nonignored_output_inside_the_source_worktree(self):
        self.output = self.source / "local-packages" / "private-package"
        self.package()
        self.assertTrue((self.output / "package-receipt.json").is_file())
        self.assertEqual(list(self.output.parent.iterdir()), [self.output])

    def test_rejects_dirty_source_invalid_identity_and_non_apple_hosts(self):
        with patch("package_macos.platform.system", return_value="Linux"):
            with self.assertRaisesRegex(ValueError, "Apple Silicon"):
                self.package()
        with self.assertRaisesRegex(ValueError, "bundle identifier"):
            build_package(self.source, self.output, "../invalid", "17")
        with self.assertRaisesRegex(ValueError, "build number"):
            build_package(self.source, self.output, IDENTIFIER, "0")
        (self.source / "Cargo.lock").write_text("changed")
        with self.assertRaisesRegex(ValueError, "clean"):
            self.package()
        self.assertFalse(self.commands)
        self.assertFalse(self.output.exists())


if __name__ == "__main__":
    unittest.main()
