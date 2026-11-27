"""Prepare and read reference maps for inspection-weighted scores."""

import argparse
from contextlib import redirect_stdout
from hashlib import sha256
from io import BytesIO
import json
from pathlib import Path
import sys
from tempfile import NamedTemporaryFile

import numpy as np
from PIL import Image

from .data._config import DEFAULT_CONFIG, data_root, read_config
from .data.lazy import SceneCatalog
from .scoring import validate_heatmap, validate_rgb

INSPECTION_MAP_VERSION = "inspection-blend-v1"
_MAX_SIDE = 640


def reference_key(catalog, scene, preview):
    frame = catalog.frame(scene.reference_ids[0])
    identity = (frame.asset_sha256, frame.source_sha256, frame.page, preview.size)
    key = sha256(json.dumps(identity).encode())
    key.update(preview.tobytes())
    return key.hexdigest()


def map_path(directory, scene):
    return directory / (scene.id + ".npz")


def _map_size(shape):
    height, width = shape
    scale = min(1, _MAX_SIDE / max(shape))
    return max(1, round(width * scale)), max(1, round(height * scale))


def read_map(path, reference, key):
    """Read one verified, reference-bound map and interpolate to native pixel centers."""
    payload = path.read_bytes()
    height, width = reference.shape[:2]
    size = _map_size((height, width))
    with np.load(BytesIO(payload), allow_pickle=False) as data:
        if (data["version"].item() != INSPECTION_MAP_VERSION or data["reference"].item() != key
                or tuple(data["shape"]) != (height, width)):
            raise ValueError(f"Inspection map does not match the reference or map version: {path}")
        heatmap = validate_heatmap(data["heatmap"], size[::-1])
    if size != (width, height):
        heatmap = np.asarray(Image.fromarray(heatmap).resize((width, height), Image.Resampling.BILINEAR))
    return heatmap, sha256(payload).hexdigest()


def write_map(path, key, shape, heatmap):
    heatmap = validate_heatmap(heatmap, _map_size(shape)[::-1])
    path.parent.mkdir(parents=True, exist_ok=True)
    staged = None
    try:
        with NamedTemporaryFile(dir=path.parent, suffix=".npz", delete=False) as temporary:
            staged = Path(temporary.name)
            np.savez_compressed(temporary, version=INSPECTION_MAP_VERSION, reference=key, shape=shape, heatmap=heatmap)
        staged.replace(path)
    finally:
        if staged is not None:
            staged.unlink(missing_ok=True)


def reference_preview(reference):
    validate_rgb(reference)
    pixels = np.empty(reference.shape, np.uint8)
    for start in range(0, reference.shape[0], 128):
        rows = slice(start, start + 128)
        linear = np.clip(reference[rows], 0, 1)
        srgb = np.where(linear <= 0.0031308, 12.92 * linear, 1.055 * linear ** (1 / 2.4) - .055)
        pixels[rows] = np.rint(srgb * 255)
    image = Image.fromarray(pixels)
    return image.resize(_map_size(reference.shape[:2]), Image.Resampling.LANCZOS)


def main(argv=None):
    from .bench import _require_benchmark_reference

    parser = argparse.ArgumentParser(prog="bokehforge inspection",
                                     description="Prepare inspection blend maps from clean catalog references")
    parser.add_argument("--models", type=Path, required=True, help="Local inspection model files and upstream code")
    parser.add_argument("--output", type=Path, required=True, help="Directory for the generated reference maps")
    parser.add_argument("--root", type=Path, help="Dataset root")
    parser.add_argument("--config", type=Path, default=DEFAULT_CONFIG, help="Dataset source configuration")
    parser.add_argument("--lock", type=Path, help="Optional source snapshot")
    parser.add_argument("--dataset", action="append", help="Dataset ID, repeatable")
    parser.add_argument("--scene", action="append", help="Exact catalog scene ID, repeatable")
    args = parser.parse_args(argv)
    root = args.root or data_root(args.config, read_config(args.config).get("defaults", {}))
    catalog = SceneCatalog(root, datasets=args.dataset, config=args.config, lock=args.lock)
    scenes = [catalog.scene(key) for key in dict.fromkeys(args.scene or catalog.scene_ids)]
    if not scenes:
        raise ValueError("No reference scenes selected")
    for scene in scenes:
        _require_benchmark_reference(scene)

    from .inspection_models import inspection_blend

    with redirect_stdout(sys.stderr):
        predict = inspection_blend(args.models.resolve())
    args.output.mkdir(parents=True, exist_ok=True)
    for scene in scenes:
        reference = catalog.decode(scene.reference_ids[0])
        if reference.alpha is not None:
            raise ValueError(f"The RGB benchmark reference cannot have native alpha: {scene.id}")
        preview = reference_preview(reference.data)
        with redirect_stdout(sys.stderr):
            heatmap = predict(preview)
        write_map(map_path(args.output, scene), reference_key(catalog, scene, preview), reference.data.shape[:2], heatmap)
    return 0
