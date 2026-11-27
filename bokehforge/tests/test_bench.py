from hashlib import sha256
import json
import os
import subprocess
import sys
from threading import Event

import numpy as np
import pytest
import tifffile
from PIL import Image

from bokehforge.bench import _score, score_saved_output
from bokehforge.data._canonical import INFINIGEN_DISTANCES
from bokehforge.data.lazy import Scene, SceneCatalog
from bokehforge.inspection import INSPECTION_MAP_VERSION, reference_key, reference_preview, write_map
from bokehforge.review.scores import Scores
from bokehforge.scoring import (
    FAMILY_WEIGHTS, FEATURES, INSPECTION_SCORE_VERSION, SCORE_DAMAGE_SCALE, SCORE_EXPLANATION_VERSION, SCORE_VERSION,
    measure,
)

_FIXTURE_REFERENCE = np.float32(20 / 255)


@pytest.fixture
def corpus(tmp_path):
    root = tmp_path / "data"
    for dataset, reference_type in (("referenced", "synthetic_source_image_reference"), ("field", "none")):
        raw = root / "raw" / dataset
        if reference_type != "none":
            folder = "a1_0"
            images = {f"{folder}/image_ap_1.40_fd_{distance:.2f}.png": 20 for distance in INFINIGEN_DISTANCES}
            images[f"{folder}/image_all_in_focus.png"] = 20
            layout = "infinigen_defocus"
        else:
            images = {"example/near.tif": 0.25, "example/far.tif": 0.75}
            layout = "directory_stacks"
        files = []
        for name, level in images.items():
            image = raw / name
            image.parent.mkdir(parents=True, exist_ok=True)
            if image.suffix == ".png":
                Image.fromarray(np.full((3, 4, 3), level, np.uint8)).save(image)
            else:
                tifffile.imwrite(image, np.full((3, 4, 3), level, np.float32), photometric="rgb")
            files.append({"path": name, "sha256": sha256(image.read_bytes()).hexdigest()})
        (raw / "download-manifest.json").write_text(json.dumps({
            "id": dataset, "status": "available", "layout": layout, "transfer": "linear",
            "reference_type": reference_type, "files": files,
        }))

    (root / "sources.json").write_text(json.dumps({"datasets": [{"id": "referenced"}, {"id": "field"}]}))
    return root


def run_bench(root, output, *args, env=None):
    return subprocess.run(
        [sys.executable, "-m", "bokehforge", "bench", "--root", str(root), "--config", str(root / "sources.json"),
         "--output", str(output), *args],
        capture_output=True, text=True, env=env,
    )


def test_bench_mean_runs_full_scene_and_scores_reference(corpus, tmp_path):
    output = tmp_path / "mean"
    result = run_bench(corpus, output, "--dataset", "referenced", "--algorithm", "mean",
                       "--decode-workers", "2", "--save-images")
    assert result.returncode == 0, result.stderr
    assert result.stdout == "100.000000\n"
    report = json.loads((output / "report.json").read_text())
    assert report["schema_version"] == 4
    assert report["score"] == 100.0
    assert report["score_version"] == SCORE_VERSION
    assert report["scoring"]["family_weights"] == FAMILY_WEIGHTS
    assert "view_weights" not in report["scoring"]
    assert report["scoring"]["damage_scale"] == SCORE_DAMAGE_SCALE
    assert report["scene_status_counts"] == {"ok": 1}
    assert report["reference_status_counts"] == {"eligible": 1}
    target, = report["scenes"]
    assert target["dataset_id"] == "referenced" and target["scene_score"] == 100.0
    assert target["frames_read"] == target["frame_count"] == 9
    pixels = tifffile.imread(output / target["image"])
    assert pixels.dtype == np.float32
    assert np.all(pixels == _FIXTURE_REFERENCE)
    assert target["score_features"] == dict.fromkeys(FEATURES, 0)
    assert set(target["artifact_diagnostics"]["views"]) == {"native"}
    assert "score_explanation_path" not in target

    replay_dir = tmp_path / "replay"
    replay = run_bench(corpus, replay_dir, "--scene", target["scene_id"],
                       "--candidate-dir", str(output / "images"), "--explain-score", "tiles")
    assert replay.returncode == 0, replay.stderr
    assert replay.stdout == result.stdout
    replay_report = json.loads((replay_dir / "report.json").read_text())
    replay_row = next(row for row in replay_report["scenes"] if row["reference_status"] == "eligible")
    assert replay_report["cohort_sha256"] == report["cohort_sha256"]
    assert replay_row["artifact_diagnostics"] == target["artifact_diagnostics"]
    assert replay_row["frames_read"] == 0
    assert replay_report["score_explanation_version"] == SCORE_EXPLANATION_VERSION
    assert replay_report["score_aggregation"]["score"] == report["score"]
    explanation = json.loads((replay_dir / replay_row["score_explanation_path"]).read_text())
    assert explanation["reference_id"] == target["reference_ids"][0]
    assert explanation["scene_id"] == target["scene_id"]
    assert explanation["score"] == replay_row["scene_score"]
    placement, = explanation["placements"]
    assert placement["candidate_to_reference_xy"] == [[1, 0, 0], [0, 1, 0]]
    assert len(placement["comparison"]["tile_grids"]) == 8
    catalog = SceneCatalog(corpus, config=corpus / "sources.json")
    scene = catalog.scene(target["scene_id"])
    assert score_saved_output(catalog, scene, output / target["image"], details="tiles")["explanation"] == explanation
    assert "explanation" not in score_saved_output(catalog, scene, output / target["image"])

    missing = tmp_path / "empty"
    missing.mkdir()
    result_missing = run_bench(corpus, tmp_path / "report-missing", "--dataset", "referenced",
                                "--candidate-dir", str(missing))
    assert result_missing.returncode == 1
    assert result_missing.stdout == "0.000000\n"

    existing = run_bench(corpus, output, "--algorithm", "mean")
    assert existing.returncode == 2
    assert "already exists" in existing.stderr


