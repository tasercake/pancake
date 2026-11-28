"""Locate the native Inspection runtime without importing model code."""

from functools import cache
from importlib.metadata import PackageNotFoundError, distribution
import os
from pathlib import Path
import sys

from . import _rust


def model_directory():
    override = os.environ.get("BOKEHFORGE_INSPECTION_MODELS")
    if override:
        # Keep the cache entry itself available for dangling-link recovery.
        return Path(override).expanduser().absolute()
    cache_dir = Path(os.environ.get("XDG_CACHE_HOME") or Path.home() / ".cache")
    return cache_dir / "bokehforge" / "inspection-models"


def _runtime_library():
    if override := os.environ.get("ORT_DYLIB_PATH"):
        return Path(override).expanduser().resolve()
    for package in ("onnxruntime-gpu", "onnxruntime"):
        try:
            runtime = distribution(package)
        except PackageNotFoundError:
            continue
        for entry in runtime.files or ():
            name = entry.name
            if (name == "onnxruntime.dll" or name == "libonnxruntime.so" or name.startswith("libonnxruntime.so.")
                    or (name.startswith("libonnxruntime.") and name.endswith(".dylib"))):
                return Path(runtime.locate_file(entry)).resolve()
    return None


@cache
def prepare_attention():
    _rust.configure_attention(model_directory(), _runtime_library(), Path(sys.executable))
