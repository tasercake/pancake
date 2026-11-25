"""Locate and read the source policy shared by BokehForge entry points."""

import json
from pathlib import Path

DEFAULT_CONFIG = next((parent / "config/data/sources.json" for parent in Path(__file__).resolve().parents
                       if (parent / "config/data/sources.json").is_file()), Path("config/data/sources.json"))


def read_config(path):
    path = Path(path)
    if not path.is_file():
        raise ValueError(f"Source configuration not found: {path}; specify its path with --config or config=")
    return json.loads(path.read_text())


def data_root(config, defaults):
    return Path(config).parent / defaults.get("data_root", ".")
