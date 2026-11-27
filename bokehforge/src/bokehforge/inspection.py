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
    pixels = np.empty(reference.shape, np.uint8)

    def encode(start):
        rows = slice(start, start + 128)
        linear = np.clip(reference[rows], 0, 1)
        srgb = np.where(linear <= 0.0031308, 12.92 * linear, 1.055 * linear ** (1 / 2.4) - .055)
        pixels[rows] = np.rint(srgb * 255)

    strips = range(0, reference.shape[0], 128)
    if reference.shape[0] * reference.shape[1] >= 1_000_000:
        list(workers().map(encode, strips))
    else:
        for start in strips:
            encode(start)
    height, width = reference.shape[:2]
    scale = min(1, _MAX_SIDE / max(height, width))
    size = max(1, round(width * scale)), max(1, round(height * scale))
    return Image.fromarray(pixels).resize(size, Image.Resampling.LANCZOS)


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

    def generate(self, reference):
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
        height, width = reference.shape[:2]
        if raw.shape != (height, width):
            raw = np.asarray(Image.fromarray(raw).resize((width, height), Image.Resampling.BILINEAR))
        return raw, digest


default_inspection = Inspection()


def inspection_for(device: Device):
    if device == "cpu":
        raise RuntimeError("The Inspection profile requires CUDA. Disable attention or supply a heatmap.")
    return default_inspection
