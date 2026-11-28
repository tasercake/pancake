"""Measure BokehForge's bench scoring stage and compare results across checkouts."""

import argparse
from base64 import b64decode, b64encode
from hashlib import sha256
from inspect import signature
from io import BytesIO
import json
import os
from pathlib import Path
from statistics import fmean, mean, median
import sys
from time import perf_counter
from unittest.mock import patch
from zipfile import BadZipFile, ZipFile
from zlib import compress, decompress
from zlib import error as ZlibError


_DETAILS = "summary"
_SCORE_FIELDS = {"score", "scene_score", "overlap_score", "score_contribution"}
_FAMILY_VALUES = {"burden", "mean", "tail", "worst", "affected_fraction", "weighted_damage",
                  "value", "contribution", "cutoff"}
_DIAGNOSTIC_VALUES = {"mean_delta_lightness", "mean_delta_lab_a", "mean_delta_lab_b",
                      "global_exposure_delta_stops", "linear_rgb_rmse"}
_TOLERANCES = {"score": .01, "measurement": .0001}
_ALIGNMENT_TRACE_FIELDS = {"stages", "candidates", "coarse_rgb_contrast", "low_information_appearance_delta"}
_PLACEMENT_GEOMETRY = {"candidate_to_reference_xy", "offset_yx", "rotation_degrees", "scale",
                       "reference_overlap_polygon_xy", "geometric_coverage", "coverage", "resolution_cap", "weight"}


def _json_sha256(value):
    payload = json.dumps(value, sort_keys=True, separators=(",", ":"), allow_nan=False)
    return sha256(payload.encode()).hexdigest()


def _image_sha256(pixels):
    return sha256(memoryview(pixels.astype("<f4", copy=False))).hexdigest()


def _numeric_category(path):
    field = path[-1]
    if field in _SCORE_FIELDS:
        return "score"
    if (field in _DIAGNOSTIC_VALUES or field == "weighted_damage"
            or "score_features" in path
            or ("families" in path and field in _FAMILY_VALUES)
            or ("tile_grids" in path and "values" in path)):
        return "measurement"
    return None


def _is_alignment_trace(path):
    return any(left == "alignment" and right in _ALIGNMENT_TRACE_FIELDS for left, right in zip(path, path[1:]))


def _is_geometry(path):
    if path[-2:] in (("alignment", "confidence"), ("explanation", "confidence")):
        return True
    if "placements" in path:
        index = path.index("placements")
        return len(path) > index + 2 and path[index + 2] in _PLACEMENT_GEOMETRY
    return False


