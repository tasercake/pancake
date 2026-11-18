"""Verify that release frontends and the compute core have separate Cargo graphs."""

from pathlib import Path
import subprocess
import unittest


ROOT = Path(__file__).resolve().parents[1]
WINDOW_DEPENDENCIES = {"accesskit", "slint", "tiny-xlib", "winit", "x11", "xcb"}
# wgpu-hal can load Wayland symbols dynamically for GLES presentation.
# The headless graph permits that low-level loader, while excluding client toolkits.
WINDOW_PREFIXES = ("accesskit_", "i-slint-", "slint-", "wayland-", "x11-", "x11rb", "xkbcommon")


class HeadlessDependenciesTests(unittest.TestCase):
    """Resolve the actual enabled Cargo dependencies of every headless package."""

    def test_release_graphs_are_window_system_independent(self):
        for package in ("pancake",):
            with self.subTest(package=package):
                graph = subprocess.run(
                    ["cargo", "tree", "--locked", "--target", "all", "--all-features",
                     "--package", package, "--edges", "normal,build", "--prefix", "none",
                     "--format", "{p}"],
                    cwd=ROOT, text=True, capture_output=True, check=True,
                )
                names = {line.split()[0] for line in graph.stdout.splitlines() if line.strip()}
                window = sorted(name for name in names
                                if name != "wayland-sys"
                                and (name in WINDOW_DEPENDENCIES or name.startswith(WINDOW_PREFIXES)))
                self.assertIn(package, names, graph.stdout)
                self.assertFalse(window, f"{package} resolves window-system dependencies: {window}")


if __name__ == "__main__":
    unittest.main()
