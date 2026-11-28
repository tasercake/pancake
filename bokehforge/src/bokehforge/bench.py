"""Run a stacker against a reproducible cohort from the local SceneCatalog."""

import argparse
from contextlib import closing, redirect_stdout, suppress
from dataclasses import dataclass
from importlib import import_module
from importlib.metadata import version
import json
import os
from pathlib import Path
import sys
from time import perf_counter

import numpy as np

from . import _rust
from ._attention_setup import prepare_attention
from ._device import DEVICES, Device
from ._score_contract import (
    ALIGNMENT_VERSION, MAX_ROTATION_DEGREES, FAMILY_WEIGHTS, INSPECTION_MAP_VERSION,
    INSPECTION_MAX_MULTIPLIER, INSPECTION_MIX, INSPECTION_SCORE_VERSION, SCORE_DAMAGE_SCALE,
    SCORE_DETAIL_LEVELS, SCORE_EXPLANATION_VERSION, SCORE_VERSION, ScoreDetails,
)
from .data._canonical import encode_tiff
from .data._files import digest
from .data._config import DEFAULT_CONFIG, data_root
from .data.lazy import Scene, SceneCatalog
from .stacking import StackFrame, StackInput

_RGB_REFERENCES = {"rendered_all_in_focus", "synthetic_source_image_reference"}
_MIN_PARALLEL_PIXELS = 1_000_000
_MAX_PARALLEL_PIXELS = 20_000_000


validate_rgb = _rust.validate_rgb


def _measure_delivery(*args, **kwargs):
    from .scoring import _measure_delivery

    return _measure_delivery(*args, **kwargs)


_default_measure_delivery = _measure_delivery


def _benchmark_override(attention):
    if _measure_delivery is _default_measure_delivery:
        return None, None
    from .inspection import Inspection

    return _measure_delivery, Inspection() if attention else None


def _validate_details(details):
    if details is not None and details not in SCORE_DETAIL_LEVELS:
        raise ValueError(f"Expected score details to be None or one of {SCORE_DETAIL_LEVELS}, got {details!r}")


class _InputError(RuntimeError):
    """An error raised by the evaluator's focal frame stream."""


def _positive_int(value):
    number = int(value)
    if number < 1:
        raise argparse.ArgumentTypeError("must be a positive integer")
    return number


def _load_algorithm(spec):
    if spec == "mean":
        spec = "bokehforge.stacking:mean"
    module_name, _, name = spec.partition(":")
    if not module_name or not name:
        raise ValueError("Use --algorithm mean or an importable module:callable")
    try:
        with redirect_stdout(sys.stderr):
            module = import_module(module_name)
        algorithm = getattr(module, name)
    except (ImportError, AttributeError) as exc:
        raise ValueError(f"Cannot load {spec}: {exc}") from exc
    if not callable(algorithm):
        raise ValueError(f"{spec} is not callable")
    module_file = getattr(module, "__file__", None)
    module_hash = digest(Path(module_file)) if module_file and Path(module_file).is_file() else None
    return algorithm, {"entry_point": spec, "module_sha256": module_hash}


def _error_text(exc):
    return f"{type(exc).__name__}: {exc}"


def _read_frames(catalog, scene, row, workers):
    with closing(catalog.decoder.iter_decode(map(catalog.frame, scene.frame_ids), workers=workers)) as decoded:
        for frame_id in scene.frame_ids:
            try:
                frame = next(decoded)
            except Exception as exc:
                raise _InputError(f"{frame_id}: {_error_text(exc)}") from exc
            for array in (frame.data, frame.alpha):
                if array is not None:
                    array.setflags(write=False)
            row["frames_read"] += 1
            yield StackFrame(frame_id, frame.provenance["ordinal"], frame.data, frame.alpha,
                             frame.provenance.get("focus_distance_m"), frame.provenance.get("aperture_f_number"))


def has_benchmark_reference(scene):
    return scene.metadata["reference_type"] in _RGB_REFERENCES and len(scene.reference_ids) == 1


def _require_benchmark_reference(scene):
    if not has_benchmark_reference(scene):
        kind = scene.metadata["reference_type"]
        raise ValueError(f"Benchmark scene {scene.id} requires exactly one supported RGB ground-truth image "
                         f"(reference type {kind!r}, found {len(scene.reference_ids)} images).")


def _reference(catalog, scene, row):
    try:
        reference = catalog.decode(scene.reference_ids[0])
        validate_rgb(reference.data)
        if reference.alpha is not None:
            raise ValueError("The RGB benchmark reference cannot have native alpha")
    except Exception as exc:
        row.update(status="reference_error", reference_status="error", error=_error_text(exc))
        return None
    row.update(reference_status="eligible", target_shape=reference.data.shape, scene_score=0.0)
    return reference.data


