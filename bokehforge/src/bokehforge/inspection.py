"""Generate inspection maps from clean references during scoring."""

from hashlib import sha256
import os
from pathlib import Path
from threading import Lock

import numpy as np
from PIL import Image

from ._device import Device
from ._workers import workers
from .scoring import validate_heatmap, validate_rgb

INSPECTION_MAP_VERSION = "inspection-blend-v1"
_MAX_SIDE = 640


def _model_directory():
    override = os.environ.get("BOKEHFORGE_INSPECTION_MODELS")
    if override:
        return Path(override).expanduser().resolve()
    cache_dir = Path(os.environ.get("XDG_CACHE_HOME") or (Path.home() / ".cache"))
    return cache_dir / "bokehforge" / "inspection-models"


def reference_preview(reference):
    validate_rgb(reference)
    height, width = reference.shape[:2]
    scale = min(1, _MAX_SIDE / max(height, width))
    size = max(1, round(width * scale)), max(1, round(height * scale))
    pool = workers() if height * width >= 1_000_000 else None
    if pool is None:
        pixels = np.empty(reference.shape, np.uint8)
    else:
        planes = np.empty((3, height, width), np.uint8)
        pixels = np.moveaxis(planes, 0, -1)

    def encode(start):
        rows = slice(start, start + 128)
        linear = np.clip(reference[rows], 0, 1)
        srgb = linear ** (1 / 2.4)
        srgb *= 1.055
        srgb -= .055
        np.multiply(linear, 12.92, out=srgb, where=linear <= 0.0031308)
        srgb *= 255
        np.rint(srgb, out=srgb)
        pixels[rows] = srgb

    strips = range(0, height, 128)
    if pool is None:
        for start in strips:
            encode(start)
        return Image.fromarray(pixels).resize(size, Image.Resampling.LANCZOS)

    list(pool.map(encode, strips))

    def resize(plane):
        return Image.fromarray(plane).resize(size, Image.Resampling.LANCZOS)

    # Pillow applies the same fixed-point Lanczos filter independently to
    # each RGB channel. Run the three passes on contiguous planes in parallel.
    return Image.merge("RGB", tuple(pool.map(resize, planes)))


class Inspection:
    """Load the blend once and reuse the last reference map in this scoring session."""

    def __init__(self):
        self._predict = None
        self._last = None
        self._lock = Lock()

    @staticmethod
    def _load():
        from .inspection_models import inspection_blend

        try:
            return inspection_blend(_model_directory())
        except (ImportError, OSError, RuntimeError) as exc:
            raise RuntimeError(f"Could not load the Inspection blend: {exc}. "
                               "See the benchmark guide for model setup or use --no-attention.") from exc

    def preview_map(self, reference):
        """Return the original model grid and its digest, before native interpolation."""
        preview = reference_preview(reference)
        key = (preview.size, sha256(preview.tobytes()).digest())
        with self._lock:
            if self._last is None or self._last[0] != key:
                if self._predict is None:
                    self._predict = self._load()
                raw = validate_heatmap(self._predict(preview), preview.size[::-1])
                digest = sha256(raw.astype("<f4", copy=False).tobytes()).hexdigest()
                raw.setflags(write=False)
                self._last = key, raw, digest
            _, raw, digest = self._last
        return raw, digest

    def cached_map(self, digest):
        with self._lock:
            if self._last is not None and self._last[2] == digest:
                return self._last[1]
        return None

    def generate(self, reference):
        raw, digest = self.preview_map(reference)
        height, width = reference.shape[:2]
        if raw.shape != (height, width):
            raw = np.asarray(Image.fromarray(raw).resize((width, height), Image.Resampling.BILINEAR))
        return raw, digest


default_inspection = Inspection()


def inspection_for(device: Device):
    if device in ("cpu", "mps"):
        raise RuntimeError("The Inspection profile requires CUDA. Disable attention or supply a heatmap.")
    return default_inspection
