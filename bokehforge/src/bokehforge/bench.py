"""Run a stacker against a reproducible cohort from the local SceneCatalog."""

import argparse
from collections import Counter
from concurrent.futures import ThreadPoolExecutor
from contextlib import closing, redirect_stdout, suppress
from dataclasses import dataclass
from hashlib import sha256
from importlib import import_module
from itertools import chain
from importlib.metadata import version
import json
import os
from pathlib import Path
from statistics import StatisticsError
import sys
from time import perf_counter

import numpy as np
import tifffile

from . import _rust
from ._device import DEVICES, Device, damage_backend
from .alignment import ALIGNMENT_VERSION, MAX_ROTATION_DEGREES
from .data._canonical import encode_tiff
from .data._files import digest
from .data._config import DEFAULT_CONFIG, data_root
from .data.download import save_json
from .data.lazy import Scene, SceneCatalog
from .inspection import INSPECTION_MAP_VERSION, Inspection, inspection_for
from .scoring import (
    FAMILY_WEIGHTS, INSPECTION_MAX_MULTIPLIER, INSPECTION_MIX, INSPECTION_SCORE_VERSION,
    SCORE_DAMAGE_SCALE, SCORE_EXPLANATION_VERSION, SCORE_VERSION,
    ScoreDetails, _measure_delivery, _validate_details, validate_rgb,
)
from .stacking import StackFrame, StackInput

_RGB_REFERENCES = {"rendered_all_in_focus", "synthetic_source_image_reference"}
_MIN_PARALLEL_PIXELS = 1_000_000
_MAX_PARALLEL_PIXELS = 20_000_000
_RETRY_SCORING = object()


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


def _hash(value):
    return sha256(json.dumps(value, sort_keys=True, separators=(",", ":")).encode()).hexdigest()


def _describe_scene(catalog, scene):
    metadata = scene.metadata
    inputs = [
        (frame.id, frame.source_sha256, frame.asset_sha256, frame.page)
        for frame in map(catalog.frame, scene.frame_ids + scene.reference_ids)
    ]
    return {
        "scene_id": scene.id,
        "dataset_id": metadata["dataset_id"],
        "leakage_group_id": metadata["leakage_group_id"],
        "frame_count": len(scene.frame_ids),
        "reference_type": metadata["reference_type"],
        "reference_ids": scene.reference_ids,
        "input_sha256": _hash((metadata["recipe_sha256"], inputs)),
    }


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


def _run_stacker(catalog, scene, row, algorithm, shape, workers):
    with closing(_read_frames(catalog, scene, row, workers)) as frames:
        try:
            with redirect_stdout(sys.stderr):
                result = algorithm(StackInput(scene.id, scene.frame_ids, shape, frames))
        except _InputError:
            raise
        except Exception as exc:
            row.update(status="algorithm_error", error=_error_text(exc))
            result = None
        # An algorithm can legitimately stop early. Verify the selected scene's
        # focal files to keep evaluator failure accounting comparable across algorithms.
        for _ in frames:
            pass
        if row["frames_read"] != row["frame_count"]:
            raise _InputError("The focal frame stream ended before the scene was fully decoded")
        return result


def _read_candidate(path):
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
    exhausted = (MemoryError, getattr(torch, "OutOfMemoryError", MemoryError))
    while error is not None:
        if isinstance(error, exhausted) or (
            isinstance(error, RuntimeError) and str(error).startswith(("CUDA out of memory", "CUDA error: out of memory"))
        ):
            return True
        error = error.__cause__
    return False


def _score_delivery(result, reference, row, *, details=None, inspection=None, device: Device = "auto"):
    if _validate_candidate(result, row):
        return _score_validated_delivery(result, reference, row, details=details, inspection=inspection, device=device)


def _score_validated_delivery(result, reference, row, *, details, inspection, device, in_batch=False):
    scoring_start = perf_counter()
    try:
        def generate(target):
            heatmap, row["inspection_map_sha256"] = inspection.generate(target)
            return heatmap

        measurement = _measure_delivery(result, reference, details=details, device=device,
                                        generate=generate if inspection is not None else None,
                                        retry_on_oom=not in_batch)
        row.update(scene_score=measurement.score, alignment=measurement.alignment)
        if measurement.features:
            row.update(score_features=measurement.features, artifact_diagnostics=measurement.diagnostics)
        return measurement.explanation
    except Exception as exc:
        if in_batch and _allocation_failed(exc):
            return _RETRY_SCORING
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
    if not attention:
        inspection = None
    elif not inspection or (device == "mps" and type(inspection) is Inspection):
        inspection = inspection_for(device)
    row = {"status": "ok", "scene_score": None}
    explanation = None
    reference = _reference(catalog, scene, row)
    if reference is not None:
        result = _load_candidate(path, row)
        if result is not None:
            explanation = _score_delivery(result, reference, row, details=details, inspection=inspection, device=device)
    profile = SCORE_VERSION if inspection is None else INSPECTION_SCORE_VERSION
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