def _read_candidate(path):
    pixels = _rust.read_delivery_tiff(path)
    return _read_candidate_fallback(path) if pixels is None else pixels


def _read_candidate_fallback(path):
    import tifffile

    with tifffile.TiffFile(path) as image:
        if len(image.pages) != 1:
            raise ValueError(f"The delivery has {len(image.pages)} TIFF pages. The benchmark requires one page.")
        page = image.pages[0]
        if page.photometric != tifffile.PHOTOMETRIC.RGB or page.axes != "YXS" or page.samplesperpixel != 3:
            photometric = getattr(page.photometric, "name", page.photometric)
            raise ValueError("The benchmark requires an interleaved TIFF with three RGB channels per pixel. "
                             f"The TIFF uses {photometric} photometric interpretation, "
                             f"{page.samplesperpixel} samples per pixel, and {page.axes} axis order.")
        orientation = page.tags.get("Orientation")
        if orientation is not None and orientation.value != 1:
            raise ValueError(f"The TIFF has orientation {orientation.value}. "
                             "The benchmark requires orientation 1 (top-left).")
        return page.asarray()


def _load_candidate(path, row, *, fingerprint=False):
    try:
        result = _read_candidate(path)
        if fingerprint:
            row["candidate_sha256"] = digest(path)
        return result
    except FileNotFoundError as exc:
        row.update(status="missing_output", error=_error_text(exc))
    except Exception as exc:
        row.update(status="invalid_output", error=str(exc))
    return None


def _validate_candidate(result, row):
    try:
        validate_rgb(result)
    except ValueError as exc:
        row.update(status="invalid_output", error=str(exc))
        return False
    return True


def _allocation_failed(error):
    torch = sys.modules.get("torch")
    exhausted = (MemoryError, _rust.DeviceOutOfMemory, getattr(torch, "OutOfMemoryError", MemoryError))
    while error is not None:
        if isinstance(error, exhausted) or (
            isinstance(error, RuntimeError) and str(error).startswith(("CUDA out of memory", "CUDA error: out of memory"))
        ):
            return True
        error = error.__cause__
    return False


def _score_delivery(result, reference, row, *, details=None, inspection=None, device: Device = "auto", attention=False):
    if not _validate_candidate(result, row):
        return
    scoring_start = perf_counter()
    try:
        if inspection is not None:
            from .inspection import Inspection

            if type(inspection) is Inspection and inspection.device in ("auto", device):
                attention = True
                inspection = None

        def generate(target):
            heatmap, row["inspection_map_sha256"] = inspection.generate(target)
            return heatmap

        def record_digest(digest):
            row["inspection_map_sha256"] = digest

        measurement = _measure_delivery(result, reference, details=details, device=device,
                                        generate=generate if inspection is not None else None,
                                        attention=attention, on_digest=record_digest)
        row.update(scene_score=measurement.score, alignment=measurement.alignment)
        if measurement.features:
            row.update(score_features=measurement.features, artifact_diagnostics=measurement.diagnostics)
        return measurement.explanation
    except Exception as exc:
        row.update(status="scoring_error", reference_status="error", scene_score=None, error=_error_text(exc))
        return
    finally:
        row["scoring_seconds"] = perf_counter() - scoring_start


def score_saved_output(catalog, scene, path, *, details: ScoreDetails | None = None, attention=True, inspection=None,
                       device: Device = "auto"):
    """Compare a canonical TIFF with the scene's required RGB ground truth.

    A custom inspection provider can supply a precomputed heatmap and its digest.
    """
    _validate_details(details)
    _require_benchmark_reference(scene)
    if not attention or not inspection:
        inspection = None
    row = {"status": "ok", "scene_score": None}
    explanation = None
    reference = _reference(catalog, scene, row)
    if reference is not None:
        result = _load_candidate(path, row)
        if result is not None:
            explanation = _score_delivery(result, reference, row, details=details, inspection=inspection,
                                          device=device, attention=attention)
    profile = INSPECTION_SCORE_VERSION if attention else SCORE_VERSION
    response = {"profile": profile, "score": row["scene_score"], "status": row["status"]}
    for field in ("error", "alignment", "inspection_map_sha256"):
        if field in row:
            response[field] = row[field]
    if explanation is not None:
        response["explanation"] = _scene_explanation(scene, row, explanation)
    return response


def _scene_explanation(scene, row, explanation):
    result = {**explanation, "scene_id": scene.id, "reference_id": scene.reference_ids[0]}
    if "inspection_map_sha256" in row:
        result["inspection_map_sha256"] = row["inspection_map_sha256"]
    return result


def write_score_explanation(path, explanation):
    """Write a complete explanation without exposing a partially written export."""
    temporary = path.with_suffix(".tmp")
    try:
        path.parent.mkdir(parents=True, exist_ok=True)
        with temporary.open("w", encoding="utf-8") as stream:
            json.dump(explanation, stream, separators=(",", ":"), allow_nan=False)
            stream.write("\n")
        temporary.replace(path)
    finally:
        with suppress(OSError):
            temporary.unlink(missing_ok=True)


