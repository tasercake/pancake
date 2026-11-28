"""Generate inspection maps from clean references during scoring."""

from functools import lru_cache
from hashlib import sha256
import os
from pathlib import Path
from threading import Lock

import numpy as np
from PIL import Image

from ._device import Device
from ._workers import workers
from .scoring import validate_heatmap, validate_rgb

try:
    from . import _rust
except ImportError:
    _rust = None

INSPECTION_MAP_VERSION = "inspection-blend-v1"
_MAX_SIDE = 640


def _model_directory():
    override = os.environ.get("BOKEHFORGE_INSPECTION_MODELS")
    if override:
        # Keep the cache entry itself available for dangling-link recovery.
        return Path(override).expanduser().absolute()
    cache_dir = Path(os.environ.get("XDG_CACHE_HOME") or (Path.home() / ".cache"))
    return cache_dir / "bokehforge" / "inspection-models"


def _encode_srgb8(samples):
    linear = np.clip(samples, 0, 1)
    srgb = linear ** (1 / 2.4)
    srgb *= 1.055
    srgb -= .055
    np.multiply(linear, 12.92, out=srgb, where=linear <= 0.0031308)
    srgb *= 255
    np.rint(srgb, out=srgb)
    return srgb.astype(np.uint8)


@lru_cache(maxsize=1)
def _preview_encoder():
    # The inverse curve centers a search for each byte transition. Use the
    # local NumPy float32 curve and ties-to-even rounding within that window.
    codes = np.arange(255)
    levels = (codes + .5) / 255
    inverse = np.where(levels <= .04045, levels / 12.92, ((levels + .055) / 1.055) ** 2.4)
    nearest = inverse.astype(np.float32).view(np.int32)
    bits = nearest[:, None] + np.arange(-64, 65, dtype=np.int32)
    quantized = _encode_srgb8(bits.view(np.float32))
    if (np.any(quantized[:, 0] != codes) or np.any(quantized[:, -1] != codes + 1)
            or np.any(quantized[:, :-1] > quantized[:, 1:])):
        return None
    transition = np.argmax(quantized > codes[:, None], axis=1)
    return _rust.PreviewEncoder(bits[codes, transition].tolist())


def reference_preview(reference):
    validate_rgb(reference)
    height, width = reference.shape[:2]
    scale = min(1, _MAX_SIDE / max(height, width))
    size = max(1, round(width * scale)), max(1, round(height * scale))
    pool = workers() if height * width >= 1_000_000 else None
    if pool is None:
        pixels = np.empty(reference.shape, np.uint8)
        for start in range(0, height, 128):
            rows = slice(start, start + 128)
            pixels[rows] = _encode_srgb8(reference[rows])
        return Image.fromarray(pixels).resize(size, Image.Resampling.LANCZOS)

    encoder = None
    if (_rust is not None and type(reference) is np.ndarray and reference.dtype == np.float32
            and reference.flags.c_contiguous and reference.flags.aligned):
        encoder = _preview_encoder()
    encode = _encode_srgb8 if encoder is None else encoder.encode
    planes = np.empty((3, height, size[0]), np.uint8)
    rows_per_strip = 128 if encoder is None else min(256, max(1, (height + 7) // 8))

    def horizontal(start):
        pixels = encode(reference[start:start + rows_per_strip])
        if width != size[0]:
            strip = Image.fromarray(pixels).resize((size[0], len(pixels)), Image.Resampling.LANCZOS)
            pixels = np.asarray(strip)
        planes[:, start:start + len(pixels)] = np.moveaxis(pixels, -1, 0)

    def vertical(plane):
        return Image.fromarray(plane).resize(size, Image.Resampling.LANCZOS)

    # Pillow applies horizontal filtering first and uses identical fixed-point
    # Lanczos weights for every RGB channel. Horizontal strips are independent.
    list(pool.map(horizontal, range(0, height, rows_per_strip)))
    return Image.merge("RGB", tuple(pool.map(vertical, planes)))


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
