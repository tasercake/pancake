"""Display rendering and interchange with external stackers."""

from collections import OrderedDict
from concurrent.futures import Future
from contextlib import ExitStack, closing, contextmanager
from dataclasses import dataclass
from io import BytesIO
from pathlib import Path
from threading import Condition, Lock
from zipfile import ZipFile

import numpy as np
from PIL import Image, features
import tifffile

from ..data._canonical import decode, encode_tiff
from ..data.lazy import FrameDecoder


def srgb_samples(pixels, bits):
    linear = np.clip(pixels, 0, 1)
    low = linear <= 0.0031308
    linear[low] *= 12.92
    linear[~low] = 1.055 * linear[~low] ** (1 / 2.4) - 0.055
    return np.rint(linear * ((1 << bits) - 1)).astype(f"uint{bits}")


# Only scaled previews use the lookup table. The full-size lightbox keeps exact
# 8-bit display values. A 16-bit linear step is < 0.06 of an 8-bit sRGB step.
_DISPLAY_LUT = srgb_samples(np.linspace(0, 1, 65536, dtype=np.float32), 8)
WEBP = features.check("webp")
SIZES = {"thumb": 192, "panel": 768, "fit": 1500, "native": 0}
PREVIEW_VERSION = "2"


def preview_mime(variant):
    if variant == "thumb":
        return "image/jpeg"
    return "image/webp" if WEBP and variant != "native" else "image/png"


def display_samples(pixels, scaled):
    if not scaled:
        return srgb_samples(pixels, 8)
    linear = np.clip(pixels, 0, 1)
    linear *= 65535
    return _DISPLAY_LUT[linear.astype(np.uint16)]


def preview(pixels, variant):
    size = SIZES[variant]
    image = Image.fromarray(display_samples(pixels, size > 0))
    return encode_preview(image, variant)


def encode_preview(image, variant):
    size = SIZES[variant]
    if size:
        image.thumbnail((size, size), Image.Resampling.LANCZOS, reducing_gap=2)
    with BytesIO() as stream:
        mime = preview_mime(variant)
        if mime == "image/jpeg":
            image.save(stream, format="JPEG", quality=82)
        elif mime == "image/webp":
            image.save(stream, format="WEBP", lossless=True, method=0)
        else:
            image.save(stream, format="PNG", compress_level=1)
        return stream.getvalue()


class DecodeSlots:
    """Reserve one decode slot for visible images while thumbnails compete."""

    def __init__(self):
        self.condition = Condition()
        self.busy = 0
        self.background = 0
        self.waiting = 0

    @contextmanager
    def slot(self, background):
        with self.condition:
            self.waiting += not background
            try:
                self.condition.wait_for(lambda: self.busy < 2 and
                                        (not background or (not self.background and not self.waiting)))
            finally:
                self.waiting -= not background
            self.busy += 1
            self.background += background
        try:
            yield
        finally:
            with self.condition:
                self.busy -= 1
                self.background -= background
                self.condition.notify_all()


@dataclass
class _ZipIndex:
    archive: ZipFile
    leases: int = 0


class ReviewFrameDecoder(FrameDecoder):
    """Retain a few ZIP indexes for random access without changing canonical decoding."""

    def __init__(self):
        self.archives = OrderedDict()
        self.lock = Lock()

    @contextmanager
    def _lease_zip(self, asset):
        path = asset.path
        with self.lock:
            cached = self.archives.get(path)
            if cached is None:
                if len(self.archives) >= 4:
                    oldest_idle = next((key for key, entry in self.archives.items() if not entry.leases), None)
                    if oldest_idle is not None:
                        self.archives.pop(oldest_idle).archive.close()
                if len(self.archives) < 4:
                    asset.check()
                    cached = self.archives[path] = _ZipIndex(ZipFile(path))
            if cached is not None:
                cached.leases += 1
                self.archives.move_to_end(path)
        try:
            yield cached.archive if cached is not None else None
        finally:
            if cached is not None:
                with self.lock:
                    cached.leases -= 1

    def decode(self, frame, *, output="array"):
        original = frame._original
        path = original.asset.path
        if original.member is None or path.suffix.lower() != ".zip":
            return super().decode(frame, output=output)
        with self._lease_zip(original.asset) as archive:
            if archive is None:
                return super().decode(frame, output=output)
            with ExitStack() as scope:
                return self._decode_frame(frame, output, {path: archive}, scope)


