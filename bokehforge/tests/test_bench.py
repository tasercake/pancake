from hashlib import sha256
import json
import os
import subprocess
import sys

import numpy as np
import pytest
import tifffile
from PIL import Image

from bokehforge.bench import _score
from bokehforge.calibration import ScoreCalibration
from bokehforge.data._canonical import INFINIGEN_DISTANCES
from bokehforge.scoring import FEATURES, SCORE_VERSION

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
    return root


def run_bench(root, output, *args, env=None):
    return subprocess.run(
        [sys.executable, "-m", "bokehforge", "bench", "--root", str(root), "--output", str(output), *args],
        capture_output=True, text=True, env=env,
    )


def test_bench_mean_runs_full_scene_and_scores_reference(corpus, tmp_path):
    output = tmp_path / "mean"
    result = run_bench(corpus, output, "--algorithm", "mean", "--decode-workers", "2", "--save-images")
    assert result.returncode == 0, result.stderr
    assert result.stdout == "100.000000\n"
    report = json.loads((output / "report.json").read_text())
    assert report["score"] == 100.0
    assert report["score_version"] == SCORE_VERSION
    assert report["scoring"]["calibration"] == "provisional_engineering_curve"
    assert report["scene_status_counts"] == {"ok": 1, "unscored": 1}
    assert report["reference_status_counts"] == {"eligible": 1, "no_eligible_reference": 1}
    assert {row["dataset_id"]: row["scene_score"] for row in report["scenes"]} == {
        "field": None, "referenced": 100.0,
    }
    target = next(row for row in report["scenes"] if row["reference_status"] == "eligible")
    assert target["frames_read"] == target["frame_count"] == 9
    pixels = tifffile.imread(output / target["image"])
    assert pixels.dtype == np.float32
    assert np.all(pixels == _FIXTURE_REFERENCE)
    assert list(target["score_features"].values()) == [0] * len(FEATURES)

    replay_dir = tmp_path / "replay"
    replay = run_bench(corpus, replay_dir, "--candidate-dir", str(output / "images"))
    assert replay.returncode == 0, replay.stderr
    assert replay.stdout == result.stdout
    replay_report = json.loads((replay_dir / "report.json").read_text())
    replay_row = next(row for row in replay_report["scenes"] if row["reference_status"] == "eligible")
    assert replay_report["cohort_sha256"] == report["cohort_sha256"]
    assert replay_row["artifact_diagnostics"] == target["artifact_diagnostics"]
    assert replay_row["frames_read"] == 0

    missing = tmp_path / "empty"
    missing.mkdir()
    result_missing = run_bench(corpus, tmp_path / "report-missing", "--candidate-dir", str(missing))
    assert result_missing.returncode == 1
    assert result_missing.stdout == "0.000000\n"

    existing = run_bench(corpus, output, "--algorithm", "mean")
    assert existing.returncode == 2
    assert "already exists" in existing.stderr


def test_external_stacker_failure_counts_and_cohort_is_stable(corpus, tmp_path):
    plugin = tmp_path / "fixture_stacker.py"
    plugin.write_text(
        "print('loading fixture stacker')\n"
        "def stack(scene):\n"
        "    print('stacking fixture scene')\n"
        "    frame = next(scene.frames)\n"
        "    assert not frame.pixels.flags.writeable\n"
        "    assert frame.ordinal == 0 and frame.id == scene.frame_ids[0]\n"
        "    if scene.scene_id.startswith('referenced/'):\n"
        "        return frame.pixels[:-1]\n"
        "    return frame.pixels\n"
    )
    output = tmp_path / "external"
    result = run_bench(corpus, output, "--algorithm", "fixture_stacker:stack",
                       env={**os.environ, "PYTHONPATH": str(tmp_path)})
    assert result.returncode == 1, result.stderr
    assert result.stdout == "0.000000\n"
    assert "loading fixture stacker" in result.stderr and "stacking fixture scene" in result.stderr
    report = json.loads((output / "report.json").read_text())
    assert report["scene_status_counts"] == {"invalid_output": 1, "unscored": 1}
    assert {row["dataset_id"]: row["frames_read"] for row in report["scenes"]} == {"field": 0, "referenced": 9}
    assert report["algorithm"]["module_sha256"] == sha256(plugin.read_bytes()).hexdigest()

    baseline = tmp_path / "mean"
    assert run_bench(corpus, baseline, "--algorithm", "mean").returncode == 0
    assert report["cohort_sha256"] == json.loads((baseline / "report.json").read_text())["cohort_sha256"]


def test_unreferenced_selection_is_unscored(corpus, tmp_path):
    output = tmp_path / "field"
    result = run_bench(corpus, output, "--algorithm", "mean", "--dataset", "field")
    assert result.returncode == 0, result.stderr
    assert result.stdout == "unscored\n"
    assert json.loads((output / "report.json").read_text())["score"] is None


def test_cli_loads_a_frozen_calibration(corpus, tmp_path):
    calibration = tmp_path / "calibration.json"
    calibration.write_text(json.dumps(ScoreCalibration(dict.fromkeys(FEATURES, 6.0), (3, 2, 1, 0)).as_dict()))
    output = tmp_path / "calibrated"
    result = run_bench(corpus, output, "--algorithm", "mean", "--score-calibration", str(calibration))
    assert result.returncode == 0, result.stderr
    assert result.stdout == "100.000000\n"
    report = json.loads((output / "report.json").read_text())
    assert report["scoring"]["calibration_sha256"] == sha256(calibration.read_bytes()).hexdigest()


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
        for dataset, group, score in (("D09", "A", 100), ("D09", "A", 80), ("D09", "B", 0), ("D29", "C", 80))
    ]
    assert _score(rows) == (62.5, 3)
    rows.append({"leakage_group_id": "D", "scene_score": None, "reference_status": "error"})
    assert _score(rows) == (None, 3)
