from base64 import b64decode
from hashlib import sha256
import json
import os
import subprocess
import sys
from threading import Barrier, Event, get_ident
from types import ModuleType, SimpleNamespace
from zlib import decompress

import numpy as np
import pytest
import tifffile
from PIL import Image

import bokehforge
from bokehforge import _rust, bench, scoring
from bokehforge.bench import _score, bench as run_benchmark, main, score_saved_output
from bokehforge.data._canonical import INFINIGEN_DISTANCES
from bokehforge.data.lazy import Scene, SceneCatalog
from bokehforge.inspection import INSPECTION_MAP_VERSION, Inspection, reference_preview
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
         "--output", str(output), "--no-attention", *args],
        capture_output=True, text=True, env=env,
    )


def test_benchmark_callable_matches_cli(corpus, tmp_path, capsys):
    options = ["--dataset", "referenced", "--algorithm", "mean", "--explain-score"]
    cli_output = tmp_path / "cli"
    cli_result = run_bench(corpus, cli_output, *options)
    assert cli_result.returncode == 0, cli_result.stderr

    api_output = tmp_path / "api"
    assert type(bench) is ModuleType
    assert bokehforge.bench is bench
    assert bench.bench is run_benchmark
    assert run_benchmark([
        "--root", str(corpus), "--config", str(corpus / "sources.json"), "--no-attention",
        "--output", str(api_output), *options,
    ]) == cli_result.returncode
    assert capsys.readouterr().out == cli_result.stdout

    cli_report, api_report = [json.loads((path / "report.json").read_text()) for path in (cli_output, api_output)]
    for field in ("score", "score_aggregation", "score_version", "cohort_sha256", "scored_groups"):
        assert api_report[field] == cli_report[field]


@pytest.mark.parametrize("device", ["cpu", "mps"])
def test_weighted_benchmark_accepts_cpu_and_mps_selection(corpus, tmp_path, monkeypatch, capsys, device):
    def missing_runtime():
        raise RuntimeError("fixture native Inspection runtime is unavailable")

    monkeypatch.setattr(bench, "prepare_attention", missing_runtime)
    output = tmp_path / "benchmark"
    assert main(["--algorithm", "mean", "--output", str(output), "--device", device,
                 "--root", str(corpus), "--config", str(corpus / "sources.json"),
                 "--dataset", "referenced"]) == 1
    assert capsys.readouterr().out == "unscored\n"
    report = json.loads((output / "report.json").read_text())
    failed, = report["scenes"]
    assert failed["status"] == "scoring_error"
    assert "native Inspection runtime is unavailable" in failed["error"]


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
                       "--candidate-dir", str(output / "images"), "--explain-score", "--device", "cpu")
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
    assert score_saved_output(catalog, scene, output / target["image"], details="full", attention=False)["explanation"] == explanation
    assert "explanation" not in score_saved_output(catalog, scene, output / target["image"], attention=False)

    missing = tmp_path / "empty"
    missing.mkdir()
    result_missing = run_bench(corpus, tmp_path / "report-missing", "--dataset", "referenced",
                                "--candidate-dir", str(missing))
    assert result_missing.returncode == 1
    assert result_missing.stdout == "0.000000\n"

    existing = run_bench(corpus, output, "--algorithm", "mean")
    assert existing.returncode == 2
    assert "already exists" in existing.stderr


@pytest.mark.parametrize("device", ["cpu", "mps"])
def test_saved_output_accepts_precomputed_inspection_provider(corpus, tmp_path, device):
    if not _rust.damage_available(device) and device != "cpu":
        pytest.skip("Requires a native Apple Metal device")
    catalog = SceneCatalog(corpus, config=corpus / "sources.json", datasets=["referenced"])
    scene = catalog.scene(catalog.scene_ids[0])
    reference = catalog.decode(scene.reference_ids[0]).data
    path = tmp_path / "reference.tif"
    tifffile.imwrite(path, reference, photometric="rgb")
    heatmap = np.full(reference.shape[:2], .5, dtype="<f4")
    digest = sha256(heatmap.tobytes()).hexdigest()

    class PrecomputedInspection:
        def generate(self, target):
            np.testing.assert_array_equal(target, reference)
            return heatmap, digest

    saved = score_saved_output(catalog, scene, path, details="summary", inspection=PrecomputedInspection(), device=device)
    assert saved["status"] == "ok"
    assert saved["score"] == 100
    assert saved["profile"] == INSPECTION_SCORE_VERSION
    assert saved["inspection_map_sha256"] == digest
    assert saved["explanation"]["attention"]["mean"] == .5


