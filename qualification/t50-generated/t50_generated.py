"""Native-resolution GENERATED focal-stack scale data; not camera photographs."""

import argparse
import hashlib
import json
import platform
import resource
import shutil
import sys
import time
import uuid
import zlib
from dataclasses import asdict, dataclass
from functools import lru_cache
from pathlib import Path

import imagecodecs
import numpy as np
import tifffile
from t50_oracle import scalar_pixel


def file_hash(path):
    """Hash file bytes with a bounded 1 MiB read buffer."""
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def cleanup_owned(output, token, records):
    """Delete only an authenticated, fully verified set of this run's files."""
    if (
        output.is_symlink()
        or json.loads((output / "ownership.json").read_text())["token"] != token
    ):
        raise ValueError("Cleanup ownership does not match this run")
    paths = []
    for record in records:
        name = record["name"]
        if Path(name).name != name or not name.endswith(".tif"):
            raise ValueError("Cleanup path is not an owned TIFF basename")
        path = output / name
        if (
            path.is_symlink()
            or not path.is_file()
            or file_hash(path) != record["file_sha256"]
        ):
            raise ValueError("Cleanup file changed after verification")
        paths.append(path)
    if len(set(paths)) != len(paths):
        raise ValueError("Cleanup manifest repeats a path")
    deleted = [{"name": path.name, "bytes": path.stat().st_size} for path in paths]
    for path in paths:
        path.unlink()
    return deleted


@dataclass(frozen=True)
class Recipe:
    """Exact native dimensions, focal positions, phase seed and TIFF tile size."""

    width: int = 6000
    height: int = 4000
    frames: int = 50
    seed: int = 20261117
    tile: int = 256

    def __post_init__(self):
        if min(self.width, self.height) <= 0 or self.width * self.height > 64_000_000:
            raise ValueError("Native dimensions must contain 1..64000000 pixels")
        if not 2 <= self.frames <= min(self.width, 256):
            raise ValueError("Every focal layer must have a native image column")
        if not 16 <= self.tile <= 1024 or self.tile % 16:
            raise ValueError("TIFF tiles must be a multiple of 16 in 16..1024")
        if not 0 <= self.seed < 2**64:
            raise ValueError("Seed must fit unsigned 64-bit")


@lru_cache(maxsize=8)
def phases(seed):
    """Publish stable per-channel/per-band phase offsets from the recipe seed."""
    return tuple(
        tuple(
            tuple(
                hashlib.sha256(
                    f"generated-t50/v1/{seed}/{channel}/{band}".encode()
                ).digest()[:2]
            )
            for band in range(3)
        )
        for channel in range(3)
    )


def on_prefix(position, period):
    """Count the on-half of an infinite square wave, including negative positions."""
    half = period // 2
    return position // period * half + np.minimum(position % period, half)


def render_tile(recipe, focus, y0, x0, height, width):
    """Return one native-coordinate RGB16 tile."""
    if focus is not None and not 0 <= focus < recipe.frames:
        raise ValueError("Focus index is outside the generated depth range")
    if (
        min(x0, y0) < 0
        or min(height, width) <= 0
        or x0 + width > recipe.width
        or y0 + height > recipe.height
    ):
        raise ValueError("Tile is outside native image coordinates")
    x = np.arange(x0, x0 + width, dtype=np.int64)[None, :]
    y = np.arange(y0, y0 + height, dtype=np.int64)[:, None]
    depth = x * recipe.frames // recipe.width
    radius = np.zeros_like(depth) if focus is None else np.abs(depth - focus)
    length = 2 * radius + 1
    denominator = length * length
    output = np.empty((height, width, 3), dtype=np.uint16)
    offsets = phases(recipe.seed)
    for channel, bias in enumerate((32768, 30000, 35000)):
        values = (
            bias
            + ((73 * x + 137 * y + recipe.seed % 4096 + channel * 431) % 4096)
            - 2048
        )
        for band, (period, amplitude) in enumerate(
            ((4, 12000), (14, 7000), (62, 3000))
        ):
            phase_x, phase_y = offsets[channel][band]
            sum_x = (
                2
                * (
                    on_prefix(x + radius + phase_x + 1, period)
                    - on_prefix(x - radius + phase_x, period)
                )
                - length
            )
            sum_y = (
                2
                * (
                    on_prefix(y + radius + phase_y + 1, period)
                    - on_prefix(y - radius + phase_y, period)
                )
                - length
            )
            values = values + amplitude * sum_x * sum_y // denominator
        output[..., channel] = np.clip(values, 0, 65535).astype(np.uint16)
    return output


