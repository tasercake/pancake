"""Compare the public BokehForge scorers on deterministic synthetic scenes."""

from __future__ import annotations

import argparse
from dataclasses import dataclass
from hashlib import file_digest
import json
from pathlib import Path
import sys
from tempfile import TemporaryDirectory
from types import SimpleNamespace
from typing import TYPE_CHECKING

from compare import ResultComparison, _image_sha256

if TYPE_CHECKING:
    import numpy as np


@dataclass(frozen=True)
class Fixture:
    name: str
    candidate: np.ndarray
    reference: np.ndarray
    details: str | None
    expected_alignment: str = "matched"
    heatmap: np.ndarray | None = None
    partial_support: bool = False


def _fixtures():
    import numpy as np
    from scipy.ndimage import affine_transform, gaussian_filter

    def texture(shape):
        samples = gaussian_filter(np.random.default_rng(7).normal(size=shape), 2.8)
        light = np.clip(.5 + .12 * samples / samples.std(), 0, 1).astype(np.float32)
        return np.stack((.92 * light + .03, light, .83 * light + .06), axis=-1)

    def strided(pixels):
        height, width, _ = pixels.shape
        buffer = np.empty((2 * height, 2 * width, 3), dtype=np.float32)
        view = buffer[::-2, ::2, ::-1]
        view[...] = pixels
        view.setflags(write=False)
        return view

    reference = texture((209, 273))
    crop = reference[19:-9, 31:-15]
    edited = reference.copy()
    blurred = np.s_[28:107, 22:138]
    edited[blurred] = gaussian_filter(edited[blurred], (2.3, 2.3, 0))
    edited[48:171, 83:208, 0] += np.float32(.043)
    edited[74:189, 119:243, 2] *= np.float32(.91)
    yield Fixture("exact", reference.copy(), reference, "summary")
    yield Fixture("exact-crop", crop, reference, "tiles", partial_support=True)
    yield Fixture("detail-and-color", edited, reference, "tiles")
    yield Fixture("strided-native-grid", strided(edited), strided(reference), None)

    reference = texture((120, 150))
    y, x = np.indices(reference.shape[:2], dtype=np.float32)
    heatmap_buffer = np.empty(y.nbytes + 1, dtype=np.uint8)
    heatmap = np.ndarray(y.shape, dtype=np.float32, buffer=heatmap_buffer, offset=1)
    heatmap[:] = np.exp(-((y - 35) / 22) ** 2 - ((x - 73) / 35) ** 2)
    angle = np.deg2rad(-2.35)
    c, s = np.cos(angle), np.sin(angle)
    matrix = np.array([[c, -s], [s, c]])
    delivery = np.stack([affine_transform(plane, matrix, (-4.13, 2.37), order=3, mode="reflect")
                         for plane in np.moveaxis(reference, -1, 0)], axis=-1)
    delivery[39:61, 80:113, 1] *= np.float32(.96)
    yield Fixture("affine-strided", strided(delivery), strided(reference), "full",
                  heatmap=heatmap, partial_support=True)
    unrelated = np.random.default_rng(19).uniform(.1, .9, size=(75, 112, 3)).astype(np.float32)
    yield Fixture("unmatched", unrelated, reference, "attention", expected_alignment="unmatched", heatmap=heatmap)

    reference = np.repeat(np.linspace(-.05, 1.08, 37, dtype=np.float32)[None, :, None], 3, axis=-1)
    threshold = np.float32((6 / 29) ** 3)
    reference[0, 2:5] = np.array([np.nextafter(threshold, np.float32(0)), threshold,
                                  np.nextafter(threshold, np.float32(1))])[:, None]
    delivery = reference.copy()
    delivery[0, 2:6, 0] += np.float32(.008)
    delivery[0, 33:, 2] += np.float32(.32)
    yield Fixture("range-boundaries", delivery.astype(">f4"), reference.astype(">f4"), "summary",
                  expected_alignment="uncertain")


def _file_sha256(path):
    with path.open("rb") as file:
        return file_digest(file, "sha256").hexdigest()


def _check_fixture(fixture, measurement, saved):
    if saved["status"] != "ok":
        raise ValueError(f"{fixture.name} benchmark returned {saved['status']}: {saved.get('error')}")
    for alignment in (measurement.alignment, saved["alignment"]):
        if alignment["status"] != fixture.expected_alignment:
            raise ValueError(f"{fixture.name} expected {fixture.expected_alignment} alignment, got {alignment['status']}.")
    explanations = (measurement.explanation, saved.get("explanation"))
    for explanation in explanations:
        observed_details = None if explanation is None else explanation["detail_level"]
        if observed_details != fixture.details:
            raise ValueError(f"{fixture.name} expected explanation level {fixture.details}, got {observed_details}.")
        if fixture.heatmap is not None and explanation is not None:
            attention = explanation["attention"]
            if ("data" in attention) != (fixture.details in ("attention", "full")):
                raise ValueError(f"{fixture.name} has the wrong attention evidence for its explanation level.")
    if fixture.name == "exact" and measurement.score != 100:
        raise ValueError("The exact fixture must score 100.")
    if fixture.name == "exact-crop" and measurement.alignment["placements"][0]["overlap_score"] != 100:
        raise ValueError("The crop fixture must have exact overlap.")
    if fixture.name == "unmatched" and measurement.score != 0:
        raise ValueError("The unmatched fixture must score zero.")
    if fixture.name in ("detail-and-color", "strided-native-grid") and not all(
        measurement.features[name] > 0 for name in ("native.lost_detail", "native.color")
    ):
        raise ValueError(f"{fixture.name} must carry lost-detail and color damage.")
    if fixture.name == "range-boundaries" and measurement.features["native.range"] <= 0:
        raise ValueError("The range fixture must carry clipping damage.")
    if fixture.partial_support:
        area = fixture.reference.shape[0] * fixture.reference.shape[1]
        for explanation in explanations:
            comparison = explanation["placements"][0]["comparison"]
            if not 0 < comparison["measured_pixels"] < area:
                raise ValueError(f"{fixture.name} must have partial measured support.")
            if not all(grid["covered_pixels"] is not None for grid in comparison["tile_grids"].values()):
                raise ValueError(f"{fixture.name} must expose support counts in every tile grid.")
    if fixture.name == "affine-strided" and all(
        placement["rotation_degrees"] == 0 for placement in measurement.alignment["placements"]
    ):
        raise ValueError("The affine fixture must use a rotated projection.")