def test_builtin_inspection_uses_native_measurement_and_records_digest(monkeypatch):
    pixels = np.zeros((3, 4, 3), dtype=np.float32)

    def measure_delivery(result, reference, *, details, device, generate, attention, on_digest):
        assert device == "cpu"
        assert attention and generate is None
        on_digest("native-digest")
        return SimpleNamespace(score=100.0, alignment={}, features={}, explanation=None)

    monkeypatch.setattr(bench, "_measure_delivery", measure_delivery)
    row = {}
    bench._score_delivery(pixels, pixels, row, inspection=Inspection(), device="cpu")
    assert row["scene_score"] == 100.0
    assert row["inspection_map_sha256"] == "native-digest"


def test_inspection_is_generated_during_bench_and_review(corpus, tmp_path, monkeypatch, capsys):
    catalog = SceneCatalog(corpus, config=corpus / "sources.json", datasets=["referenced"])
    scene = catalog.scene(catalog.scene_ids[0])
    reference = catalog.decode(scene.reference_ids[0]).data
    heatmap = np.repeat(np.array([[.25, .25, .75, .75]], dtype=np.float32), reference.shape[0], axis=0)
    candidates = tmp_path / "candidates"
    path = candidates / (scene.id + ".tif")
    path.parent.mkdir(parents=True)
    candidate = reference.copy()
    candidate[:, 2:] += .07
    tifffile.imwrite(path, candidate, photometric="rgb")
    seen = []

    expected_digest = sha256(heatmap.astype("<f4").tobytes()).hexdigest()

    class FixtureInspection:
        def preview_map(self, reference):
            if not hasattr(self, "_test_inspection_map"):
                seen.append(np.asarray(reference_preview(reference)).copy())
                self._test_inspection_map = heatmap, expected_digest
            return self._test_inspection_map

        def cached_map(self, digest):
            cached, cached_digest = getattr(self, "_test_inspection_map", (None, None))
            return cached if digest == cached_digest else None

        generate = preview_map

    def scoring_override(attention):
        return (scoring._measure_delivery, FixtureInspection()) if attention else (None, None)

    monkeypatch.setattr(bench, "_benchmark_override", scoring_override)
    monkeypatch.setattr("bokehforge.review.scores.Inspection", FixtureInspection)
    args = ["--root", str(corpus), "--config", str(corpus / "sources.json"),
            "--dataset", "referenced", "--candidate-dir", str(candidates)]
    output = tmp_path / "attention"
    assert main([*args, "--output", str(output), "--explain-score"]) == 0
    report = json.loads((output / "report.json").read_text())
    assert report["score_version"] == INSPECTION_SCORE_VERSION
    assert report["scoring"]["inspection"]["map_version"] == INSPECTION_MAP_VERSION
    row, = report["scenes"]
    assert row["inspection_map_sha256"] == sha256(heatmap.astype("<f4").tobytes()).hexdigest()
    assert len(seen) == 1
    np.testing.assert_array_equal(seen[0], reference_preview(reference))
    assert row["scene_score"] == measure(candidate, reference, inspection_heatmap=heatmap).score
    baseline = measure(candidate, reference, attention=False).score
    assert row["scene_score"] < baseline
    saved = score_saved_output(catalog, scene, path, details="full", inspection=FixtureInspection())
    assert saved["score"] == row["scene_score"]
    assert saved["explanation"] == json.loads((output / row["score_explanation_path"]).read_text())
    assert len(saved["explanation"]["placements"][0]["comparison"]["tile_grids"]) == 8
    attention = saved["explanation"]["attention"]
    assert attention["encoding"] == "base64+zlib+float32-le"
    assert attention["size_wh"] == list(reference.shape[1::-1])
    pixels = np.frombuffer(decompress(b64decode(attention["data"])), dtype="<f4").reshape(heatmap.shape)
    np.testing.assert_array_equal(pixels, heatmap)
    assert attention["damage_multiplier_range"] == [0.75, 1.25]

    updated = Event()
    review = Scores(updated.set)
    delivery = {"image": path.relative_to(tmp_path).as_posix(), "stacker": "test", "scene_fingerprint": scene.id}
    try:
        pending = review.get(catalog, scene, tmp_path, delivery)
        assert pending["status"] == "pending"
        assert updated.wait(5)
        result = review.get(catalog, scene, tmp_path, delivery)
        assert result["score"] == saved["score"]
        updated.clear()
        assert review.tiles(catalog, scene, tmp_path, delivery, result["artifact_revision"], "tone", 8)["status"] == "pending"
        assert updated.wait(5)
        tiles = review.tiles(catalog, scene, tmp_path, delivery, result["artifact_revision"], "tone", 8)
        assert tiles["inspection_map_sha256"] == saved["inspection_map_sha256"]
        assert "data" not in result["explanation"]["attention"]
        updated.clear()
        revision = result["artifact_revision"]
        assert review.attention_overlay(catalog, scene, tmp_path, delivery, revision)["status"] == "pending"
        assert updated.wait(5)
        overlay = review.attention_overlay(catalog, scene, tmp_path, delivery, revision)
        assert overlay["artifact_revision"] == revision
        assert overlay["inspection_map_sha256"] == saved["inspection_map_sha256"]
        assert overlay["reference_id"] == scene.reference_ids[0]
        assert overlay["candidate_to_reference_xy"] == tiles["candidate_to_reference_xy"]
        assert overlay["attention"]["encoding"] == "base64+uint8"
        assert b64decode(overlay["attention"]["data"]) == np.rint(heatmap * 255).astype(np.uint8).tobytes()
        assert review.attention_overlay(catalog, scene, tmp_path, delivery, "outdated")["status"] == "stale"
        assert len(seen) == 3
        delivery.update(review.record(catalog, scene, tmp_path, delivery))
        persisted = json.loads((tmp_path / delivery["score_explanation_path"]).read_text())
        assert persisted == saved["explanation"]
        assert len(seen) == 3
    finally:
        review.close()

    def missing_models(self, reference):
        raise RuntimeError("test model loading failed")

    monkeypatch.setattr(FixtureInspection, "preview_map", missing_models)
    monkeypatch.setattr(FixtureInspection, "generate", missing_models)
    updated.clear()
    reopened = Scores(updated.set)
    try:
        summary = reopened.get(catalog, scene, tmp_path, delivery)
        assert summary["score"] == saved["score"] and reopened.worker is None
        assert "data" not in summary["explanation"]["attention"]
        for fetch in (
            lambda: reopened.tiles(catalog, scene, tmp_path, delivery, revision, "tone", 8),
            lambda: reopened.attention_overlay(catalog, scene, tmp_path, delivery, revision),
        ):
            updated.clear()
            assert fetch()["status"] == "pending"
            assert updated.wait(5)
            persisted_overlay = fetch()
            assert persisted_overlay["status"] == "ok"
            assert persisted_overlay["inspection_map_sha256"] == saved["inspection_map_sha256"]
    finally:
        reopened.close()
    disabled = tmp_path / "disabled"
    assert main([*args, "--output", str(disabled), "--no-attention"]) == 0
    report = json.loads((disabled / "report.json").read_text())
    assert report["score"] == baseline
    assert report["score_version"] == SCORE_VERSION
    assert "inspection" not in report["scoring"]
    capsys.readouterr()

    broken = tmp_path / "model-error"
    assert main([*args, "--output", str(broken)]) == 1
    assert capsys.readouterr().out == "unscored\n"
    failed, = json.loads((broken / "report.json").read_text())["scenes"]
    assert failed["status"] == "scoring_error"
    assert "test model loading failed" in failed["error"]


