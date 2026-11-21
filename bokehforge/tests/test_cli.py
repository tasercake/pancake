import os
import json
from pathlib import Path
import subprocess
import sys

import pytest


@pytest.mark.parametrize("arguments,expected", [
    (["--help"], ["bokehforge", "data"]),
    (["data", "--help"], ["download", "process"]),
    (["data", "download", "--help"], ["bokehforge data download", "--dataset"]),
    (["data", "process", "--help"], ["bokehforge data process", "--inventory-only", "--dataset"]),
])
def test_cli_exposes_data_commands(arguments, expected):
    result = subprocess.run([sys.executable, "-m", "bokehforge", *arguments], capture_output=True, text=True)
    assert result.returncode == 0, result.stderr
    assert all(value in result.stdout for value in expected)


@pytest.mark.parametrize("arguments", [["unknown"], ["data", "unknown"]])
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
    assert "download" in result.stdout and "process" in result.stdout


def test_default_source_config_works_outside_repository(tmp_path):
    executable = Path(sys.executable).with_name("bokehforge")
    environment = os.environ.copy()
    environment.pop("PYTHONPATH", None)
    lock = tmp_path / "sources.lock.json"
    lock.write_text(json.dumps({"datasets": [{"id": "D29", "status": "available",
                                              "layout": "infinigen_defocus", "files": []}]}))
    result = subprocess.run([str(executable), "data", "download", "--dataset", "D29", "--resolve-only",
        "--root", str(tmp_path / "data"), "--lock", str(lock)], cwd=tmp_path, env=environment,
        capture_output=True, text=True)
    assert result.returncode == 0, result.stderr
    source = next(d for d in json.loads(lock.read_text())["datasets"] if d["id"] == "D29")
    assert source["status"] == "available"
    assert source["layout"] == "infinigen_defocus"


def test_nested_project_runs_from_repository_root():
    repository = next(parent for parent in Path(__file__).resolve().parents if (parent / ".git").exists())
    project = repository / "bokehforge"
    result = subprocess.run(["uv", "run", "--project", str(project), "--frozen", "--offline",
                             "bokehforge", "data", "--help"], cwd=repository, capture_output=True, text=True)
    assert result.returncode == 0, result.stderr
    assert "download" in result.stdout and "process" in result.stdout
    config = project / "config/data/sources.json"
    default_root = json.loads(config.read_text())["defaults"]["data_root"]
    assert (config.parent / default_root).resolve() == repository / "datasets"
