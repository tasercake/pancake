from pathlib import Path
import sys
from threading import Event
from types import SimpleNamespace

import numpy as np
import tifffile

from bokehforge.review import runs
from bokehforge.review.stacker_setup import StackerSetup


def test_evaluation_prepares_missing_tools_and_keeps_independent_results(tmp_path, monkeypatch):
    devices = {
        "focus-stack": runs.Stacker("focus-stack", "focus-stack", "Wavelet", ()),
        "openfocus": runs.Stacker("openfocus", "OpenFocus", "StackMFF V3", ()),
    }
    batches = runs.Runs(tmp_path, devices)
    scene = SimpleNamespace(id="Demo/scene", frame_ids=(), reference_ids=(),
                            metadata={"recipe_sha256": "test", "recipe": {"loader_sha256": "test"}})
    catalog = SimpleNamespace(scene=lambda scene_id: scene)
    status = []

    def setup(tool, log, stop, tell):
        assert not stop.is_set()
        tell(f"Installing {tool.label}")
        status.append(batches.list()[0]["setup"])
        if tool.id == "focus-stack":
            raise RuntimeError("test setup failure")
        return sys.executable

    def render(tool, frames, work, log, stop):
        assert tool.id == "openfocus"
        assert tool.executable == sys.executable
        path = work / "rendered.tif"
        tifffile.imwrite(path, np.full((3, 4, 3), 128, dtype=np.uint8), photometric="rgb")
        return path

    monkeypatch.setattr(batches.setup, "prepare", setup)
    monkeypatch.setattr(runs, "_render", render)
    assert not any(tool["available"] for tool in batches.tooling())
    run_id = batches.start(catalog, [scene.id], list(devices), {"type": "scene", "label": "Scene"})
    batches.worker.join(timeout=5)
    assert not batches.worker.is_alive()
    assert status == [
        {"stacker": "focus-stack", "message": "Installing focus-stack"},
        {"stacker": "openfocus", "message": "Installing OpenFocus"},
    ]
    summary, = batches.list()
    assert summary["status"] == "completed_with_errors"
    assert (summary["ready"], summary["failed"], summary["total"]) == (1, 1, 2)
    failed, ready = batches.scene_results(run_id, catalog, scene)
    assert failed["status"] == "failed"
    assert "test setup failure" in failed["error"]
    assert "test setup failure" in (batches.directory(run_id) / failed["log"]).read_text()
    assert ready["status"] == "ready"


def test_setup_reuses_checked_tool_without_running_installer(tmp_path, monkeypatch):
    setup = StackerSetup(tmp_path, repo=tmp_path)
    setup.pins = {".submodules/OpenFocus": "abc"}
    tool = runs.Stacker("openfocus", "OpenFocus", "V3", ())
    installs = []

    def install(tool, directory, log, stop, tell):
        installs.append(tool.id)
        return directory / "bin" / "openfocus"

    monkeypatch.setattr(setup, "_python_stacker", install)
    monkeypatch.setattr("bokehforge.review.stacker_setup.run_logged", lambda *args, **kwargs: None)
    monkeypatch.setattr("bokehforge.review.stacker_setup.executable", lambda *candidates: str(candidates[0]) if candidates else None)

    assert setup.available(tool) is None
    log = tmp_path / "setup.log"
    expected = tmp_path / "tools/openfocus/abc/bin/openfocus"
    assert setup.prepare(tool, log, Event(), lambda message: None) == str(expected)
    assert setup.prepare(tool, log, Event(), lambda message: None) == str(expected)
    assert installs == ["openfocus"]
    assert Path(expected.parents[1], "ready").read_text() == "bin/openfocus"