def test_inspection_scores_a_saved_delivery_with_a_reference_bound_map(corpus, tmp_path):
    catalog = SceneCatalog(corpus, config=corpus / "sources.json", datasets=["referenced"])
    scene = catalog.scene(catalog.scene_ids[0])
    reference = catalog.decode(scene.reference_ids[0]).data
    heatmap = np.repeat(np.array([[.25, .25, .75, .75]], dtype=np.float32), reference.shape[0], axis=0)
    candidates = tmp_path / "candidates"
    candidates.mkdir()
    path = candidates / (scene.id + ".tif")
    path.parent.mkdir(parents=True)
    candidate = reference.copy()
    candidate[:, 2:] += .07
    tifffile.imwrite(path, candidate, photometric="rgb")
    maps = tmp_path / "maps"
    maps.mkdir()
    heatmap_path = maps / (scene.id + ".npz")
    write_map(heatmap_path, reference_key(catalog, scene, reference_preview(reference)), reference.shape[:2], heatmap)

    output = tmp_path / "attention"
    args = ("--dataset", "referenced", "--candidate-dir", str(candidates), "--inspection-maps", str(maps))
    result = run_bench(corpus, output, *args, "--explain-score", "tiles")
    assert result.returncode == 0, result.stderr
    report = json.loads((output / "report.json").read_text())
    assert report["score_version"] == INSPECTION_SCORE_VERSION
    assert report["scoring"]["inspection"]["map_version"] == INSPECTION_MAP_VERSION
    row, = report["scenes"]
    assert row["inspection_map_sha256"] == sha256(heatmap_path.read_bytes()).hexdigest()
    assert row["scene_score"] == measure(candidate, reference, inspection_heatmap=heatmap).score
    assert row["scene_score"] < measure(candidate, reference).score
    saved = score_saved_output(catalog, scene, path, details="tiles", inspection_maps=maps)
    assert saved["score"] == row["scene_score"]
    assert saved["explanation"] == json.loads((output / row["score_explanation_path"]).read_text())

    updated = Event()
    review = Scores(updated.set, maps)
    delivery = {"image": path.relative_to(tmp_path).as_posix(), "stacker": "test", "scene_fingerprint": scene.id}
    try:
        pending = review.get(catalog, scene, tmp_path, delivery)
        assert pending["status"] == "pending"
        assert updated.wait(5)
        assert review.get(catalog, scene, tmp_path, delivery)["score"] == saved["score"]
        updated.clear()
        write_map(heatmap_path, reference_key(catalog, scene, reference_preview(reference)),
                  reference.shape[:2], np.zeros_like(heatmap))
        assert review.get(catalog, scene, tmp_path, delivery)["status"] == "pending"
        assert updated.wait(5)
        neutral = review.get(catalog, scene, tmp_path, delivery)
        assert neutral["inspection_map_sha256"] != saved["inspection_map_sha256"]
        assert neutral["score"] == measure(candidate, reference).score
    finally:
        review.close()

    write_map(heatmap_path, "other-reference", reference.shape[:2], heatmap)
    result = run_bench(corpus, tmp_path / "stale", *args)
    assert result.returncode == 1
    assert result.stdout == "unscored\n"
    stale, = json.loads((tmp_path / "stale" / "report.json").read_text())["scenes"]
    assert stale["status"] == "scoring_error"
    assert "does not match the reference" in stale["error"]


