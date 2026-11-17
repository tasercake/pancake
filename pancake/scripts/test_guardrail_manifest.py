"""Keep real-tool guardrail fixtures independent from application dependencies."""

import json
from pathlib import Path
import subprocess
import sys
import tempfile
import tomllib
import unittest


class GuardrailManifestTests(unittest.TestCase):
    """Preserve exact lint rules without copying production feature/patch graphs."""

    def test_fixture_retains_lints_without_resolving_missing_production_paths(self):
        source = Path(__file__).resolve().parents[1] / "Cargo.toml"
        original = tomllib.loads(source.read_text())
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            input_manifest = root / "production.toml"
            # Unavailable production paths must not become fixture prerequisites.
            input_manifest.write_text(source.read_text() + '\n[dependencies.guardrail-missing]\npath = "vendor/missing"\n'
                                      '[patch.crates-io.guardrail-other]\npath = "vendor/other"\n')
            fixture = root / "fixture"
            (fixture / "src").mkdir(parents=True)
            (fixture / "src/lib.rs").write_text("//! Fixture crate.\n")
            result = subprocess.run([
                sys.executable, str(Path(__file__).with_name("guardrail_manifest.py")),
                str(input_manifest), str(fixture / "Cargo.toml"),
            ], capture_output=True, text=True, check=False)
            self.assertEqual(result.returncode, 0, result.stderr)
            generated = tomllib.loads((fixture / "Cargo.toml").read_text())
            self.assertEqual(generated["workspace"]["lints"], original["workspace"]["lints"])
            self.assertNotIn("dependencies", generated)
            self.assertNotIn("patch", generated)
            metadata = subprocess.run([
                "cargo", "metadata", "--offline", "--format-version", "1",
                "--manifest-path", str(fixture / "Cargo.toml"),
            ], capture_output=True, text=True, check=False)
            self.assertEqual(metadata.returncode, 0, metadata.stderr)
            self.assertEqual(len(json.loads(metadata.stdout)["packages"]), 1)


if __name__ == "__main__":
    unittest.main()