def _save_explanation(output, scene, row, explanation):
    path = output / "score-explanations" / (scene.id + ".json")
    try:
        write_score_explanation(path, _scene_explanation(scene, row, explanation))
    except (OSError, ValueError) as exc:
        row["score_explanation_error"] = _error_text(exc)
        if row["status"] == "ok":
            row["status"] = "score_explanation_error"
    else:
        row["score_explanation_path"] = path.relative_to(output).as_posix()


@dataclass
class _PreparedScene:
    scene: Scene
    row: dict
    result: np.ndarray
    reference: np.ndarray

    @property
    def pixels(self):
        return self.reference.shape[0] * self.reference.shape[1]


def _prepare_scene(catalog, scene, row, algorithm, args):
    row.update(frames_read=0, scene_score=None, status="ok")
    reference = _reference(catalog, scene, row)
    if reference is None:
        return
    shape = reference.shape
    start = perf_counter()
    try:
        if args.candidate_dir is not None:
            path = args.candidate_dir / (scene.id + ".tif")
            result = _load_candidate(path, row, fingerprint=True)
            if result is None:
                return
        else:
            result = _run_stacker(catalog, scene, row, algorithm, shape, args.decode_workers)
    except _InputError as exc:
        row.update(status="input_error", reference_status="error", scene_score=None, error=str(exc))
        return
    finally:
        row["seconds"] = perf_counter() - start

    if row["status"] == "ok" and _validate_candidate(result, row):
        return _PreparedScene(scene, row, result, reference)


def _save_scene(prepared, args, explanation):
    scene, row, result = prepared.scene, prepared.row, prepared.result
    if row["status"] != "ok":
        return
    if args.save_images:
        image_path = args.output / "images" / (scene.id + ".tif")
        try:
            image_path.parent.mkdir(parents=True, exist_ok=True)
            encode_tiff(image_path, result)
        except Exception as exc:
            image_path.unlink(missing_ok=True)
            row.update(status="image_error", error=_error_text(exc))
        else:
            row["image"] = image_path.relative_to(args.output).as_posix()
    if explanation is not None:
        _save_explanation(args.output, scene, row, explanation)


def _score_prepared_scene(prepared, row, args, inspection, *, in_batch=False):
    return _score_validated_delivery(prepared.result, prepared.reference, row, details="full" if args.explain_score else None,
                                     inspection=inspection, device=args.device, in_batch=in_batch)


def _run_scene(catalog, scene, row, algorithm, args, inspection):
    if (prepared := _prepare_scene(catalog, scene, row, algorithm, args)) is not None:
        with redirect_stdout(sys.stderr):
            explanation = _score_prepared_scene(prepared, row, args, inspection)
        _save_scene(prepared, args, explanation)


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
    try:
        if damage_backend(device) != "cuda":
            return False
        import torch

        cuda = torch.cuda
        # The replay workers use the CLI's default device and stream.
        if cuda.current_device() != 0 or cuda.current_stream() != cuda.default_stream():
            return False
        if torch.is_autocast_enabled("cuda"):
            return False
        free, _ = cuda.mem_get_info()
    except (ImportError, OSError, RuntimeError):
        return False
    # Allow for a cold Inspection blend plus full-resolution maps for both
    # deliveries, including support masks and CUDA allocator headroom.
    pixels = sum(scene.pixels for scene in batch)
    if len(batch) == 1:
        pixels += _MAX_PARALLEL_PIXELS
    return free >= 6 * 1024 ** 3 + 128 * pixels


def _score_replay_batch(batch, args, inspection, executor):
    def attempt(prepared):
        row = prepared.row.copy()
        return row, _score_prepared_scene(prepared, row, args, inspection, in_batch=True)

    if len(batch) == 2 and _cuda_replay_fits(batch, args.device):
        # Drain both attempts before retrying a CUDA or host allocation failure.
        outcomes = list(executor.map(attempt, batch))
    else:
        outcomes = [(None, _RETRY_SCORING)] * len(batch)
    for prepared, (row, explanation) in zip(batch, outcomes, strict=True):
        if explanation is _RETRY_SCORING:
            explanation = _score_prepared_scene(prepared, prepared.row, args, inspection)
        else:
            prepared.row.update(row)
        _save_scene(prepared, args, explanation)


def _run_replays(catalog, entries, args, inspection):
    batch = []
    with redirect_stdout(sys.stderr), ThreadPoolExecutor(max_workers=2, thread_name_prefix="bokehforge-replay") as executor:
        for scene, row in entries:
            prepared = _prepare_scene(catalog, scene, row, None, args)
            if prepared is not None:
                batch.append(prepared)
                if len(batch) == 2 or not _cuda_replay_fits(batch, args.device):
                    _score_replay_batch(batch, args, inspection, executor)
                    batch.clear()
                del prepared
        if batch:
            _score_replay_batch(batch, args, inspection, executor)