@dataclass
class _PreparedScene:
    scene: Scene
    row: dict
    result: np.ndarray
    reference: np.ndarray

    @property
    def pixels(self):
        return self.reference.shape[0] * self.reference.shape[1]


def _host_available_memory():
    # Linux counts reclaimable page cache in MemAvailable.
    try:
        for line in Path("/proc/meminfo").read_text().splitlines():
            if line.startswith("MemAvailable:"):
                amount, unit = line.removeprefix("MemAvailable:").split()
                if unit == "kB" and (available := int(amount)) >= 0:
                    return available * 1024
                break
    except (OSError, ValueError):
        pass
    return os.sysconf("SC_AVPHYS_PAGES") * os.sysconf("SC_PAGE_SIZE")


def _host_replay_fits():
    try:
        free = _host_available_memory()
        paths = Path("/proc/self/cgroup").read_text().splitlines()
        if len(paths) != 1 or not paths[0].startswith("0::/"):
            return False
        root = Path("/sys/fs/cgroup")
        group = (root / paths[0][4:]).resolve()
        if not group.is_relative_to(root) or not group.is_dir():
            return False
        for parent in (group, *group.parents):
            with suppress(FileNotFoundError):
                limit = (parent / "memory.max").read_text().strip()
                if limit != "max":
                    free = min(free, int(limit) - int((parent / "memory.current").read_text()))
            if parent == root:
                break
    except (AttributeError, OSError, ValueError):
        return False
    # Allow room for two decoded deliveries, projection and pooling working
    # sets, and temporary arrays while decoding the next scene.
    return free >= 8 * 1024 ** 3


def _cuda_replay_fits(batch, device):
    if any(not _MIN_PARALLEL_PIXELS <= scene.pixels <= _MAX_PARALLEL_PIXELS
           or scene.result.size > _MAX_PARALLEL_PIXELS * 3 for scene in batch):
        return False
    if not _host_replay_fits():
        return False
    free = _rust.benchmark_cuda_memory(device)
    if free is None:
        return False
    # Allow for a cold Inspection blend plus full-resolution maps for both
    # deliveries, including support masks and CUDA allocator headroom.
    pixels = sum(scene.pixels for scene in batch)
    if len(batch) == 1:
        pixels += _MAX_PARALLEL_PIXELS
    return free >= 6 * 1024 ** 3 + 128 * pixels


_save_scene = _rust.save_benchmark_scene
_score = _rust.benchmark_score


def bench(argv=None):
    """Run a benchmark with command-line arguments and return its exit status."""
    parser = argparse.ArgumentParser(prog="bokehforge bench", description=__doc__)
    source = parser.add_argument_group("Stack source")
    candidate = source.add_mutually_exclusive_group(required=True)
    candidate.add_argument("--algorithm", help="mean or an importable Python module:callable")
    candidate.add_argument("--candidate-dir", type=Path, help="Evaluate existing canonical TIFFs by scene ID")
    selection = parser.add_argument_group("Scene selection")
    selection.add_argument("--root", type=Path,
                           help="Dataset directory (default: the shared BokehForge data location)")
    selection.add_argument("--dataset", action="append", help="Dataset ID, repeatable. Defaults to configured downloads")
    selection.add_argument("--scene", action="append",
                           help="Exact catalog scene ID, repeatable. Every selected scene requires RGB ground truth")
    selection.add_argument("--config", type=Path, default=DEFAULT_CONFIG, help="Dataset source configuration")
    selection.add_argument("--lock", type=Path, help="Optional source snapshot instead of download manifests")
    score_options = parser.add_argument_group("Scoring")
    score_options.add_argument("--no-attention", action="store_true", help="Disable inspection heatmaps and damage weighting")
    score_options.add_argument("--device", choices=DEVICES, default="auto",
                               help="Damage scoring backend. Auto (default) prefers CUDA, then MPS, then CPU. "
                                    "GPU requires CUDA or MPS.")
    score_options.add_argument(
        "--decode-workers", type=_positive_int, default=1, help="Concurrent focal decodes per scene (default: 1)",
    )
    report_options = parser.add_argument_group("Report")
    report_options.add_argument("--output", required=True, type=Path, help="New directory for the report and optional images")
    report_options.add_argument(
        "--explain-score", action="store_true",
        help="Save all score diagnostics, including scoring tiles and the attention map when enabled",
    )
    report_options.add_argument("--save-images", action="store_true", help="Save full-resolution float32 TIFFs")
    report_options.add_argument("--progress", action="store_true", help="Write progress to stderr")
    args = parser.parse_args(argv)
    return _rust.benchmark(args, sys.modules[__name__])

def main(argv=None):
    return bench(argv)