@pytest.mark.parametrize(("meminfo", "free_gib", "parent_headroom_gib", "fits"), [
    pytest.param("MemAvailable: 25165824 kB\n", 2, None, True, id="reclaimable-page-cache"),
    pytest.param("MemAvailable: 25165824 kB\n", 2, 4, False, id="parent-cgroup-limit"),
    pytest.param("MemAvailable: 25165824 B\n", 2, None, False, id="unsupported-meminfo-unit"),
    pytest.param("MemAvailable: invalid kB\n", 12, None, True, id="invalid-meminfo-fallback"),
    pytest.param(None, 12, None, True, id="missing-meminfo-fallback"),
    pytest.param("MemAvailable: 0 kB\n", 12, None, False, id="no-available-memory"),
])
def test_host_replay_memory_accounts_for_page_cache_and_cgroups(
    tmp_path, meminfo, free_gib, parent_headroom_gib, fits,
):
    gib = 1024 ** 3
    proc = tmp_path / "proc"
    (proc / "self").mkdir(parents=True)
    (proc / "self" / "cgroup").write_text("0::/jobs/benchmark\n")
    if meminfo is not None:
        (proc / "meminfo").write_text(meminfo)
    leaf = tmp_path / "sys" / "fs" / "cgroup" / "jobs" / "benchmark"
    leaf.mkdir(parents=True)
    (leaf / "memory.max").write_text("max\n")
    if parent_headroom_gib is not None:
        (leaf.parent / "memory.max").write_text(str(16 * gib))
        (leaf.parent / "memory.current").write_text(str((16 - parent_headroom_gib) * gib))

    assert _rust.benchmark_host_replay_fits(proc, tmp_path / "sys/fs/cgroup", free_gib * gib) is fits


