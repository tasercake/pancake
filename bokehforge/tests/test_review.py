from argparse import Namespace
import csv
from hashlib import sha256
from http.client import HTTPConnection
from io import BytesIO, StringIO
import json
from shutil import copyfile
import sys
from threading import Thread
from time import monotonic, sleep

import numpy as np
from PIL import Image
import pytest
import tifffile

from bokehforge.review import runs
from bokehforge.review.server import Review, ReviewServer


def write_scene(root, level):
    raw = root / "raw/Demo"
    files = {}
    for scene in ("reviewed", "other"):
        for index in range(2):
            relative = f"{scene}/frame{index}.tif"
            image = raw / relative
            image.parent.mkdir(parents=True, exist_ok=True)
            tifffile.imwrite(image, np.full((6, 8, 3), level + index * 0.1, dtype=np.float32),
                             photometric="rgb")
            files[relative] = sha256(image.read_bytes()).hexdigest()
    (raw / "download-manifest.json").write_text(json.dumps({
        "id": "Demo", "status": "available", "layout": "directory_stacks",
        "transfer": "linear", "reference_type": "none",
        "files": [{"path": name, "sha256": digest} for name, digest in files.items()],
    }))


def open_review(root, store):
    store.mkdir(exist_ok=True)
    app = Review(Namespace(root=root, store=store, dataset=None, config=None, lock=None,
                           pancake=sys.executable, focus_stack=sys.executable,
                           openfocus=None, shine_python=None))
    deadline = monotonic() + 5
    while app.catalog_status["status"] == "indexing" and monotonic() < deadline:
        sleep(0.005)
    assert app.catalog_status["status"] == "ready", app.catalog_status
    return app


def test_evaluation_retains_per_stacker_failures_and_canonical_outputs(tmp_path, monkeypatch):
    def render(tool, frames, work, log, stop):
        log.parent.mkdir(parents=True, exist_ok=True)
        log.write_text("fixture stacker output")
        if tool.id == "focus-stack":
            raise RuntimeError("example exit")
        staged = tifffile.imread(work / frames[0])
        assert staged.dtype == np.uint16
        assert np.all(staged > np.uint16(0.25 * 65535))
        delivery = work / "generated.tif"
        copyfile(work / frames[0], delivery)
        return delivery

    monkeypatch.setattr(runs, "_render", render)
    root, store = tmp_path / "data", tmp_path / "review"
    write_scene(root, 0.25)
    app = open_review(root, store)
    other, scene = [item["id"] for item in app.scenes]
    job = app.start({"scope": "scene", "scene": scene, "stackers": ["pancake", "focus-stack"]})
    app.runs.worker.join(timeout=10)
    assert not app.runs.worker.is_alive()
    summary, = app.runs.list()
    assert (summary["status"], summary["ready"], summary["failed"]) == ("completed_with_errors", 1, 1)

    inspected = app.scene_details(scene, job)
    assert not inspected["references"]
    assert len(inspected["frames"]) == 2
    successful, failed = inspected["outputs"]
    assert successful["status"] == "ready"
    assert failed["status"] == "failed" and failed["error"] == "example exit"
    assert all(row["status"] == "outside_scope" for row in app.scene_details(other, job)["outputs"])

    choice = {"run": job, "scene": scene, "stacker": "pancake"}
    path = app.artifact(choice, "image")
    assert path == store / "runs" / job / "pancake/images" / (scene + ".tif")
    pixels = tifffile.imread(path)
    assert pixels.dtype == np.float32
    assert np.allclose(pixels, 0.25, atol=2e-5)
    mime, preview = app.image({**choice, "size": "native"})
    assert mime == "image/png"
    assert Image.open(BytesIO(preview)).size == (8, 6)

    with pytest.raises(KeyError):
        app.rate({**choice, "stacker": "focus-stack", "grade": 5})
    with pytest.raises(ValueError, match="1 to 5"):
        app.rate({**choice, "grade": True})
    app.rate({**choice, "grade": 2})
    comment = 'Sharp edges,\nslight "halo"'
    app.rate({**choice, "grade": 4, "comment": comment})

    reopened = open_review(root, store)
    rating = reopened.scene_details(scene, job)["outputs"][0]["rating"]
    exported, = csv.DictReader(StringIO(reopened.ratings.export_csv()))
    assert rating["grade"] == 4 and rating["comment"] == comment
    assert exported["run_id"] == job and exported["scene_id"] == scene
    assert exported["stacker"] == "pancake" and exported["grade"] == "4" and exported["comment"] == comment

    write_scene(root, 0.5)
    changed = open_review(root, store)
    stale = changed.scene_details(scene, job)["outputs"][0]
    assert stale["status"] == "different_source" and stale["rating"] is None
    with pytest.raises(KeyError):
        changed.rate({**choice, "grade": 5})


def test_review_accepts_machine_hostname_and_same_http_origin(tmp_path, monkeypatch):
    monkeypatch.setattr("bokehforge.review.server.gethostname", lambda: "review-node")
    root = tmp_path / "data"
    write_scene(root, 0.25)
    with ReviewServer(("0.0.0.0", 0)) as server:
        server.app = open_review(root, tmp_path / "review")
        worker = Thread(target=server.serve_forever, kwargs={"poll_interval": 0.02})
        worker.start()
        port = server.server_port

        def request(host, origin=None):
            headers = {"Host": host}
            if origin is not None:
                headers["Origin"] = origin
            client = HTTPConnection("127.0.0.1", port, timeout=3)
            try:
                client.request("GET", "/api/catalog", headers=headers)
                response = client.getresponse()
                return response.status, response.read()
            finally:
                client.close()

        try:
            authority = f"review-node:{port}"
            status, body = request(authority, f"http://{authority}")
            assert status == 200 and json.loads(body)["status"] == "ready"
            assert request(f"127.0.0.1:{port}")[0] == 200
            assert request(f"localhost:{port}")[0] == 200
            assert request(f"other-site.test:{port}")[0] == 403
            assert request(authority, f"http://other-site.test:{port}")[0] == 403
        finally:
            server.shutdown()
            worker.join()