def _score(rows, *, explanation=None):
    entries = [(row["dataset_id"], row["leakage_group_id"], row["scene_score"])
               for row in rows if row["scene_score"] is not None]
    unavailable = any(row["reference_status"] == "error" for row in rows)
    score, group_count, datasets = _rust.cohort_score(entries, unavailable, explanation is not None)
    if score is None and not unavailable:
        raise StatisticsError("fmean requires at least one data point")
    if explanation is not None:
        explanation.update(score=score, status="evaluation_error" if unavailable else "ok",
                           datasets={
                               dataset: {"score": mean, "weight": weight, "groups": {
                                   group: {"score": mean, "weight": weight, "scene_count": count}
                                   for group, mean, weight, count in groups
                               }} for dataset, mean, weight, groups in datasets
                           })
    return score, group_count


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
    if args.device in ("cpu", "mps") and not args.no_attention:
        parser.error(f"--device {args.device} requires --no-attention because the Inspection profile requires CUDA")
    if args.output.exists():
        raise ValueError(f"Output directory already exists: {args.output}")
    root = args.root.expanduser().resolve() if args.root is not None else data_root(args.config)

    start = perf_counter()
    catalog = SceneCatalog(root, datasets=args.dataset, config=args.config, lock=args.lock)
    selected = set(args.scene or catalog.scene_ids)
    if unknown := selected.difference(catalog.scene_ids):
        raise ValueError(f"Unknown scene IDs: {sorted(unknown)}")
    scenes = [catalog.scene(scene_id) for scene_id in catalog.scene_ids if scene_id in selected]
    if not scenes:
        raise ValueError("No benchmark scenes in the selected datasets")
    for scene in scenes:
        _require_benchmark_reference(scene)

    if args.candidate_dir is not None:
        if not args.candidate_dir.is_dir():
            raise ValueError(f"Candidate directory does not exist: {args.candidate_dir}")
        algorithm, implementation = None, {"candidate_dir": str(args.candidate_dir.resolve())}
    else:
        algorithm, implementation = _load_algorithm(args.algorithm)
    rows = [_describe_scene(catalog, scene) for scene in scenes]
    cohort_hash = _hash(rows)
    inspection = None if args.no_attention else Inspection()
    args.output.mkdir(parents=True)
    if args.progress:
        print(f"Benchmarking {len(scenes)} scenes from {root.resolve()}", file=sys.stderr, flush=True)

    def entries():
        last_dataset = None
        for index, (scene, row) in enumerate(zip(scenes, rows, strict=True), 1):
            dataset = row["dataset_id"]
            if args.progress and (dataset != last_dataset or index % 25 == 0):
                print(f"{index}/{len(scenes)} {scene.id} ({row['frame_count']} frames)", file=sys.stderr, flush=True)
            last_dataset = dataset
            yield scene, row

    if (args.candidate_dir is not None and inspection is not None and len(scenes) > 1
            and not args.explain_score):
        _run_replays(catalog, entries(), args, inspection)
    else:
        for scene, row in entries():
            _run_scene(catalog, scene, row, algorithm, args, inspection)
    aggregation = {} if args.explain_score else None
    score, scored_groups = _score(rows, explanation=aggregation)
    statuses = Counter(row["status"] for row in rows)
    references = Counter(row["reference_status"] for row in rows)
    scoring = {"family_weights": FAMILY_WEIGHTS, "damage_scale": SCORE_DAMAGE_SCALE,
               "runtime": {name: version(name) for name in ("bokehforge", "numpy", "scipy", "tifffile")},
               "python": sys.version.split()[0],
               "aggregation": "equal datasets; equal leakage groups within each dataset"}
    scoring["alignment"] = {"schema_version": ALIGNMENT_VERSION, "max_rotation_degrees": MAX_ROTATION_DEGREES,
                            "coverage_penalty": "minimum of relative intersection area and contributing delivery samples"}
    profile = SCORE_VERSION
    if inspection is not None:
        profile = INSPECTION_SCORE_VERSION
        scoring["inspection"] = {"map_version": INSPECTION_MAP_VERSION, "max_mix": INSPECTION_MIX,
                                 "max_multiplier": INSPECTION_MAX_MULTIPLIER}
    report = {
        "schema_version": 4, "score": score, "score_version": profile,
        "scoring": scoring,
        "scored_groups": scored_groups, "cohort_sha256": cohort_hash, "algorithm": implementation,
        "data_root": str(root.resolve()), "decode_workers": args.decode_workers, "seconds": perf_counter() - start,
        "scene_status_counts": statuses, "reference_status_counts": references,
        "quarantine": catalog.quarantine, "scenes": rows,
    }
    if aggregation is not None:
        report["score_explanation_version"] = SCORE_EXPLANATION_VERSION
        report["score_aggregation"] = aggregation
    save_json(args.output / "report.json", report)
    print("unscored" if score is None else f"{score:.6f}")
    return int(any(status != "ok" for status in statuses))


def main(argv=None):
    return bench(argv)
