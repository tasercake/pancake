import os
import json
from pathlib import Path
import subprocess
import sys

import pytest


@pytest.mark.parametrize("arguments,expected", [
    (["--help"], ["bokehforge", "data", "bench", "review", "Stackroom", "heatmaps"]),
    (["review", "--help"], ["bokehforge review", "Stackroom", "--root", "--store", "--host", "--no-attention"]),
    (["heatmaps", "--help"], ["bokehforge heatmaps", "--host", "--port"]),
    (["bench", "--help"], ["bokehforge bench", "--algorithm", "--candidate-dir", "--no-attention"]),
    (["data", "--help"], ["download"]),
    (["data", "download", "--help"], ["bokehforge data download", "--dataset"]),
])
def test_cli_exposes_data_commands(arguments, expected):
    result = subprocess.run([sys.executable, "-m", "bokehforge", *arguments], capture_output=True, text=True)
    assert result.returncode == 0, result.stderr
    assert all(value in result.stdout for value in expected)
    assert not result.stderr
    assert "\x1b" not in result.stdout


@pytest.mark.parametrize("arguments, heading, option", [
    ([], "Commands", "bench"),
    (["data"], "Data commands", "download"),
    (["bench"], "Stack source", "--algorithm"),
    (["bench"], "Scene selection", "--scene"),
    (["bench"], "Report", "--output"),
    (["data", "download"], "Dataset selection", "--dataset"),
    (["data", "download"], "Download limits", "--workers"),
    (["review"], "Connection", "--host"),
    (["review"], "Stackers", "--pancake"),
])
def test_cli_groups_controls_by_task(arguments, heading, option):
    result = subprocess.run(
        [sys.executable, "-m", "bokehforge", *arguments, "--help"], capture_output=True, text=True,
    )
    assert result.returncode == 0, result.stderr
    _, label, rest = result.stdout.partition(f"\n{heading}:\n")
    assert label, result.stdout
    assert option in rest.partition("\n\n")[0]



@pytest.mark.parametrize("arguments", [["unknown"], ["inspection"], ["data", "unknown"], ["data", "process"]])
def test_unknown_command_fails(arguments):
    result = subprocess.run([sys.executable, "-m", "bokehforge", *arguments], capture_output=True, text=True)
    assert result.returncode == 2
    assert "invalid choice" in result.stderr


def test_installed_entry_point_works_outside_repository(tmp_path):
    executable = Path(sys.executable).with_name("bokehforge")
    assert executable.is_file()
    environment = os.environ.copy()
    environment.pop("PYTHONPATH", None)
    result = subprocess.run([str(executable), "data", "--help"], cwd=tmp_path, env=environment,
                            capture_output=True, text=True)
    assert result.returncode == 0, result.stderr
    assert "download" in result.stdout and "process" not in result.stdout


def test_default_source_config_works_outside_repository(tmp_path):
    executable = Path(sys.executable).with_name("bokehforge")
    environment = os.environ.copy()
    environment.pop("PYTHONPATH", None)
    lock = tmp_path / "sources.lock.json"
    lock.write_text(json.dumps({"datasets": [
        {"id": "D29", "status": "available", "layout": "infinigen_defocus", "files": []},
        {"id": "D02", "status": "resolve_failed", "layout": "sood_raw", "files": []},
    ]}))
    result = subprocess.run([str(executable), "data", "download", "--resolve-only",
        "--root", str(tmp_path / "data"), "--lock", str(lock)], cwd=tmp_path, env=environment,
        capture_output=True, text=True)
    assert result.returncode == 0, result.stderr
    sources = {d["id"]: d for d in json.loads(lock.read_text())["datasets"]}
    assert sources["D29"]["status"] == "available"
    assert sources["D29"]["layout"] == "infinigen_defocus"
    assert sources["D02"]["status"] == "resolve_failed"


def test_nested_project_runs_from_repository_root():
    repository = next(parent for parent in Path(__file__).resolve().parents if (parent / ".git").exists())
    project = repository / "bokehforge"
    result = subprocess.run(["uv", "run", "--project", str(project), "--frozen", "--offline",
                             "bokehforge", "data", "--help"], cwd=repository, capture_output=True, text=True)
    assert result.returncode == 0, result.stderr
    assert "download" in result.stdout and "process" not in result.stdout


def test_default_download_target_is_independent_of_checkout(tmp_path):
    executable = Path(sys.executable).with_name("bokehforge")
    target = tmp_path / "shared"
    environment = {**os.environ, "BOKEHFORGE_DATA_ROOT": str(target)}
    environment.pop("PYTHONPATH", None)
    for directory in (tmp_path / "worktree-a", tmp_path / "worktree-b"):
        directory.mkdir()
        result = subprocess.run([str(executable), "data", "download", "--resolve-only"],
                                cwd=directory, env=environment, capture_output=True, text=True)
        assert result.returncode == 0, result.stderr
        assert not list(directory.iterdir())
    sources = json.loads((target / ".bokehforge/sources.lock.json").read_text())["datasets"]
    assert [source["id"] for source in sources] == ["D29"]
    assert all(file["sha256"] for file in sources[0]["files"])