def test_external_stacker_failure_counts_and_cohort_is_stable(corpus, tmp_path):
    plugin = tmp_path / "fixture_stacker.py"
    plugin.write_text(
        "print('loading fixture stacker')\n"
        "def stack(scene):\n"
        "    print('stacking fixture scene')\n"
        "    frame = next(scene.frames)\n"
        "    assert not frame.pixels.flags.writeable\n"
        "    assert frame.ordinal == 0 and frame.id == scene.frame_ids[0]\n"
        "    return frame.pixels.astype('float64')\n"
    )
    output = tmp_path / "external"
    result = run_bench(corpus, output, "--dataset", "referenced", "--algorithm", "fixture_stacker:stack",
                       env={**os.environ, "PYTHONPATH": str(tmp_path)})
    assert result.returncode == 1, result.stderr
    assert result.stdout == "0.000000\n"
    assert "loading fixture stacker" in result.stderr and "stacking fixture scene" in result.stderr
    report = json.loads((output / "report.json").read_text())
    assert report["scene_status_counts"] == {"invalid_output": 1}
    assert {row["dataset_id"]: row["frames_read"] for row in report["scenes"]} == {"referenced": 9}
    assert report["algorithm"]["module_sha256"] == sha256(plugin.read_bytes()).hexdigest()

    baseline = tmp_path / "mean"
    assert run_bench(corpus, baseline, "--dataset", "referenced", "--algorithm", "mean").returncode == 0
    assert report["cohort_sha256"] == json.loads((baseline / "report.json").read_text())["cohort_sha256"]


def test_default_scores_original_crop_and_replays_it(corpus, tmp_path):
    plugin = tmp_path / "crop_stacker.py"
    plugin.write_text("def stack(scene):\n    return next(scene.frames).pixels[1:, 1:]\n")
    env = {**os.environ, "PYTHONPATH": str(tmp_path)}
    output = tmp_path / "crop"
    cropped = run_bench(corpus, output, "--dataset", "referenced", "--algorithm", "crop_stacker:stack", "--save-images",
                        env=env)
    assert cropped.returncode == 0, cropped.stderr
    assert cropped.stdout == "50.000000\n"
    report = json.loads((output / "report.json").read_text())
    assert report["score_version"] == SCORE_VERSION
    scene = next(row for row in report["scenes"] if row["reference_status"] == "eligible")
    assert scene["alignment"]["placements"][0]["coverage"] == 0.5
    assert scene["alignment"]["placements"][0]["overlap_score"] == 100
    assert tifffile.imread(output / scene["image"]).shape == (2, 3, 3)

    replay_dir = tmp_path / "replay"
    replay = run_bench(corpus, replay_dir, "--dataset", "referenced", "--candidate-dir", str(output / "images"))
    assert replay.returncode == 0, replay.stderr
    assert replay.stdout == cropped.stdout
    replay_row, = json.loads((replay_dir / "report.json").read_text())["scenes"]
    assert replay_row["alignment"] == scene["alignment"]
    saved = output / scene["image"]
    catalog = SceneCatalog(corpus, datasets=["referenced"], config=corpus / "sources.json")
    catalog_scene = catalog.scene(scene["scene_id"])
    reference = catalog.decode(catalog_scene.reference_ids[0]).data
    assert measure(tifffile.imread(saved), reference).alignment == replay_row["alignment"]
    assert score_saved_output(catalog, catalog_scene, saved)["alignment"] == replay_row["alignment"]

    removed = run_bench(corpus, tmp_path / "removed", "--dataset", "referenced", "--candidate-dir", str(output / "images"),
                        "--score-profile", "reference-fidelity-crop-v2")
    assert removed.returncode == 2
    assert "unrecognized arguments: --score-profile" in removed.stderr



