"""Measure BokehForge's bench scoring stage and compare results across checkouts."""

import argparse
from hashlib import sha256
from io import BytesIO
import json
import os
from pathlib import Path
from statistics import fmean, mean, median
import sys
from time import perf_counter
from unittest.mock import patch
from zipfile import BadZipFile, ZipFile


_DETAILS = "summary"
_SCORE_FIELDS = {"score", "scene_score", "overlap_score", "score_contribution"}
_FAMILY_VALUES = {"burden", "mean", "tail", "worst", "affected_fraction", "weighted_damage",
                  "value", "contribution", "cutoff"}
_DIAGNOSTIC_VALUES = {"mean_delta_lightness", "mean_delta_lab_a", "mean_delta_lab_b",
                      "global_exposure_delta_stops", "linear_rgb_rmse"}
_TOLERANCES = {"score": .01, "measurement": .0001}


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


class ResultComparison:
    def __init__(self):
        self.mismatches = 0
        self.maximum = {category: (0.0, ()) for category in _TOLERANCES}

    def _mismatch(self, path, reason):
        self.mismatches += 1
        if self.mismatches <= 20:
            print(f"{'.'.join(map(str, path))} {reason}.", file=sys.stderr)

    def check(self, left, right, *path):
        if type(left) is not type(right) and not (
            {type(left), type(right)} == {int, float} and _numeric_category(path) == "score"
        ):
            self._mismatch(path, f"has different types ({type(left).__name__}, {type(right).__name__})")
        elif isinstance(left, dict):
            if left.keys() != right.keys():
                self._mismatch(path, f"has different keys ({sorted(left.keys() ^ right.keys())})")
            for key in sorted(left.keys() & right.keys()):
                self.check(left[key], right[key], *path, key)
        elif isinstance(left, list):
            if len(left) != len(right):
                self._mismatch(path, f"has different lengths ({len(left)}, {len(right)})")
            for index, (first, second) in enumerate(zip(left, right)):
                self.check(first, second, *path, index)
        elif type(left) in (int, float) and (category := _numeric_category(path)):
            difference = abs(left - right)
            tolerance = _TOLERANCES[category]
            if difference > self.maximum[category][0]:
                self.maximum[category] = (difference, path)
            if not difference <= tolerance:
                self._mismatch(path, f"differs by {difference:.9g} (limit {tolerance:g})")
        elif left != right:
            self._mismatch(path, f"differs ({repr(left)[:120]} vs {repr(right)[:120]})")

    def finish(self):
        for category, (difference, path) in self.maximum.items():
            location = f" at {'.'.join(map(str, path))}" if path else ""
            print(f"Maximum absolute {category} difference {difference:.9g}{location} "
                  f"(limit {_TOLERANCES[category]:g}).")
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


def _warm_statistics(samples, field):
    if not samples or not all(_scored(sample) for sample in samples):
        return None
    values = [sample[field] for sample in samples]
    return {"mean": mean(values), "median": median(values), "min": min(values), "max": max(values)}


def _run(args):
    from bokehforge import bench, inspection_assets
    from bokehforge._device import validate_device
    from bokehforge.inspection import inspection_for

    validate_device(args.device)
    if args.output.resolve() == args.d29_zip.resolve():
        raise ValueError("The report path must differ from the input ZIP path.")
    attention = not args.no_attention
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
            explanation = bench._score_delivery(
                candidate, reference, row, details=_DETAILS, device=args.device,
                inspection=inspection_for(args.device) if attention else None,
            )
            wall = perf_counter() - start
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

    parity = ResultComparison() if args.tolerant else None
    failed = False
    fields = ("inputs", "status", "reference_status", "error", "inspection_map_sha256")
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
                        or result["row"]["scene_score"] != sample["scene_score"]):
                    raise ValueError(f"Stored results are inconsistent with the report for {name}.")
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
    compare = modes.add_parser("compare", help="Check the fixtures and all functional outputs of two reports.")
    compare.add_argument("before", type=Path)
    compare.add_argument("after", type=Path)
    compare.add_argument("--allow-device-change", action="store_true",
                         help="Compare functional parity across different device selectors.")
    compare.add_argument("--tolerant", action="store_true", help="Allow bounded differences in numeric scoring results.")
    args = parser.parse_args()
    if args.mode == "run" and args.device in ("cpu", "mps") and not args.no_attention:
        run.error(f"--device {args.device} requires --no-attention")
    try:
        return _run(args) if args.mode == "run" else _compare(args)
    except (OSError, BadZipFile, KeyError, ValueError) as exc:
        parser.exit(1, f"{exc}\n")


if __name__ == "__main__":
    sys.exit(main())
