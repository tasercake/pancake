"""Linux package integration tests with temporary native program fixtures."""

import json
import subprocess
import tempfile
import unittest
from pathlib import Path

from package_linux import build_package, file_sha256


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
            revision = "a" * 40
            build_package(release, output, "0.1.0", "Pancake fixture <test@localhost>", revision, "b" * 64)
            receipt = json.loads((output / "package-receipt.json").read_text())
            self.assertEqual(receipt["source_commit"], revision)
            artifact = output / receipt["archive"]["file"]
            self.assertEqual(receipt["archive"]["sha256"], file_sha256(artifact))
            subprocess.run(["dpkg-deb", "--extract", artifact, root / "unpack"], check=True)
            for name in ("pancake-cli", "pancake-gui"):
                self.assertEqual(file_sha256(release / name), file_sha256(root / "unpack/usr/bin" / name))
            self.assertIn("libc6", receipt["depends"])
            self.assertIn("libvulkan1", receipt["depends"])
            launcher = root / "unpack/usr/share/applications/pancake.desktop"
            self.assertIn("Exec=pancake-gui %F", launcher.read_text())
            self.assertIn("Shared library: [libc.so.6]", receipt["binaries"]["pancake-cli"]["dynamic"])

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
                build_package(release, output, "0.1.0", "Pancake fixture <test@localhost>", "a" * 40, "b" * 64)
            self.assertFalse(output.exists())


if __name__ == "__main__":
    unittest.main()
