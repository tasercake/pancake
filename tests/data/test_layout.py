import hashlib
import json
import subprocess
import sys

from PIL import Image
import pytest


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


def run_data(root, action, *extra):
    return subprocess.run([sys.executable, "-m", "bokehforge", "data", action,
        "--root", str(root), "--config", str(root / "sources.json"), "--lock", str(root / "sources.lock.json"),
        *extra], capture_output=True, text=True)


@pytest.mark.parametrize("action,report", [("download", "download-report.json"), ("process", "process-report.json")])
def test_commands_write_logs_and_reports_only_inside_hidden_directory(tmp_path, action, report):
    root = source_fixture(tmp_path)
    result = run_data(root, action)
    assert result.returncode == 0, result.stdout + result.stderr
    assert (root / ".bokehforge/reports" / report).is_file()
    log = (root / ".bokehforge/logs" / f"{action}.log").read_text()
    assert result.stdout.strip() in log
    assert not list(root.glob("*.log")) and not list(root.glob("*-report.json"))
    assert (root / ".bokehforge/locks/data.lock").is_file()
    if action == "download":
        assert (root / ".bokehforge/state/download-plan.json").is_file()
        manifest = json.loads((root / "raw/demo/download-manifest.json").read_text())
        assert manifest["selection"] == ".bokehforge/state/download-plan.json"
    else:
        assert (root / ".bokehforge/work/conversion-state").is_dir()
        assert (root / "standardized/index.json").is_file()


def test_logs_append_and_reports_remain_idempotent(tmp_path):
    root = source_fixture(tmp_path)
    assert run_data(root, "download").returncode == 0
    report = root / ".bokehforge/reports/download-report.json"
    first = (report.read_bytes(), report.stat().st_mtime_ns)
    log = root / ".bokehforge/logs/download.log"
    initial = log.read_bytes()
    assert run_data(root, "download").returncode == 0
    assert (report.read_bytes(), report.stat().st_mtime_ns) == first
    assert log.read_bytes().startswith(initial) and log.stat().st_size > len(initial)


def test_fatal_processing_error_is_saved_in_log(tmp_path):
    root = source_fixture(tmp_path)
    (root / "sources.lock.json").unlink()
    result = run_data(root, "process")
    assert result.returncode == 2
    assert "ERROR:" in result.stderr
    log = (root / ".bokehforge/logs/process.log").read_text()
    assert "ERROR:" in log and "sources.lock.json" in log


def test_shared_data_lock_prevents_concurrent_data_commands(tmp_path):
    import fcntl
    root = source_fixture(tmp_path)
    lock = root / ".bokehforge/locks/data.lock"
    lock.parent.mkdir(parents=True)
    with lock.open("a") as handle:
        fcntl.flock(handle, fcntl.LOCK_EX | fcntl.LOCK_NB)
        result = run_data(root, "download")
    assert result.returncode == 2
    assert "Another process holds" in result.stderr
    assert not (root / ".bokehforge/state/download-plan.json").exists()


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
