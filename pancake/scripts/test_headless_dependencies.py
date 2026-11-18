"""Verify that release frontends and the compute core have separate Cargo graphs."""

from pathlib import Path
import subprocess
import unittest


ROOT = Path(__file__).resolve().parents[1]
WINDOW_DEPENDENCIES = {"accesskit", "atk", "atk-sys", "gdk", "gdk-sys", "rfd",
                       "slint", "tiny-xlib", "winit", "x11", "xcb"}
# wgpu-hal can load Wayland symbols dynamically for GLES presentation.
# The headless graph permits that low-level loader, while excluding client toolkits.
WINDOW_PREFIXES = ("accesskit_", "gtk", "i-slint-", "slint-", "wayland-", "x11-", "x11rb", "xkbcommon")


def package_names(package, target):
    """Read the locked normal and build dependencies for one platform."""
    graph = subprocess.run(
        ["cargo", "tree", "--locked", "--target", target, "--all-features",
         "--package", package, "--edges", "normal,build", "--prefix", "none",
         "--format", "{p}"],
        cwd=ROOT, text=True, capture_output=True, check=True,
    )
    return {line.split()[0] for line in graph.stdout.splitlines() if line.strip()}


class HeadlessDependenciesTests(unittest.TestCase):
    """Resolve the actual enabled Cargo dependencies of every headless package."""

    def test_release_graphs_are_window_system_independent(self):
        for package in ("pancake", "pancake-cli"):
            with self.subTest(package=package):
                names = package_names(package, "all")
                window = sorted(name for name in names
                                if name != "wayland-sys"
                                and (name in WINDOW_DEPENDENCIES or name.startswith(WINDOW_PREFIXES)))
                self.assertIn(package, names)
                self.assertFalse(window, f"{package} resolves window-system dependencies: {window}")

    def test_gui_resolves_the_native_panel_for_each_release_platform(self):
        panels = {
            "x86_64-unknown-linux-gnu": "gtk",
            "aarch64-apple-darwin": "rfd",
            "x86_64-pc-windows-msvc": "rfd",
        }
        for target, panel in panels.items():
            with self.subTest(target=target):
                names = package_names("pancake-gui", target)
                self.assertTrue({"pancake", "slint", "slint-interpreter"} <= names)
                self.assertEqual(names & {"gtk", "rfd"}, {panel})


if __name__ == "__main__":
    unittest.main()
