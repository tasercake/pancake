"""Compile contracts for the pinned CubeCL function expand_only patch."""

from pathlib import Path
import shutil
import subprocess
import unittest


ROOT = Path(__file__).resolve().parents[2]


class ExpandOnlyContracts(unittest.TestCase):
    """Keep checked launch generation while removing only the host DSL stub."""

    def check(self, name, annotation, statement):
        directory = ROOT / "target" / "macro-contract" / name
        directory.mkdir(parents=True, exist_ok=True)
        (directory / "src").mkdir(exist_ok=True)
        (directory / "Cargo.toml").write_text(
            '[package]\nname = "macro-contract-' + name + '"\nversion = "0.0.0"\nedition = "2024"\n'
            '[workspace]\n[dependencies]\n'
            'cubecl = { version = "=0.11.4", default-features = false, features = ["std"] }\n'
            '[patch.crates-io]\n'
            'cubecl-core = { path = "' + str(ROOT / 'vendor/cubecl-core-0.11.4') + '" }\n'
            'cubecl-macros = { path = "' + str(ROOT / 'vendor/cubecl-macros-0.11.4') + '" }\n'
        )
        (directory / "src/main.rs").write_text(
            'use cubecl::prelude::*;\n'
            '#[cube(' + annotation + ')]\npub fn kernel() {}\n'
            'fn main() { ' + statement + ' }\n'
        )
        shutil.copyfile(ROOT / "Cargo.lock", directory / "Cargo.lock")
        return subprocess.run(
            ["cargo", "check", "--offline", "--manifest-path", str(directory / "Cargo.toml"),
             "--target-dir", str(ROOT / "target/macro-contract/build")],
            cwd=ROOT, capture_output=True, text=True, check=False,
        )

    def test_expand_only_has_no_original_function_symbol(self):
        result = self.check("absent", "launch, expand_only", "let _host: fn() = kernel;")
        self.assertNotEqual(result.returncode, 0, result.stderr)
        self.assertIn("error[E0423]: cannot find value `kernel`", result.stderr)
        self.assertIn("a module named `kernel` exists in another namespace", result.stderr)

    def test_expand_only_retains_safe_checked_launch(self):
        result = self.check(
            "checked", "launch, expand_only",
            "let _launch: fn(&Client, CubeCount, CubeDim) = kernel::launch;",
        )
        self.assertEqual(result.returncode, 0, result.stderr)

    def test_default_macro_retains_original_function(self):
        result = self.check("default", "launch", "let _host: fn() = kernel;")
        self.assertEqual(result.returncode, 0, result.stderr)


if __name__ == "__main__":
    unittest.main()
