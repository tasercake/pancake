"""Run a stacker against a reproducible cohort from the local SceneCatalog."""

import argparse
from collections import Counter, defaultdict
from contextlib import closing, redirect_stdout
from hashlib import sha256
from importlib import import_module
from itertools import chain
from importlib.metadata import version
import json
from pathlib import Path
from statistics import fmean
import sys
from time import perf_counter

import numpy as np
import tifffile

from .data._canonical import encode_tiff
from .data._files import digest
from .data.download import CONFIG_DIRECTORY, save_json
from .data.lazy import SceneCatalog
from .scoring import FAMILY_WEIGHTS, SCORE_DAMAGE_SCALE, SCORE_VERSION, VIEW_WEIGHTS, measure, scene_score, validate_rgb
from .stacking import StackFrame, StackInput

_RGB_REFERENCES = {"rendered_all_in_focus", "synthetic_source_image_reference"}


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


def _reference(catalog, scene, row):
    if row["reference_type"] not in _RGB_REFERENCES or not scene.reference_ids:
        row.update(status="unscored", reference_status="no_eligible_reference")
        return None
    try:
        if len(scene.reference_ids) != 1:
            raise ValueError("Expected exactly one RGB reference")
        reference = catalog.decode(scene.reference_ids[0])
        validate_rgb(reference.data)
        if reference.alpha is not None:
            raise ValueError("The RGB reference profile cannot score native alpha")
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
            raise ValueError("Expected a single-page TIFF")
        page = image.pages[0]
        orientation = page.tags.get("Orientation")
        if (page.photometric != tifffile.PHOTOMETRIC.RGB or page.axes != "YXS"
                or page.samplesperpixel != 3 or (orientation is not None and orientation.value != 1)):
            raise ValueError("Expected orientation-1, interleaved RGB TIFF")
        return page.asarray()


def _run_scene(catalog, scene, row, algorithm, args):
    row.update(frames_read=0, scene_score=None, status="ok")
    reference = _reference(catalog, scene, row)
    if reference is None:
        return
    shape = reference.shape
    start = perf_counter()
    try:
        if args.candidate_dir is not None:
            path = args.candidate_dir / (scene.id + ".tif")
            try:
                result = _read_candidate(path)
                row["candidate_sha256"] = digest(path)
            except FileNotFoundError as exc:
                row.update(status="missing_output", error=_error_text(exc))
                return
            except Exception as exc:
                row.update(status="invalid_output", error=_error_text(exc))
                return
        else:
            result = _run_stacker(catalog, scene, row, algorithm, shape, args.decode_workers)
    except _InputError as exc:
        row.update(status="input_error", reference_status="error", scene_score=None, error=str(exc))
        return
    finally:
        row["seconds"] = perf_counter() - start

    if row["status"] != "ok":
        return
    try:
        validate_rgb(result, shape)
    except ValueError as exc:
        row.update(status="invalid_output", error=str(exc))
        return

    scoring_start = perf_counter()
    try:
        measurement = measure(result, reference)
        row.update(scene_score=scene_score(measurement), score_features=measurement.features,
                   artifact_diagnostics=measurement.diagnostics)
    except Exception as exc:
        row.update(status="scoring_error", reference_status="error", scene_score=None, error=_error_text(exc))
        return
    finally:
        row["scoring_seconds"] = perf_counter() - scoring_start
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


def _score(rows):
    datasets = defaultdict(lambda: defaultdict(list))
    groups = set()
    for row in rows:
        if row["scene_score"] is not None:
            group = row["leakage_group_id"]
            datasets[row["dataset_id"]][group].append(row["scene_score"])
            groups.add(group)
    unavailable = any(row["reference_status"] == "error" for row in rows)
    dataset_means = (fmean(map(fmean, dataset.values())) for dataset in datasets.values())
    return (fmean(dataset_means) if datasets and not unavailable else None), len(groups)