class ResultComparison:
    def __init__(self, *, allow_alignment_trace_change=False, allow_generated_map_change=False,
                 allow_native_rounding=False):
        self.mismatches = 0
        self.tolerances = dict(_TOLERANCES)
        if allow_native_rounding:
            self.tolerances["geometry"] = .0001
        self.maximum = {category: (0.0, ()) for category in self.tolerances}
        self.allow_alignment_trace_change = allow_alignment_trace_change
        self.allow_generated_map_change = allow_generated_map_change
        self.allow_native_rounding = allow_native_rounding
        self.trace_changes = 0
        self.max_trace_change = (0.0, ())
        self.trace_candidate_counts = []
        self.trace_sample_changes = 0

    def _numeric_category(self, path):
        if self.allow_native_rounding and _is_geometry(path):
            return "geometry"
        if self.allow_generated_map_change or self.allow_native_rounding:
            if (path[-1] == "inspection_mix"
                    or "attention" in path and (path[-1] in {"mean", "mix"} or "damage_multiplier_range" in path)):
                return "measurement"
        return _numeric_category(path)

    def _mismatch(self, path, reason):
        self.mismatches += 1
        if self.mismatches <= 20:
            print(f"{'.'.join(map(str, path))} {reason}.", file=sys.stderr)

    def check(self, left, right, *path):
        if type(left) is not type(right) and not (
            {type(left), type(right)} == {int, float} and self._numeric_category(path) in {"score", "geometry"}
        ):
            self._mismatch(path, f"has different types ({type(left).__name__}, {type(right).__name__})")
        elif isinstance(left, dict):
            if left.keys() != right.keys():
                self._mismatch(path, f"has different keys ({sorted(left.keys() ^ right.keys())})")
            for key in sorted(left.keys() & right.keys()):
                self.check(left[key], right[key], *path, key)
        elif isinstance(left, list):
            if len(left) != len(right):
                reason = f"has different lengths ({len(left)}, {len(right)})"
                if self.allow_alignment_trace_change and path[-2:] == ("alignment", "candidates"):
                    self.trace_candidate_counts.append((path, reason))
                    selected = [sum(row["decision"] == "selected" for row in rows) for rows in (left, right)]
                    if selected[0] != selected[1]:
                        self._mismatch(path, f"has different numbers of selected candidates ({selected[0]}, {selected[1]})")
                else:
                    self._mismatch(path, reason)
            for index, (first, second) in enumerate(zip(left, right)):
                self.check(first, second, *path, index)
        elif type(left) in (int, float) and (category := self._numeric_category(path)):
            difference = abs(left - right)
            tolerance = self.tolerances[category]
            if difference > self.maximum[category][0]:
                self.maximum[category] = (difference, path)
            if not difference <= tolerance:
                self._mismatch(path, f"differs by {difference:.9g} (limit {tolerance:g})")
        elif left != right:
            if self.allow_alignment_trace_change and type(left) is float and _is_alignment_trace(path):
                self.trace_changes += 1
                difference = abs(left - right)
                if difference > self.max_trace_change[0]:
                    self.max_trace_change = difference, path
            elif (self.allow_alignment_trace_change and type(left) is int and "regions" in path
                  and path[-1] == 1 and _is_alignment_trace(path)):
                self.trace_sample_changes += 1
            elif self.allow_native_rounding and path[-2:] == ("attention", "data"):
                if decompress(b64decode(left, validate=True)) != decompress(b64decode(right, validate=True)):
                    self._mismatch(path, "contains different decoded attention pixels")
            elif not (self.allow_generated_map_change and path[-1] == "inspection_map_sha256"):
                self._mismatch(path, f"differs ({repr(left)[:120]} vs {repr(right)[:120]})")

    def finish(self):
        for category, (difference, path) in self.maximum.items():
            location = f" at {'.'.join(map(str, path))}" if path else ""
            print(f"Maximum absolute {category} difference {difference:.9g}{location} "
                  f"(limit {self.tolerances[category]:g}).")
        if self.allow_alignment_trace_change:
            for path, reason in self.trace_candidate_counts:
                print(f"{'.'.join(map(str, path))} {reason}.")
            difference, path = self.max_trace_change
            location = f" at {'.'.join(map(str, path))}" if path else ""
            print(f"Alignment trace changed in {self.trace_changes} floating-point values and "
                  f"{self.trace_sample_changes} regional sample counts. Largest absolute floating-point "
                  f"difference {difference:.9g}{location}.")
        if self.mismatches:
            print(f"{self.mismatches} parity mismatches.", file=sys.stderr)
        return int(bool(self.mismatches))


