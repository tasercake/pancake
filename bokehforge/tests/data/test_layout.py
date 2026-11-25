import hashlib
import json
from shutil import copytree
import subprocess
import sys

from PIL import Image
import pytest

from bokehforge import FrameWindowDataset, SceneCatalog


def source_fixture(tmp_path):
    root = tmp_path / "datasets"
    files = []
    for number in range(2):
        path = root / "raw/demo" / f"scene/{number}.png"
        path.parent.mkdir(parents=True, exist_ok=True)
        Image.new("RGB", (8, 8), (100 + number, 120, 140)).save(path)
        files.append({"path": f"scene/{number}.png", "size": path.stat().st_size,
                      "sha256": hashlib.sha256(path.read_bytes()).hexdigest()})
    source = {"id": "demo", "status": "available", "provider": "files", "layout": "directory_stacks",
              "transfer": "srgb", "files": files}
    (root / "sources.json").write_text(json.dumps({"defaults": {"reserve_gib": 0}, "datasets": [source]}))
    (root / "sources.lock.json").write_text(json.dumps({"datasets": [source]}))
    return root


def run_download(root, *args):
    return subprocess.run([sys.executable, "-m", "bokehforge", "data", "download",
        "--root", str(root), "--config", str(root / "sources.json"), "--lock", str(root / "sources.lock.json"), *args],
        capture_output=True, text=True)


def test_download_outputs_feed_the_library_directly(tmp_path):
    root = source_fixture(tmp_path)
    result = run_download(root)
    assert result.returncode == 0, result.stdout + result.stderr
    assert (root / ".bokehforge/reports/download-report.json").is_file()
    log = (root / ".bokehforge/logs/download.log").read_text()
    assert result.stdout.strip() in log
    assert not list(root.glob("*.log")) and not list(root.glob("*-report.json"))
    assert (root / ".bokehforge/locks/data.lock").is_file()
    manifest = json.loads((root / "raw/demo/download-manifest.json").read_text())
    assert len(manifest["files"]) == 2
    before = set(root.rglob("*"))
    catalog = SceneCatalog(root, config=root / "sources.json")
    frames = FrameWindowDataset(catalog, window_size=2)[0]["frames"]
    assert len(frames) == 2 and frames[0].shape == (8, 8, 3)
    assert set(root.rglob("*")) == before


def test_logs_append_and_reports_remain_idempotent(tmp_path):
    root = source_fixture(tmp_path)
    assert run_download(root).returncode == 0
    report = root / ".bokehforge/reports/download-report.json"
    first = (report.read_bytes(), report.stat().st_mtime_ns)
    log = root / ".bokehforge/logs/download.log"
    initial = log.read_bytes()
    assert run_download(root).returncode == 0
    assert (report.read_bytes(), report.stat().st_mtime_ns) == first
    assert log.read_bytes().startswith(initial) and log.stat().st_size > len(initial)


def test_old_lock_only_downloads_sources_in_active_configuration(tmp_path):
    root = source_fixture(tmp_path)
    lock = root / "sources.lock.json"
    pinned = json.loads(lock.read_text())
    retired = {"id": "D09", "status": "available", "layout": "mfi_whu", "files": []}
    inactive = {**pinned["datasets"][0], "id": "inactive"}
    pinned["datasets"].append(inactive)
    old_lock = json.dumps({"datasets": [*pinned["datasets"], retired]})
    lock.write_text(old_lock)

    result = run_download(root, "--dataset", "D09")
    assert result.returncode == 2
    assert "Unknown dataset IDs: ['D09']" in result.stderr
    assert not (root / "raw/D09").exists()
    lock.write_text(old_lock)
    result = run_download(root)
    assert result.returncode == 0, result.stdout + result.stderr
    assert json.loads(lock.read_text()) == pinned
    assert not (root / "raw/inactive").exists()
    report = json.loads((root / ".bokehforge/reports/download-report.json").read_text())
    assert [row["id"] for row in report["datasets"]] == ["demo"]

    config = root / "sources.json"
    policy = json.loads(config.read_text())
    policy["datasets"].append(inactive)
    config.write_text(json.dumps(policy))
    copytree(root / "raw/demo/scene", root / "raw/inactive/scene")
    result = run_download(root)
    assert result.returncode == 0, result.stdout + result.stderr
    for source_lock in (None, lock):
        catalog = SceneCatalog(root, lock=source_lock, config=config)
        assert {catalog.scene(scene_id).metadata["dataset_id"] for scene_id in catalog.scene_ids} == {"demo", "inactive"}
    assert not (root / "raw/D09").exists()


def test_catalog_ignores_retired_source_from_old_manifests_and_locks(tmp_path):
    root = source_fixture(tmp_path)
    assert run_download(root).returncode == 0
    expected_scenes = SceneCatalog(root, config=root / "sources.json").scene_ids
    retired = {"id": "D09", "status": "available", "layout": "mfi_whu",
               "files": [{"path": "MFI-WHU.rar"}]}
    manifest = root / "raw/D09/download-manifest.json"
    manifest.parent.mkdir()
    manifest.write_text(json.dumps(retired))
    lock = root / "sources.lock.json"
    pinned = json.loads(lock.read_text())
    lock.write_text(json.dumps({"datasets": [*pinned["datasets"], retired]}))

    config = root / "sources.json"
    policy = json.loads(config.read_text())
    policy["datasets"].append(retired)
    config.write_text(json.dumps(policy))
    for source_lock in (None, lock):
        assert SceneCatalog(root, lock=source_lock, config=root / "sources.json").scene_ids == expected_scenes
        with pytest.raises(ValueError, match="Unknown or undownloaded datasets"):
            SceneCatalog(root, datasets="D09", lock=source_lock, config=root / "sources.json")


def test_data_lock_prevents_concurrent_downloads(tmp_path):
    import fcntl
    root = source_fixture(tmp_path)
    lock = root / ".bokehforge/locks/data.lock"
    lock.parent.mkdir(parents=True)
    with lock.open("a") as handle:
        fcntl.flock(handle, fcntl.LOCK_EX | fcntl.LOCK_NB)
        result = run_download(root)
    assert result.returncode == 2
    assert "Another process holds" in result.stderr
    assert not (root / ".bokehforge/reports/download-report.json").exists()


def test_data_directory_can_be_reset_without_losing_source_policy(tmp_path):
    import shutil
    config = tmp_path / "config"
    config.mkdir()
    policy = config / "sources.json"
    policy.write_text(json.dumps({"defaults": {"data_root": "../datasets"},
        "datasets": [{"id": "disabled", "provider": "disabled", "reason": "Excluded"}]}))
    lock = config / "sources.lock.json"
    command = [sys.executable, "-m", "bokehforge", "data", "download", "--config", str(policy),
               "--lock", str(lock), "--resolve-only"]
    first = subprocess.run(command, capture_output=True, text=True)
    assert first.returncode == 0, first.stderr
    root = tmp_path / "datasets"
    assert (root / ".bokehforge/logs/download.log").is_file()
    pinned = lock.read_bytes()
    shutil.rmtree(root)
    second = subprocess.run(command, capture_output=True, text=True)
    assert second.returncode == 0, second.stderr
    assert lock.read_bytes() == pinned and policy.is_file()
    assert (root / ".bokehforge/logs/download.log").is_file()
    assert {p.name for p in root.iterdir()} == {".bokehforge"}
