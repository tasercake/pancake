"""Compile contracts for the pinned CubeCL function expand_only patch."""

import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import tomllib
import unittest


ROOT = Path(__file__).resolve().parents[2]


class ExpandOnlyContracts(unittest.TestCase):
    """Keep checked launch generation while removing only the host DSL stub."""

    @classmethod
    def setUpClass(cls):
        """Resolve offline with the HIP index entry absent, even on warm machines."""
        super().setUpClass()
        build = ROOT / "target/macro-contract"
        build.mkdir(parents=True, exist_ok=True)
        source_home = Path(os.environ.get("CARGO_HOME", Path.home() / ".cargo"))
        cold_home = Path(cls.enterClassContext(tempfile.TemporaryDirectory(
            prefix="cold-cargo-", dir=build)))
        registry = cold_home / "registry"
        registry.mkdir()
        shutil.copytree(source_home / "registry/index", registry / "index",
                        ignore=shutil.ignore_patterns("cubecl-hip"))
        # Offline checks neither download nor garbage-collect the shared caches.
        for name in ("cache", "src"):
            (registry / name).symlink_to((source_home / "registry" / name).resolve(),
                                       target_is_directory=True)
        if list((registry / "index").rglob("cubecl-hip")):
            raise AssertionError("cold index still contains cubecl-hip")
        cls.cargo_environment = dict(os.environ, CARGO_HOME=str(cold_home))

    def check(self, name, annotation, statement):
        directory = ROOT / "target" / "macro-contract" / name
        directory.mkdir(parents=True, exist_ok=True)
        (directory / "src").mkdir(exist_ok=True)
        patches = tomllib.loads((ROOT / "Cargo.toml").read_text())["patch"]["crates-io"]
        patch_lines = []
        for package, specification in patches.items():
            specification = dict(specification)
            if "path" in specification:
                specification["path"] = str((ROOT / specification["path"]).resolve())
            fields = ", ".join(f"{key} = {json.dumps(value, ensure_ascii=False)}"
                               for key, value in specification.items())
            patch_lines.append(f"{json.dumps(package, ensure_ascii=False)} = {{ {fields} }}\n")
        (directory / "Cargo.toml").write_text(
            '[package]\nname = "macro-contract-' + name + '"\nversion = "0.0.0"\nedition = "2024"\n'
            '[workspace]\n[dependencies]\n'
            'cubecl = { version = "=0.11.4", default-features = false, features = ["std"] }\n'
            '[patch.crates-io]\n' + "".join(patch_lines)
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
            cwd=ROOT, env=self.cargo_environment, capture_output=True, text=True, check=False,
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