class DecodedFrames:
    """Retain the last working set so multiple views can share the same decode."""

    LIMIT = 224 * 1024 ** 2

    def __init__(self):
        self.frames = OrderedDict()
        self.pending = {}
        self.size = 0
        self.lock = Lock()

    def get(self, key, read):
        with self.lock:
            cached = self.frames.get(key)
            if cached is not None:
                self.frames.move_to_end(key)
                return cached
            pending = self.pending.get(key)
            if pending is None:
                pending = self.pending[key] = Future()
                leader = True
            else:
                leader = False
        if not leader:
            return pending.result()
        try:
            pixels = read()
            if pixels.nbytes <= self.LIMIT:
                with self.lock:
                    self.frames[key] = pixels
                    self.size += pixels.nbytes
                    while self.size > self.LIMIT or len(self.frames) > 48:
                        _, evicted = self.frames.popitem(last=False)
                        self.size -= evicted.nbytes
            pending.set_result(pixels)
            return pixels
        except BaseException as exc:
            pending.set_exception(exc)
            raise
        finally:
            with self.lock:
                del self.pending[key]


class PreviewCache:
    """Bound the memory and disk cost of encoded previews and share active renders."""

    MEMORY_LIMIT = 48 * 1024 ** 2
    DISK_LIMIT = 384 * 1024 ** 2
    MAX_FILES = 12000

    def __init__(self, directory):
        self.directory = directory / f"previews-v{PREVIEW_VERSION}"
        self.memory = OrderedDict()
        self.disk = OrderedDict()
        self.memory_bytes = self.disk_bytes = 0
        self.lock = Lock()
        self.slots = DecodeSlots()
        self.pending = {}
        try:
            self.directory.mkdir(exist_ok=True)
            self._index_disk()
        except OSError:
            self._disable_disk()

    def _disable_disk(self):
        self.directory = None
        self.disk.clear()
        self.disk_bytes = 0

    def _index_disk(self):
        files = [(path.stat(), path) for path in self.directory.glob("*.preview")]
        for info, path in sorted(files, key=lambda item: item[0].st_mtime_ns):
            self.disk[path.stem] = info.st_size
            self.disk_bytes += info.st_size
        self._prune_disk()

    def _prune_disk(self):
        while self.disk_bytes > self.DISK_LIMIT or len(self.disk) > self.MAX_FILES:
            key = next(iter(self.disk))
            (self.directory / f"{key}.preview").unlink(missing_ok=True)
            self.disk_bytes -= self.disk.pop(key)

    def _remember(self, key, data):
        if len(data) > self.MEMORY_LIMIT:
            return
        self.memory[key] = data
        self.memory_bytes += len(data)
        while self.memory_bytes > self.MEMORY_LIMIT or len(self.memory) > 1024:
            _, evicted = self.memory.popitem(last=False)
            self.memory_bytes -= len(evicted)

    def _lookup(self, key):
        if key in self.memory:
            self.memory.move_to_end(key)
            if key in self.disk:
                self.disk.move_to_end(key)
            return self.memory[key]
        if key not in self.disk:
            return None
        path = self.directory / f"{key}.preview"
        try:
            result = path.read_bytes()
            self.disk.move_to_end(key)
            self._remember(key, result)
            return result
        except OSError:
            self.disk_bytes -= self.disk.pop(key)
            return None

    def _save(self, key, payload):
        if self.directory is None:
            return
        pending = self.directory / f"{key}.tmp"
        try:
            pending.write_bytes(payload)
            pending.replace(self.directory / f"{key}.preview")
            self.disk[key] = len(payload)
            self.disk_bytes += len(payload)
            self._prune_disk()
        except OSError:
            self._disable_disk()
            try:
                pending.unlink(missing_ok=True)
            except OSError:
                pass

    def render(self, key, callback, *, background=False, persistent=True):
        with self.lock:
            cached = self._lookup(key)
            if cached is not None:
                return cached
            pending = self.pending.get(key)
            if pending is None:
                pending = self.pending[key] = Future()
                leader = True
            else:
                leader = False
        if not leader:
            return pending.result()
        try:
            with self.slots.slot(background):
                payload = callback()
            with self.lock:
                self._remember(key, payload)
                if persistent:
                    self._save(key, payload)
            pending.set_result(payload)
            return payload
        except BaseException as exc:
            pending.set_exception(exc)
            raise
        finally:
            with self.lock:
                del self.pending[key]


def stage_frame(path, pixels):
    tifffile.imwrite(path, srgb_samples(pixels, 16), photometric="rgb",
                     compression="deflate", compressionargs={"level": 2}, maxworkers=1)


def normalize_output(delivery, destination):
    with closing(decode(delivery, "srgb_assumed", page=0)) as decoded:
        pixels, _, metadata = next(decoded)
    destination.parent.mkdir(parents=True, exist_ok=True)
    pending = destination.with_suffix(".tmp")
    try:
        encode_tiff(pending, pixels)
        pending.replace(destination)
    finally:
        pending.unlink(missing_ok=True)
    return {"width": pixels.shape[1], "height": pixels.shape[0],
            "native_dtype": metadata["native_dtype"],
            "output_transfer": "embedded ICC" if "icc_transform" in metadata else "assumed sRGB"}


def output_pixels(path: Path):
    return tifffile.imread(path, maxworkers=1)