@pytest.mark.parametrize("selection", [(), ("--dataset", "field")])
def test_bench_rejects_selections_without_ground_truth(corpus, tmp_path, selection):
    plugin = tmp_path / "must_not_load.py"
    plugin.write_text("raise AssertionError('stacker loaded before reference validation')\n")
    output = tmp_path / "field"
    result = run_bench(corpus, output, *selection, "--algorithm", "must_not_load:stack",
                       env={**os.environ, "PYTHONPATH": str(tmp_path)})
    assert result.returncode == 2, result.stderr
    assert result.stdout == ""
    assert "Benchmark scene field/" in result.stderr
    assert "requires exactly one supported RGB ground-truth image" in result.stderr
    assert not output.exists()


def test_replay_rejects_selection_without_ground_truth(corpus, tmp_path):
    candidates = tmp_path / "candidates"
    candidates.mkdir()
    output = tmp_path / "replay"
    result = run_bench(corpus, output, "--candidate-dir", str(candidates))
    assert result.returncode == 2, result.stderr
    assert result.stdout == ""
    assert "requires exactly one supported RGB ground-truth image" in result.stderr
    assert not output.exists()


@pytest.mark.parametrize(("reference_type", "reference_ids"), [
    ("none", ()),
    ("rendered_all_in_focus", ()),
    ("focal_stack", ("source",)),
    ("rendered_all_in_focus", ("first", "second")),
])
def test_score_saved_output_requires_one_supported_reference(tmp_path, reference_type, reference_ids):
    scene = Scene("example", (), reference_ids, {"reference_type": reference_type})
    with pytest.raises(ValueError, match="requires exactly one supported RGB ground-truth image"):
        score_saved_output(None, scene, tmp_path / "output.tif")


def test_focal_decode_error_is_reported_even_after_early_return(corpus, tmp_path):
    raw = corpus / "raw" / "referenced"
    damaged = raw / "a1_0/image_ap_1.40_fd_1.20.png"
    damaged.write_bytes(b"not a PNG")
    manifest_path = raw / "download-manifest.json"
    manifest = json.loads(manifest_path.read_text())
    manifest["files"] = [
        {**item, "sha256": sha256(damaged.read_bytes()).hexdigest()}
        if item["path"] == damaged.relative_to(raw).as_posix() else item for item in manifest["files"]
    ]
    manifest_path.write_text(json.dumps(manifest))
    (tmp_path / "first_frame.py").write_text("def stack(scene):\n    return next(scene.frames).pixels\n")
    output = tmp_path / "damaged"
    result = run_bench(corpus, output, "--algorithm", "first_frame:stack", "--dataset", "referenced",
                       env={**os.environ, "PYTHONPATH": str(tmp_path)})
    assert result.returncode == 1, result.stderr
    assert result.stdout == "unscored\n"
    row, = json.loads((output / "report.json").read_text())["scenes"]
    assert row["status"] == "input_error"
    assert row["frames_read"] == 1


    # The same reference is still scorable when an existing delivery is replayed.
    replay_root = tmp_path / "precomputed"
    image = replay_root / (row["scene_id"] + ".tif")
    image.parent.mkdir(parents=True)
    tifffile.imwrite(image, np.full((3, 4, 3), _FIXTURE_REFERENCE, np.float32), photometric="rgb")
    replay = run_bench(corpus, tmp_path / "replay", "--candidate-dir", str(replay_root), "--dataset", "referenced")
    assert replay.returncode == 0, replay.stderr
    assert replay.stdout == "100.000000\n"


def test_score_averages_independent_groups_and_withholds_on_reference_error():
    rows = [
        {"dataset_id": dataset, "leakage_group_id": group, "scene_score": score, "reference_status": "eligible"}
        for dataset, group, score in (("first", "A", 100), ("first", "A", 80), ("first", "B", 0), ("second", "C", 80))
    ]
    explanation = {}
    assert _score(rows, explanation=explanation) == (62.5, 3)
    assert explanation["status"] == "ok"
    assert explanation["datasets"]["first"] == {
        "score": 45, "weight": 0.5,
        "groups": {"A": {"score": 90, "weight": 0.5, "scene_count": 2},
                   "B": {"score": 0, "weight": 0.5, "scene_count": 1}},
    }
    rows.append({"leakage_group_id": "D", "scene_score": None, "reference_status": "error"})
    assert _score(rows, explanation=explanation) == (None, 3)
    assert explanation["status"] == "evaluation_error"
    assert explanation["score"] is None
