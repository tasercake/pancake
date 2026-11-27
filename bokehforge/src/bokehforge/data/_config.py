"""Locate and read the source policy shared by BokehForge entry points."""

import json
import os
from pathlib import Path
import sys

DEFAULT_CONFIG = Path(__file__).with_name("sources.json").resolve()


def read_config(path):
    path = Path(path)
    if not path.is_file():
        raise ValueError(f"Source configuration not found: {path}; specify its path with --config or config=")
    return json.loads(path.read_text())


def data_root(config=DEFAULT_CONFIG, defaults=None):
    if override := os.environ.get("BOKEHFORGE_DATA_ROOT"):
        return Path(override).expanduser().resolve()
    if defaults is None:
        defaults = read_config(config).get("defaults", {})
    if configured := defaults.get("data_root"):
        return (Path(config).parent / Path(configured).expanduser()).resolve()
    home = Path.home()
    if sys.platform == "darwin":
        base = home / "Library/Application Support"
    elif os.name == "nt":
        base = Path(os.environ.get("LOCALAPPDATA") or home / "AppData/Local")
    else:
        base = Path(os.environ.get("XDG_DATA_HOME") or home / ".local/share")
    return (base / "pancake/datasets").resolve()