def main(argv=None):
    parser = argparse.ArgumentParser(prog="bokehforge bench", description=__doc__)
    candidate = parser.add_mutually_exclusive_group(required=True)
    candidate.add_argument("--algorithm", help="mean or an importable Python module:callable")
    candidate.add_argument("--candidate-dir", type=Path, help="Evaluate existing canonical TIFFs by scene ID")
    parser.add_argument("--output", required=True, type=Path, help="New directory for the report and optional images")
    parser.add_argument("--root", type=Path, help="Dataset directory used by data download")
    parser.add_argument("--dataset", action="append", help="Dataset ID, repeatable. Default: all downloaded datasets")
    parser.add_argument("--scene", action="append", help="Exact catalog scene ID, repeatable. Default: all selected scenes")
    parser.add_argument("--config", type=Path, help="Optional catalog policy overrides")
    parser.add_argument("--lock", type=Path, help="Optional source snapshot instead of download manifests")
    parser.add_argument("--decode-workers", type=_positive_int, default=1, help="Concurrent focal decodes per scene (default: 1)")
    parser.add_argument("--save-images", action="store_true", help="Save full-resolution float32 TIFFs")
    parser.add_argument("--progress", action="store_true", help="Write progress to stderr")
    args = parser.parse_args(argv)
    if args.output.exists():
        raise ValueError(f"Output directory already exists: {args.output}")
    root = args.root
    if root is None:
        config = args.config or CONFIG_DIRECTORY / "sources.json"
        defaults = json.loads(config.read_text()).get("defaults", {}) if config.is_file() else {}
        root = config.parent / defaults["data_root"] if "data_root" in defaults else Path.cwd() / "datasets"

    if args.candidate_dir is not None:
        if not args.candidate_dir.is_dir():
            raise ValueError(f"Candidate directory does not exist: {args.candidate_dir}")
        algorithm, implementation = None, {"candidate_dir": str(args.candidate_dir.resolve())}
    else:
        algorithm, implementation = _load_algorithm(args.algorithm)
    start = perf_counter()
    catalog = SceneCatalog(root, datasets=args.dataset, config=args.config, lock=args.lock)
    selected = set(args.scene or catalog.scene_ids)
    if unknown := selected.difference(catalog.scene_ids):
        raise ValueError(f"Unknown scene IDs: {sorted(unknown)}")
    scenes = [catalog.scene(scene_id) for scene_id in catalog.scene_ids if scene_id in selected]
    if not scenes:
        raise ValueError("No benchmark scenes in the selected datasets")
    rows = [_describe_scene(catalog, scene) for scene in scenes]
    cohort_hash = _hash(rows)
    args.output.mkdir(parents=True)
    if args.progress:
        print(f"Benchmarking {len(scenes)} scenes from {root.resolve()}", file=sys.stderr, flush=True)
    last_dataset = None
    for index, (scene, row) in enumerate(zip(scenes, rows, strict=True), 1):
        dataset = row["dataset_id"]
        if args.progress and (dataset != last_dataset or index % 25 == 0):
            print(f"{index}/{len(scenes)} {scene.id} ({row['frame_count']} frames)", file=sys.stderr, flush=True)
        last_dataset = dataset
        _run_scene(catalog, scene, row, algorithm, args)
    score, scored_groups = _score(rows)
    statuses = Counter(row["status"] for row in rows)
    references = Counter(row["reference_status"] for row in rows)
    save_json(args.output / "report.json", {
        "schema_version": 2, "score": score, "score_version": SCORE_VERSION,
        "scoring": {"family_weights": FAMILY_WEIGHTS, "view_weights": VIEW_WEIGHTS,
                    "damage_scale": SCORE_DAMAGE_SCALE,
                    "runtime": {name: version(name) for name in ("bokehforge", "numpy", "scipy", "tifffile")},
                    "python": sys.version.split()[0],
                    "aggregation": "equal datasets; equal leakage groups within each dataset"},
        "scored_groups": scored_groups, "cohort_sha256": cohort_hash, "algorithm": implementation,
        "data_root": str(root.resolve()), "decode_workers": args.decode_workers, "seconds": perf_counter() - start,
        "scene_status_counts": statuses, "reference_status_counts": references,
        "quarantine": catalog.quarantine, "scenes": rows,
    })
    print("unscored" if score is None else f"{score:.6f}")
    return int(any(status not in {"ok", "unscored"} for status in statuses))