def _run(args):
    import numpy as np
    import tifffile
    from bokehforge import bench, scoring

    def image_hash(pixels):
        return _image_sha256(np.ascontiguousarray(pixels))

    samples, rows = {}, []
    with TemporaryDirectory(prefix="bokehforge-parity-") as directory:
        for index, fixture in enumerate(_fixtures()):
            reference_hash = image_hash(fixture.reference)
            heatmap_digest = None if fixture.heatmap is None else image_hash(fixture.heatmap)

            def generate(target):
                if image_hash(target) != reference_hash:
                    raise ValueError("The benchmark generated Inspection from the wrong image.")
                return fixture.heatmap, heatmap_digest

            options = {"device": args.device, "attention": fixture.heatmap is not None, "details": fixture.details}
            measurement = scoring.measure(fixture.candidate, fixture.reference, inspection_heatmap=fixture.heatmap,
                                          **options)
            path = Path(directory) / f"{fixture.name}.tif"
            tifffile.imwrite(path, fixture.candidate.astype(np.float32, copy=False), photometric="rgb", metadata=None)
            scene = SimpleNamespace(id=fixture.name, metadata={"reference_type": "synthetic_source_image_reference"},
                                    reference_ids=(f"{fixture.name}/reference",))
            decoded = SimpleNamespace(data=fixture.reference, alpha=None)
            catalog = SimpleNamespace(decode=lambda _: decoded)
            inspection = None if fixture.heatmap is None else SimpleNamespace(generate=generate)
            saved = bench.score_saved_output(catalog, scene, path, inspection=inspection, **options)
            _check_fixture(fixture, measurement, saved)
            if saved.get("inspection_map_sha256") != heatmap_digest:
                raise ValueError(f"{fixture.name} has the wrong Inspection digest.")
            rows.append({"scene_score": saved["score"], "reference_status": "eligible",
                         "dataset_id": "primary" if index < 5 else "secondary", "leakage_group_id": f"group-{index // 2}"})
            samples[fixture.name] = {
                "inputs": {"candidate_sha256": image_hash(fixture.candidate), "reference_sha256": reference_hash,
                           "candidate_tiff_sha256": _file_sha256(path), "inspection_map_sha256": heatmap_digest},
                "measure": {"score": measurement.score, "score_features": measurement.features,
                            "diagnostics": measurement.diagnostics, "alignment": measurement.alignment,
                            "explanation": measurement.explanation},
                "score_saved_output": saved,
            }
            print(f"{fixture.name} {saved['status']} {measurement.alignment['status']} score {saved['score']:.9f}",
                  flush=True)

    aggregation = {}
    score, groups = bench._score(rows, explanation=aggregation)
    report = {"schema_version": 1, "fixture": "synthetic-scorer-v1", "device": args.device,
              "scoring_module": str(Path(scoring.__file__).resolve()), "samples": samples,
              "cohort": {"score": score, "scored_groups": groups, "aggregation": aggregation}}
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, separators=(",", ":"), allow_nan=False) + "\n", encoding="utf-8")
    print(f"Cohort score {score:.9f} from {report['scoring_module']}.")
    return 0


def _compare(args):
    first, second = (json.loads(path.read_text(encoding="utf-8")) for path in (args.before, args.after))
    for report in (first, second):
        if report["schema_version"] != 1 or report["fixture"] != "synthetic-scorer-v1" or not report["samples"]:
            raise ValueError("Expected a synthetic scorer version 1 report with samples.")
    if first["device"] != second["device"]:
        raise ValueError("Use the same device selector for the baseline and candidate runs.")
    parity = ResultComparison()
    parity.check(first["samples"], second["samples"], "scenes")
    parity.check(first["cohort"], second["cohort"], "cohort")
    print(f"Cohort scores {first['cohort']['score']:.9f} and {second['cohort']['score']:.9f}.")
    return parity.finish()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    modes = parser.add_subparsers(dest="mode", required=True)
    run = modes.add_parser("run", help="Score the synthetic scenes with the active BokehForge installation.")
    run.add_argument("--device", choices=("cpu", "cuda", "mps"), default="cpu")
    run.add_argument("--output", type=Path, required=True)
    compare = modes.add_parser("compare", help="Check structural parity and numeric tolerances.")
    compare.add_argument("before", type=Path)
    compare.add_argument("after", type=Path)
    args = parser.parse_args()
    try:
        return _run(args) if args.mode == "run" else _compare(args)
    except (OSError, KeyError, ValueError) as exc:
        parser.exit(1, f"{exc}\n")


if __name__ == "__main__":
    sys.exit(main())