def pixel_bytes(array):
    """Canonical tile-trimmed C-order little-endian uint16 pixel bytes."""
    return np.asarray(array, dtype="<u2").tobytes(order="C")


def require_free(path, reserve, additional=0):
    """Refuse disk growth unless the requested reserve remains available."""
    free = shutil.disk_usage(path).free
    if free < reserve + additional:
        raise ValueError(f"Disk guard: {free} free bytes; need {reserve + additional}")
    return free


def write_and_verify(path, recipe, focus, role, reserve):
    """Stream native tiles to ZIP16 TIFF, then independently stream decode all tiles."""
    depth_only = role == "depth_truth"
    shape = (
        (recipe.height, recipe.width)
        if depth_only
        else (recipe.height, recipe.width, 3)
    )
    channels = 1 if depth_only else 3
    generated_hash = hashlib.sha256()
    expected_segments = ((recipe.height + recipe.tile - 1) // recipe.tile) * (
        (recipe.width + recipe.tile - 1) // recipe.tile
    )

    def tiles():
        """Yield fixed-size padded TIFF tiles and hash only native valid pixels."""
        for y in range(0, recipe.height, recipe.tile):
            for x in range(0, recipe.width, recipe.tile):
                height, width = (
                    min(recipe.tile, recipe.height - y),
                    min(recipe.tile, recipe.width - x),
                )
                require_free(
                    path.parent, reserve, recipe.tile * recipe.tile * channels * 4
                )
                if depth_only:
                    valid = np.broadcast_to(
                        np.arange(x, x + width, dtype=np.uint32)
                        * recipe.frames
                        // recipe.width,
                        (height, width),
                    ).astype(np.uint16)
                else:
                    valid = render_tile(recipe, focus, y, x, height, width)
                generated_hash.update(pixel_bytes(valid))
                padded = np.zeros(
                    (recipe.tile, recipe.tile)
                    if depth_only
                    else (recipe.tile, recipe.tile, 3),
                    dtype=np.uint16,
                )
                padded[:height, :width] = valid
                yield padded

    tifffile.imwrite(
        path,
        data=tiles(),
        shape=shape,
        dtype="<u2",
        byteorder="<",
        tile=(recipe.tile, recipe.tile),
        photometric="minisblack" if depth_only else "rgb",
        compression="adobe_deflate",
        compressionargs={"level": 6},
        predictor=2,
        maxworkers=1,
        metadata=None,
        description=json.dumps(
            {
                "kind": "GENERATED_SYNTHETIC_FOCAL_SCALE",
                "recipe": asdict(recipe),
                "role": role,
                "focus": focus,
            },
            sort_keys=True,
        ),
    )
    histogram = np.zeros(65536, dtype=np.int64)
    decoded_hash = hashlib.sha256()
    coordinates = {
        (0, 0),
        (recipe.width - 1, recipe.height - 1),
        (recipe.width // 2, recipe.height // 2),
    }
    coordinates.update(
        (
            min(
                recipe.width - 1, (2 * layer + 1) * recipe.width // (2 * recipe.frames)
            ),
            recipe.height // 3,
        )
        for layer in range(recipe.frames)
    )
    samples = []
    count = 0
    independent_hash = hashlib.sha256()
    with tifffile.TiffFile(path) as image:
        if len(image.pages) != 1:
            raise ValueError("Expected exactly one TIFF image page")
        page = image.pages[0]
        if (
            page.shape != shape
            or page.dtype != np.dtype("uint16")
            or page.bitspersample != 16
            or page.compression != 8
            or page.predictor != 2
        ):
            raise ValueError(
                "TIFF shape, precision, ZIP or predictor does not match recipe"
            )
        for data, index, _shape in page.segments(
            maxworkers=1, buffersize=8 * 1024 * 1024, sort=True
        ):
            y, x = index[2], index[3]
            expected_y = (
                count // ((recipe.width + recipe.tile - 1) // recipe.tile) * recipe.tile
            )
            expected_x = (
                count % ((recipe.width + recipe.tile - 1) // recipe.tile) * recipe.tile
            )
            if data is None or (y, x) != (expected_y, expected_x):
                raise ValueError("Decoded TIFF tile order or allocation is invalid")
            valid = data[
                0,
                : min(recipe.tile, recipe.height - y),
                : min(recipe.tile, recipe.width - x),
            ]
            if depth_only:
                valid = valid[..., 0]
            decoded_hash.update(pixel_bytes(valid))
            histogram += np.bincount(valid.reshape(-1), minlength=65536)
            for sx, sy in sorted(coordinates):
                if x <= sx < x + valid.shape[1] and y <= sy < y + valid.shape[0]:
                    actual = np.atleast_1d(valid[sy - y, sx - x]).tolist()
                    expected = (
                        [sx * recipe.frames // recipe.width]
                        if depth_only
                        else scalar_pixel(recipe, focus, sx, sy)
                    )
                    if actual != expected:
                        raise ValueError(
                            "Independent scalar native-coordinate oracle mismatch"
                        )
                    samples.append({"x": sx, "y": sy, "value": actual})
            count += 1
        # An independent DEFLATE implementation and explicit predictor reversal
        # verify every pixel without tifffile's imagecodecs decode path.
        with path.open("rb") as stream:
            for tile_index, (offset, size) in enumerate(
                zip(page.dataoffsets, page.databytecounts, strict=True)
            ):
                stream.seek(offset)
                encoded = stream.read(size)
                expected_bytes = recipe.tile * recipe.tile * channels * 2
                decoder = zlib.decompressobj()
                decoded = decoder.decompress(encoded, expected_bytes + 1)
                if (
                    len(decoded) != expected_bytes
                    or not decoder.eof
                    or decoder.unused_data
                    or decoder.unconsumed_tail
                ):
                    raise ValueError(
                        "Independent DEFLATE tile exceeds bounds or is incomplete"
                    )
                delta = np.frombuffer(decoded, dtype="<u2").reshape(
                    recipe.tile, recipe.tile, channels
                )
                reconstructed = np.add.accumulate(delta, axis=1, dtype=np.uint16)
                y = (
                    tile_index
                    // ((recipe.width + recipe.tile - 1) // recipe.tile)
                    * recipe.tile
                )
                x = (
                    tile_index
                    % ((recipe.width + recipe.tile - 1) // recipe.tile)
                    * recipe.tile
                )
                valid = reconstructed[
                    : min(recipe.tile, recipe.height - y),
                    : min(recipe.tile, recipe.width - x),
                ]
                independent_hash.update(pixel_bytes(valid))
    if (
        count != expected_segments
        or len(samples) != len(coordinates)
        or generated_hash.digest() != decoded_hash.digest()
        or independent_hash.digest() != generated_hash.digest()
    ):
        raise ValueError("Full decoded pixel hash, sample or segment count mismatch")
    nonzero = np.flatnonzero(histogram)
    nonpromotion = int(histogram[np.arange(65536) % 257 != 0].sum())
    unique = len(nonzero)
    if not depth_only and (unique <= 256 or nonpromotion == 0):
        raise ValueError("RGB data does not demonstrate genuine 16-bit precision")
    return {
        "name": path.name,
        "role": role,
        "focus": focus,
        "shape": list(shape),
        "dtype": "uint16",
        "bits_per_sample": 16,
        "compression": "ADOBE_DEFLATE",
        "predictor": 2,
        "tile": [recipe.tile, recipe.tile],
        "bytes": path.stat().st_size,
        "file_sha256": file_hash(path),
        "generated_pixel_sha256": generated_hash.hexdigest(),
        "decoded_pixel_sha256": decoded_hash.hexdigest(),
        "independent_zlib_pixel_sha256": independent_hash.hexdigest(),
        "converted_pixel_sha256": decoded_hash.hexdigest(),
        "unique_values": unique,
        "range": [int(nonzero[0]), int(nonzero[-1])],
        "samples_not_8bit_promoted": nonpromotion,
        "decoded_segments": count,
        "independent_scalar_samples": samples,
    }


def generate(recipe, output, receipt, reserve, max_bytes):
    """Create a new owned directory, qualify every native image, then remove images."""
    if (
        receipt.exists()
        or not receipt.parent.is_dir()
        or receipt.is_relative_to(output)
    ):
        raise ValueError("Receipt must be a new file outside the generated directory")
    free_before = require_free(output.parent, reserve, 4 * 1024 * 1024)
    output.mkdir()
    token = str(uuid.uuid4())
    (output / "ownership.json").write_text(
        json.dumps(
            {"token": token, "kind": "GENERATED_SYNTHETIC_FOCAL_SCALE"}, sort_keys=True
        )
        + "\n"
    )
    start = time.monotonic()
    files = []
    deleted = []
    for focus, role, name in [
        (i, "focus_frame", f"focus_{i:03d}.tif") for i in range(recipe.frames)
    ] + [
        (None, "sharp_truth", "sharp_truth.tif"),
        (None, "depth_truth", "depth_truth.tif"),
    ]:
        # Conservative raw-size bound plus headers; compression cannot consume the whole budget.
        upper_bound = ((recipe.width + recipe.tile - 1) // recipe.tile) * (
            (recipe.height + recipe.tile - 1) // recipe.tile
        ) * recipe.tile**2 * (2 if role == "depth_truth" else 6) * 2 + 4 * 1024 * 1024
        if upper_bound > max_bytes:
            raise ValueError("Generated-byte cap cannot safely fit another TIFF")
        require_free(output, reserve, upper_bound)
        record = write_and_verify(output / name, recipe, focus, role, reserve)
        files.append(record)
        deleted.extend(cleanup_owned(output, token, [record]))
        print(
            json.dumps(
                {
                    "verified": name,
                    "bytes": record["bytes"],
                    "unique_values": record["unique_values"],
                }
            ),
            flush=True,
        )
    focus_hashes = {
        record["decoded_pixel_sha256"]
        for record in files
        if record["role"] == "focus_frame"
    }
    if len(focus_hashes) != recipe.frames:
        raise ValueError("Generated focal frames are not all distinct")
    data = {
        "schema_version": 1,
        "dataset_kind": "GENERATED_SYNTHETIC_FOCAL_SCALE",
        "frame_count": recipe.frames,
        "maximum_simultaneous_image_files": 1,
        "total_generated_tiff_bytes": sum(record["bytes"] for record in files),
        "peak_retained_tiff_bytes": max(record["bytes"] for record in files),
        "recipe": asdict(recipe),
        "generator_license": "GPL-3.0-or-later",
        "generated_pixels_license": "CC0-1.0",
        "acquired_download_bytes": 0,
        "precision": "Direct native uint16 arithmetic; no 8-bit input, resize or intensity promotion",
        "conversion": "none; native RGB16 ZIP TIFF; converted hash equals decoded hash",
        "pixel_hash_order": "TIFF tile-row-major; trim each tile to native image bounds; C-order little-endian uint16 per tile",
        "known_truth": {
            "depth": "floor(x*frames/width)",
            "defocus_radius": "abs(depth-focus)",
            "sharp_truth": "focus=None; all radii zero",
            "scope": "Deterministic variable-defocus scale workload; not photographs or a physical lens model",
        },
        "limits": {
            "reserve_bytes": reserve,
            "max_generated_bytes": max_bytes,
            "maximum_decode_workers": 1,
            "full_frame_buffers": 0,
            "decode_buffer_bytes": 8 * 1024 * 1024,
        },
        "versions": {
            "python": platform.python_version(),
            "numpy": np.__version__,
            "tifffile": tifffile.__version__,
            "imagecodecs": imagecodecs.__version__,
            "zlib": zlib.ZLIB_VERSION,
            "platform": platform.platform(),
        },
        "sources": {
            name: file_hash(Path(__file__).with_name(name))
            for name in ("t50_generated.py", "t50_oracle.py", "test_t50_generated.py")
        },
        "files": files,
        "elapsed_seconds": time.monotonic() - start,
        "peak_rss_bytes": resource.getrusage(resource.RUSAGE_SELF).ru_maxrss
        * (1 if sys.platform == "darwin" else 1024),
        "free_disk_before_bytes": free_before,
        "free_disk_after_bytes": shutil.disk_usage(output).free,
        "cleanup": {
            "owned_output_directory": str(output.resolve()),
            "deleted_image_count": len(deleted),
            "deleted_bytes": sum(item["bytes"] for item in deleted),
            "deleted_files": deleted,
            "remaining_tiff_count": len(list(output.glob("*.tif"))),
            "ownership_marker_retained": True,
        },
    }
    with receipt.open("x") as stream:
        json.dump(data, stream, indent=2, sort_keys=True)
        stream.write("\n")
    return data


def main():
    """Run an explicitly GENERATED qualification and leave evidence, not image data."""
    parser = argparse.ArgumentParser(description=__doc__)
    for name, default in asdict(Recipe()).items():
        parser.add_argument("--" + name, type=int, default=default)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--receipt", type=Path, required=True)
    parser.add_argument("--reserve-gib", type=float, default=15)
    parser.add_argument("--max-generated-bytes", type=int, default=1490980001)
    args = parser.parse_args()
    try:
        recipe = Recipe(**{name: getattr(args, name) for name in asdict(Recipe())})
        if args.reserve_gib < 0 or args.max_generated_bytes <= 0:
            raise ValueError(
                "Resource limits must be nonnegative reserve and positive byte cap"
            )
        generate(
            recipe,
            args.output.resolve(),
            args.receipt.resolve(),
            int(args.reserve_gib * 1024**3),
            args.max_generated_bytes,
        )
    except (ValueError, OSError) as error:
        parser.exit(1, f"{error}\n")


if __name__ == "__main__":
    main()