def _prepare_images(png, size):
    import numpy as np
    from PIL import Image
    from scipy.ndimage import gaussian_filter

    with Image.open(BytesIO(png)) as source:
        plane = source.convert("RGB").resize((size, size), Image.Resampling.BICUBIC)
    srgb = np.asarray(plane, dtype=np.float32) / 255
    reference = np.where(srgb <= .04045, srgb / 12.92, ((srgb + .055) / 1.055) ** 2.4).astype("f4")
    candidate = reference.copy()
    blur_region = np.s_[size // 4:size // 2, size // 8:size // 2, :]
    candidate[blur_region] = gaussian_filter(candidate[blur_region], (2., 2., 0))
    candidate[..., 0] += np.float32(.005)
    candidate[..., 1] *= np.float32(.992)
    return candidate, reference


def _require_local_models():
    raise RuntimeError("Prepare the local Inspection model assets before timing. Downloads are disabled.")


def _scored(sample):
    return (sample["status"] == "ok" and sample["reference_status"] == "eligible"
            and sample["scoring_seconds"] is not None)


def _capture_inspection_map(inspection, reference, digest):
    import numpy as np

    raw = inspection.cached_map(digest)
    if raw is None:
        raw, observed = inspection.preview_map(reference)
        if observed != digest:
            raise ValueError("The retained model-grid Inspection digest differs from the scoring result.")
    pixels = np.ascontiguousarray(raw, dtype="<f4")
    if _image_sha256(pixels) != digest:
        raise ValueError("The retained model-grid Inspection pixels differ from the scoring digest.")
    return {"size_wh": list(pixels.shape[::-1]), "data": b64encode(compress(pixels.tobytes(), 1)).decode("ascii")}


def _inspection_pixels(sample):
    import numpy as np

    if "inspection_map" not in sample:
        raise ValueError("Generated map comparison requires reports generated with --include-inspection-maps.")
    metadata = sample["inspection_map"]
    width, height = metadata["size_wh"]
    payload = decompress(b64decode(metadata["data"], validate=True))
    if min(width, height) < 1 or len(payload) != width * height * 4:
        raise ValueError(f"{sample['name']} has an invalid Inspection map size.")
    if sha256(payload).hexdigest() != sample["inspection_map_sha256"]:
        raise ValueError(f"{sample['name']} has an invalid Inspection map digest.")
    pixels = np.frombuffer(payload, dtype="<f4").reshape(height, width)
    if not np.isfinite(pixels).all() or pixels.min() < 0 or pixels.max() > 1:
        raise ValueError(f"{sample['name']} has Inspection map pixels outside [0, 1].")
    return pixels


def _compare_inspection_maps(left, right):
    import numpy as np

    left_pixels, right_pixels = (_inspection_pixels(sample) for sample in (left, right))
    if left_pixels.shape != right_pixels.shape:
        raise ValueError(f"{left['name']} has different Inspection map dimensions.")
    difference = np.abs(left_pixels.astype(np.float64) - right_pixels)
    count = max(1, (difference.size + 99) // 100)
    first, second = (np.argsort(-pixels.ravel(), kind="stable")[:count]
                     for pixels in (left_pixels, right_pixels))
    overlap = np.intersect1d(first, second, assume_unique=True).size / count
    print(f"{left['name']} model-grid Inspection MAE {difference.mean():.9g}, "
          f"max {difference.max():.9g}, top-1% overlap {overlap:.3%}, "
          f"{np.count_nonzero(difference > .01)}/{difference.size} pixels differ by more than 0.01.")


def _warm_statistics(samples, field):
    if not samples or not all(_scored(sample) for sample in samples):
        return None
    values = [sample[field] for sample in samples]
    return {"mean": mean(values), "median": median(values), "min": min(values), "max": max(values)}


def _score_public(candidate, reference, row, device, attention):
    from bokehforge.scoring import measure

    try:
        measurement = measure(candidate, reference, details=_DETAILS, device=device, attention=attention)
    except Exception as error:
        row.update(status="scoring_error", reference_status="error", scene_score=None,
                   error=f"{type(error).__name__}: {error}")
        return None
    row.update(scene_score=measurement.score, alignment=measurement.alignment)
    if measurement.features:
        row.update(score_features=measurement.features, artifact_diagnostics=measurement.diagnostics)
    return measurement.explanation


def _run(args):
    from bokehforge import bench, inspection_assets
    from bokehforge._device import validate_device
    from bokehforge.inspection import inspection_for

    validate_device(args.device)
    if args.output.resolve() == args.d29_zip.resolve():
        raise ValueError("The report path must differ from the input ZIP path.")
    attention = not args.no_attention
    native_attention = attention and "attention" in signature(bench._score_delivery).parameters
    samples = []
    previous_reference = None
    with (
        ZipFile(args.d29_zip) as archive,
        patch.dict(os.environ, HF_HUB_OFFLINE="1"),
        patch.object(inspection_assets, "session", _require_local_models),
    ):
        names = sorted(name for name in archive.namelist() if name.endswith("/image_all_in_focus.png"))
        if len(names) < args.count:
            raise ValueError(f"Requested {args.count} scenes, but the ZIP has {len(names)} all-in-focus images.")
        if len(set(names)) != len(names):
            raise ValueError("The ZIP contains duplicate all-in-focus member names.")
        # Spread adjacent samples across the source groups using the original fixture stride.
        cohort = names[::len(names) // args.count][:args.count]
        for index, name in enumerate(cohort):
            png = archive.read(name)
            candidate, reference = _prepare_images(png, args.size)
            inputs = {
                "png_sha256": sha256(png).hexdigest(),
                "reference_sha256": _image_sha256(reference),
                "candidate_sha256": _image_sha256(candidate),
            }
            if inputs["reference_sha256"] == previous_reference:
                raise ValueError(f"Consecutive samples have identical reference pixels at {name}.")
            previous_reference = inputs["reference_sha256"]

            row = {"status": "ok", "reference_status": "eligible"}
            start = perf_counter()
            if args.public_measure:
                explanation = _score_public(candidate, reference, row, args.device, attention)
            else:
                options = {"attention": True} if native_attention else {
                    "inspection": inspection_for(args.device) if attention else None,
                }
                explanation = bench._score_delivery(
                    candidate, reference, row, details=_DETAILS, device=args.device, **options,
                )
            wall = perf_counter() - start
            if args.public_measure:
                scoring = wall
                if attention and row["status"] == "ok":
                    _, row["inspection_map_sha256"] = inspection_for(args.device).preview_map(reference)
            else:
                scoring = row.pop("scoring_seconds", None)
            sample = {
                "name": name,
                "inputs": inputs,
                "scoring_seconds": scoring,
                "wall_seconds": wall,
                "output_sha256": _json_sha256([row, explanation]),
                "inspection_map_sha256": row.get("inspection_map_sha256"),
                "scene_score": row.get("scene_score"),
                "status": row["status"],
                "reference_status": row["reference_status"],
                "error": row.get("error"),
            }
            if args.include_results:
                sample["result"] = {"row": row, "explanation": explanation}
            if args.include_inspection_maps and _scored(sample):
                sample["inspection_map"] = _capture_inspection_map(
                    inspection_for(args.device), reference, sample["inspection_map_sha256"],
                )
            samples.append(sample)
            phase = "warmup" if index == 0 else "warm"
            stage = f"{scoring * 1000:.1f} ms scoring" if scoring is not None else "scoring unavailable"
            print(f"{phase} {name} {sample['status']} {stage}, {wall * 1000:.1f} ms wall", flush=True)
            if sample["error"]:
                print(sample["error"], file=sys.stderr, flush=True)
            del png, candidate, reference

    warm = samples[1:]
    report = {
        "schema_version": 1,
        "fixture": "d29-local-blur-v1",
        "settings": {"count": args.count, "size": args.size, "attention": attention, "details": _DETAILS},
        "device": args.device,
        "scoring": {
            "score_version": bench.SCORE_VERSION,
            "profile": bench.INSPECTION_SCORE_VERSION if attention else bench.SCORE_VERSION,
            "explanation_version": bench.SCORE_EXPLANATION_VERSION,
            "alignment_version": bench.ALIGNMENT_VERSION,
            "inspection_map_version": bench.INSPECTION_MAP_VERSION if attention else None,
        },
        "reference_inventory_sha256": _json_sha256(names),
        "samples": samples,
        "warm": {
            "count": len(warm),
            "scoring_seconds": _warm_statistics(warm, "scoring_seconds"),
            "wall_seconds": _warm_statistics(warm, "wall_seconds"),
        },
    }
    if args.public_measure:
        report["settings"]["entry_point"] = "scoring.measure"
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, indent=2, allow_nan=False) + "\n", encoding="utf-8")
    if report["warm"]["scoring_seconds"] is not None:
        scoring_ms = report["warm"]["scoring_seconds"]["mean"] * 1000
        wall_ms = report["warm"]["wall_seconds"]["mean"] * 1000
        print(f"Warm mean ({len(warm)} scenes) {scoring_ms:.1f} ms scoring, {wall_ms:.1f} ms wall")
    return 0 if all(_scored(sample) for sample in samples) else 1


def _read_report(path):
    report = json.loads(path.read_text(encoding="utf-8"))
    if report["schema_version"] != 1:
        raise ValueError(f"Unsupported report schema in {path}.")
    if len(report["samples"]) != report["settings"]["count"] or not report["samples"]:
        raise ValueError(f"Incomplete scene list in {path}.")
    return report


def _compare(args):
    before = _read_report(args.before)
    after = _read_report(args.after)
    for field in ("fixture", "settings", "reference_inventory_sha256"):
        if before[field] != after[field]:
            raise ValueError(f"Reports differ in {field}.")
    for field in ("score_version", "profile", "explanation_version", "alignment_version", "inspection_map_version"):
        if before["scoring"][field] != after["scoring"][field]:
            raise ValueError(f"Reports differ in scoring {field}.")
    if before["device"] != after["device"] and not args.allow_device_change:
        raise ValueError("Device selectors differ. Pass --allow-device-change for cross-device parity.")
    first = before["samples"]
    second = after["samples"]
    if [sample["name"] for sample in first] != [sample["name"] for sample in second]:
        raise ValueError("Reports use different ordered scene cohorts.")

    parity = (ResultComparison(allow_alignment_trace_change=args.allow_alignment_trace_change,
                               allow_generated_map_change=args.allow_generated_map_change,
                               allow_native_rounding=args.allow_native_rounding)
              if args.tolerant else None)
    failed = False
    fields = ("inputs", "status", "reference_status", "error")
    if not args.allow_generated_map_change:
        fields += ("inspection_map_sha256",)
    elif not before["settings"]["attention"]:
        raise ValueError("Generated map comparison requires attention-weighted reports.")
    if parity is None:
        fields += ("output_sha256", "scene_score")
    for left, right in zip(first, second, strict=True):
        name = left["name"]
        for field in fields:
            if left[field] != right[field]:
                print(f"{name} differs in {field}.", file=sys.stderr)
                failed = True
        if not (_scored(left) and _scored(right)):
            print(f"{name} has an unsuccessful score ({left['status']}, {right['status']}).", file=sys.stderr)
            failed = True
            continue
        if parity is not None:
            for sample in (left, right):
                if "result" not in sample:
                    raise ValueError("Tolerant comparison requires reports generated with --include-results.")
                result = sample["result"]
                if (_json_sha256([result["row"], result["explanation"]]) != sample["output_sha256"]
                        or result["row"]["scene_score"] != sample["scene_score"]
                        or result["row"].get("inspection_map_sha256") != sample["inspection_map_sha256"]):
                    raise ValueError(f"Stored results are inconsistent with the report for {name}.")
            if args.allow_generated_map_change:
                _compare_inspection_maps(left, right)
            parity.check(left["result"], right["result"], name)
    if parity is not None:
        if not failed:
            first_mean, second_mean = (fmean(sample["scene_score"] for sample in cohort) for cohort in (first, second))
            parity.check(first_mean, second_mean, "cohort", "score")
            print(f"Cohort mean scores {first_mean:.9f} and {second_mean:.9f} across {len(first)} scenes, including warmup.")
        return int(bool(parity.finish() or failed))
    if failed:
        return 1
    print(f"Exact parity across {len(first)} scenes, including warmup.")
    return 0


def _positive_int(value):
    number = int(value)
    if number < 1:
        raise argparse.ArgumentTypeError("must be a positive integer")
    return number


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    modes = parser.add_subparsers(dest="mode", required=True)
    run = modes.add_parser("run", help="Score local D29 scenes with the active BokehForge installation.")
    run.add_argument("d29_zip", type=Path, help="Path to the local InfinigenDefocus.zip")
    run.add_argument("--output", type=Path, required=True, help="JSON report path")
    run.add_argument("--count", type=_positive_int, default=6, help="Total scenes including warmup (default: 6)")
    run.add_argument("--size", type=_positive_int, default=4000, help="Square image width and height (default: 4000)")
    run.add_argument("--device", choices=("auto", "cpu", "gpu", "cuda", "mps"), default="auto")
    run.add_argument("--no-attention", action="store_true", help="Use unweighted scoring.")
    run.add_argument("--include-results", action="store_true", help="Store score rows and explanations for tolerant comparison.")
    run.add_argument("--include-inspection-maps", action="store_true",
                     help="Capture the model-grid Inspection maps after timing.")
    run.add_argument("--public-measure", action="store_true",
                     help="Time public scoring.measure, including validation, instead of the bench scoring stage.")
    compare = modes.add_parser("compare", help="Check the fixtures and all functional outputs of two reports.")
    compare.add_argument("before", type=Path)
    compare.add_argument("after", type=Path)
    compare.add_argument("--allow-device-change", action="store_true",
                         help="Compare functional parity across different device selectors.")
    compare.add_argument("--tolerant", action="store_true", help="Allow bounded differences in numeric scoring results.")
    compare.add_argument("--allow-alignment-trace-change", action="store_true",
                         help="Report numeric and hypothesis count changes in search traces. Requires --tolerant.")
    compare.add_argument("--allow-generated-map-change", action="store_true",
                         help="Compare captured Inspection maps numerically. Requires --tolerant.")
    compare.add_argument("--allow-native-rounding", action="store_true",
                         help="Allow bounded geometry and attention summary rounding. Requires --tolerant.")
    args = parser.parse_args()
    if args.mode == "run" and args.device in ("cpu", "mps") and not args.no_attention:
        run.error(f"--device {args.device} requires --no-attention")
    if args.mode == "run" and args.no_attention and args.include_inspection_maps:
        run.error("--include-inspection-maps requires attention-weighted scoring")
    if (args.mode == "compare" and (args.allow_alignment_trace_change or args.allow_generated_map_change
                                    or args.allow_native_rounding)
            and not args.tolerant):
        compare.error("The native compatibility comparison options require --tolerant")
    try:
        return _run(args) if args.mode == "run" else _compare(args)
    except (OSError, BadZipFile, KeyError, ValueError, ZlibError) as exc:
        parser.exit(1, f"{exc}\n")


if __name__ == "__main__":
    sys.exit(main())
