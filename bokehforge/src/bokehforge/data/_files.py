"""Source identity, path checks, and frame filename ordering."""

import hashlib
from pathlib import PurePosixPath
import re


def digest(path, algorithm="sha256"):
    h = hashlib.new(algorithm)
    if algorithm == "sha1":
        h.update(f"blob {path.stat().st_size}\0".encode())
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(4 * 1024 * 1024), b""):
            h.update(chunk)
    return h.hexdigest()


def safe_path(root, relative):
    name = PurePosixPath(relative.replace("\\", "/"))
    if name.is_absolute() or not name.parts or any(p in {"..", ""} for p in name.parts):
        raise ValueError(f"Unsafe source path: {relative}")
    path = root.joinpath(*name.parts)
    if not path.resolve().is_relative_to(root.resolve()):
        raise ValueError(f"Source path escapes root: {relative}")
    return path


def natural_key(value):
    return [int(x) if x.isdigit() else x.casefold() for x in re.split(r"(\d+)", value)]