def test_cuda_saved_replay_batches_score_and_retries_exhaustion_in_order(tmp_path, monkeypatch, capsys):
    levels = {"first": .125, "invalid": .25, "second": .5, "last": .75}
    candidates = tmp_path / "candidates"
    candidates.mkdir()
    for scene_id, level in levels.items():
        dtype = np.float64 if scene_id == "invalid" else np.float32
        tifffile.imwrite(candidates / f"{scene_id}.tif", np.full((4, 4, 3), level, dtype), photometric="rgb")
    caller = get_ident()

    class Catalog:
        scene_ids = tuple(levels)
        quarantine = {}

        def __init__(self, *args, **kwargs):
            pass

        def scene(self, scene_id):
            return SimpleNamespace(id=scene_id, frame_ids=(), reference_ids=(scene_id,), metadata={
                "dataset_id": "fixture", "leakage_group_id": scene_id, "recipe_sha256": "fixture",
                "reference_type": "rendered_all_in_focus",
            })

        def frame(self, frame_id):
            return SimpleNamespace(id=frame_id, source_sha256="fixture", asset_sha256="fixture", page=0)

        def decode(self, frame_id):
            assert get_ident() == caller
            return SimpleNamespace(data=np.full((4, 4, 3), levels[frame_id], np.float32), alpha=None)

    barrier = Barrier(2, timeout=5)
    second_finished = Event()
    checked = []
    committed = []

    def fits(scene_ids, device):
        assert device == "cuda"
        checked.append(scene_ids)
        return True

    def measure_delivery(result, reference, *, details, device, generate, retry_on_oom):
        level = float(result[0, 0, 0])
        assert result.dtype == np.float32 and (result == reference).all()
        if details == "full":
            assert retry_on_oom and get_ident() == caller
        elif not retry_on_oom:
            assert get_ident() != caller and level in (.125, .5)
            barrier.wait()
            if level == .125:
                try:
                    raise MemoryError("parallel exhaustion")
                except MemoryError as exc:
                    raise RuntimeError("inspection failed") from exc
        else:
            assert get_ident() == caller and second_finished.is_set() and level in (.125, .75)
        generate(reference)
        if not retry_on_oom:
            second_finished.set()
        explanation = None if details is None else {"level": level}
        return SimpleNamespace(score=level * 100, alignment={}, features={}, explanation=explanation)

    original_encode = bench.encode_tiff

    def encode(path, pixels):
        assert get_ident() == caller
        committed.append(path.stem)
        original_encode(path, pixels)

    native_benchmark = _rust.benchmark

    def benchmark_with_memory(args, bridge):
        return native_benchmark(args, bridge, replay_guard=fits)

    def generate(self, reference):
        print("model stdout")
        return reference[..., 0], sha256(reference.tobytes()).hexdigest()

    monkeypatch.setattr(bench, "SceneCatalog", Catalog)
    monkeypatch.setattr(_rust, "benchmark", benchmark_with_memory)
    monkeypatch.setattr(bench, "_measure_delivery", measure_delivery)
    monkeypatch.setattr(bench, "encode_tiff", encode)
    monkeypatch.setattr(Inspection, "generate", generate)
    args = ["--root", str(tmp_path), "--candidate-dir", str(candidates), "--device", "cuda", "--save-images"]
    output = tmp_path / "report"
    assert main([*args, "--output", str(output)]) == 1
    captured = capsys.readouterr()
    assert captured.out == "34.375000\n"
    assert captured.err.count("model stdout") == 3
    assert checked == [["first"], ["first", "second"], ["last"]]
    assert committed == ["first", "second", "last"]
    rows = json.loads((output / "report.json").read_text())["scenes"]
    assert [row["scene_id"] for row in rows] == list(levels)
    assert [row["status"] for row in rows] == ["ok", "invalid_output", "ok", "ok"]
    for row in rows:
        scene_id = row["scene_id"]
        if row["status"] == "ok":
            assert row["scene_score"] == levels[scene_id] * 100
            assert (output / row["image"]).is_file()

    checked.clear()
    committed.clear()
    explained = tmp_path / "explained"
    assert main([*args, "--output", str(explained), "--explain-score"]) == 1
    assert checked == []
    assert committed == ["first", "second", "last"]
    rows = json.loads((explained / "report.json").read_text())["scenes"]
    for row in rows:
        if row["status"] == "ok":
            detail = json.loads((explained / row["score_explanation_path"]).read_text())
            assert detail["level"] == levels[row["scene_id"]]
    assert capsys.readouterr().out == "34.375000\n"

    for path in candidates.glob("*.tif"):
        path.unlink()
    checked.clear()
    assert main([*args, "--output", str(tmp_path / "missing")]) == 1
    assert checked == []
    assert capsys.readouterr().out == "0.000000\n"


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
    assert measure(tifffile.imread(saved), reference, attention=False).alignment == replay_row["alignment"]
    assert score_saved_output(catalog, catalog_scene, saved, attention=False)["alignment"] == replay_row["alignment"]

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


def test_score_keeps_report_order_and_counts_shared_group_ids_once():
    rows = [
        {"dataset_id": dataset, "leakage_group_id": group, "scene_score": score, "reference_status": "eligible"}
        for dataset, group, score in (("z", "b", 100), ("a", "b", 50), ("z", "a", 20), ("z", "b", 80))
    ]
    explanation = {}
    assert _score(rows, explanation=explanation) == (52.5, 2)
    assert list(explanation["datasets"]) == ["z", "a"]
    assert list(explanation["datasets"]["z"]["groups"]) == ["b", "a"]
    assert explanation["datasets"]["z"] == {
        "score": 55, "weight": 0.5,
        "groups": {"b": {"score": 90, "weight": 0.5, "scene_count": 2},
                   "a": {"score": 20, "weight": 0.5, "scene_count": 1}},
    }
