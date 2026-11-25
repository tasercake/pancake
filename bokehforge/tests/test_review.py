from argparse import Namespace
import csv
from hashlib import sha256
from http.client import HTTPConnection
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from io import BytesIO, StringIO
from pathlib import Path
import json
from shutil import copyfile
import sys
from threading import Event, Thread
from time import sleep
import xml.etree.ElementTree as ET

import numpy as np
from PIL import Image
import pytest
import tifffile

from bokehforge.bench import main as bench
from bokehforge.data._canonical import INFINIGEN_DISTANCES
from bokehforge.review import _zerene, runs, scores
from bokehforge.scoring import SCORE_VERSION
from bokehforge.review import server as review_server
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
        "id": "Demo", "name": "Demonstration stacks", "status": "available", "layout": "directory_stacks",
        "transfer": "linear", "reference_type": "none",
        "files": [{"path": name, "sha256": digest} for name, digest in files.items()],
    }))


def event_payload(message):
    name, data, _ = message.split(b"\n", 2)
    return name.removeprefix(b"event: ").decode(), json.loads(data.removeprefix(b"data: "))


class ReviewStream:
    def __init__(self, port):
        self.client = HTTPConnection("127.0.0.1", port, timeout=8)
        self.latest = {}

    def __enter__(self):
        self.client.request("GET", "/api/events")
        self.response = self.client.getresponse()
        assert self.response.status == 200
        assert self.response.headers["Content-Type"].startswith("text/event-stream")
        return self

    def __exit__(self, *_):
        self.response.close()
        self.client.close()

    def expect(self, name, predicate):
        while name not in self.latest or not predicate(self.latest[name]):
            header = self.response.readline()
            assert header, "Review closed the event stream"
            event, value = event_payload(header + self.response.readline() + self.response.readline())
            self.latest[event] = value
        return self.latest[name]


def open_review(root, store):
    store.mkdir(exist_ok=True)
    config = root / "sources.json"
    config.write_text(json.dumps({"datasets": [{"id": "Demo"}]}))
    app = Review(Namespace(root=root, store=store, dataset=None, config=config, lock=None,
                           pancake=sys.executable, focus_stack=sys.executable,
                           openfocus=None, shine_python=None))
    with app.events.subscribe() as (reader, writer):
        reader.settimeout(5)
        while True:
            messages = app.events.take(reader, writer)
            catalog = next((event_payload(message)[1] for message in messages
                            if message.startswith(b"event: catalog\n")), None)
            if catalog is not None and catalog["status"] != "indexing":
                assert catalog["status"] == "ready", catalog
                break
    return app


def test_evaluation_retains_per_stacker_failures_and_canonical_outputs(tmp_path, monkeypatch):
    def render(tool, frames, work, log, stop):
        log.parent.mkdir(parents=True, exist_ok=True)
        log.write_text("fixture stacker output")
        if tool.id == "focus-stack":
            assert tool.options["device"] == "auto"
            raise RuntimeError("example exit")
        assert tool.options == {"device": "cpu", "method": "depth_map", "alignment": "off", "crop": False}
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
    assert app.catalog_status["datasets"] == {"Demo": "Demonstration stacks"}
    other, scene = [item["id"] for item in app.scenes]
    with app.events.subscribe() as (reader, writer):
        reader.settimeout(10)
        job = app.start({"scope": "scene", "scene": scene, "stackers": ["pancake", "focus-stack"],
                         "options": {"pancake": {"method": "depth_map", "device": "cpu"}}})
        app.runs.worker.join(timeout=10)
        assert not app.runs.worker.is_alive()
        summary, = next(event_payload(packet)[1] for packet in app.events.take(reader, writer)
                        if packet.startswith(b"event: runs\n"))
    assert (summary["status"], summary["ready"], summary["failed"]) == ("completed_with_errors", 1, 1)
    assert summary["options"]["pancake"]["device"] == "cpu"
    assert summary["options"]["focus-stack"]["device"] == "auto"
    assert summary["methods"]["pancake"] == "Depth map"

    inspected = app.scene_details(scene, job)
    assert not inspected["references"]
    assert len(inspected["frames"]) == 2
    summary_scene = next(item for item in app.scenes if item["id"] == scene)
    assert summary_scene["first_frame"] == inspected["frames"][0]["id"]
    assert summary_scene["preview_frame"] == inspected["frames"][1]["id"]
    successful, failed = inspected["outputs"]
    assert successful["status"] == "ready"
    unscored = successful["benchmark"]
    assert (unscored["profile"], unscored["status"], unscored["score"]) == (SCORE_VERSION, "unscored", None)
    assert len(unscored["artifact_revision"]) == 64
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

    catalog_scene = app.catalog.scene(scene)
    legacy_recipe = {**catalog_scene.metadata["recipe"],
                     "loader_sha256": "fa756299117c6a728d941742abafe6afab5c88c1ee282e80b102c53e083dedb6"}
    legacy_sha256 = sha256(json.dumps(legacy_recipe, sort_keys=True).encode()).hexdigest()
    result_path = store / "runs" / job / "pancake/results" / (scene + ".json")
    saved = json.loads(result_path.read_text())
    saved["scene_fingerprint"] = runs.scene_fingerprint(app.catalog, catalog_scene, recipe_sha256=legacy_sha256)
    result_path.write_text(json.dumps(saved))

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


def test_review_image_caches_and_revalidates_without_decoding(tmp_path, monkeypatch):
    root, store = tmp_path / "data", tmp_path / "review"
    write_scene(root, 0.25)
    app = open_review(root, store)
    first = app.catalog.scene(app.catalog.scene_ids[0]).frame_ids[0]
    query = {"size": "panel", "frame": first}
    mime, original = app.image(query)
    assert mime in {"image/png", "image/webp"}
    assert Image.open(BytesIO(original)).size == (8, 6)

    warmed = open_review(root, store)

    def unexpected_decode(*_args):
        pytest.fail("A warm preview should not read the original")

    monkeypatch.setattr(warmed.frame_decoder, "decode", unexpected_decode)
    assert warmed.image(query) == (mime, original)

    with ReviewServer(("127.0.0.1", 0)) as server:
        server.app = warmed
        worker = Thread(target=server.serve_forever, kwargs={"poll_interval": 0.02})
        worker.start()
        client = HTTPConnection("127.0.0.1", server.server_port, timeout=3)
        path = f"/api/image?size=panel&frame={first}&v={warmed.image_version}"
        try:
            client.request("GET", path)
            response = client.getresponse()
            assert response.status == 200 and response.read() == original
            assert "private" in response.headers["Cache-Control"]
            assert "immutable" in response.headers["Cache-Control"]
            etag = response.headers["ETag"]

            client.request("GET", path, headers={"If-None-Match": "W/" + etag})
            response = client.getresponse()
            assert response.status == 304 and response.read() == b""
            assert response.headers["ETag"] == etag
        finally:
            client.close()
            server.shutdown()
            worker.join()

    write_scene(root, 0.5)
    changed = open_review(root, store)
    assert changed.image(query)[1] != original


def test_review_downloads_missing_data_while_existing_scenes_remain_open(tmp_path):
    gate = Event()
    stream = BytesIO()
    tifffile.imwrite(stream, np.full((1024, 1024, 3), 128, np.uint8), photometric="rgb")
    source = stream.getvalue()
    chunk = 1024 * 1024

    class Origin(BaseHTTPRequestHandler):
        def do_GET(self):
            self.send_response(200)
            self.send_header("Content-Length", str(len(source)))
            self.end_headers()
            self.wfile.write(source[:chunk])
            sleep(0.6)
            self.wfile.write(source[chunk:2 * chunk])
            if gate.wait(10):
                self.wfile.write(source[2 * chunk:])

        def log_message(self, *args):
            pass

    root = tmp_path / "data"
    store = tmp_path / "review"
    store.mkdir()
    write_scene(root, 0.25)

    with ThreadingHTTPServer(("127.0.0.1", 0), Origin) as origin, ReviewServer(("127.0.0.1", 0)) as server:
        url = f"http://127.0.0.1:{origin.server_port}/frame"
        config = tmp_path / "sources.json"
        config.write_text(json.dumps({"defaults": {"reserve_gib": 0}, "datasets": [
            {"id": "Demo"},
            {"id": "New", "name": "New scenes", "provider": "files", "layout": "directory_stacks",
             "transfer": "srgb_assumed", "reference_type": "none",
             "files": [{"path": f"stack/{name}.tif", "url": url, "size": len(source),
                        "sha256": sha256(source).hexdigest()} for name in ("near", "far")]},
        ]}))
        server.app = Review(Namespace(root=root, store=store, dataset=["Demo", "New"], config=config, lock=None,
                                      pancake=None, focus_stack=None, openfocus=None, shine_python=None))
        origin_thread = Thread(target=origin.serve_forever, daemon=True)
        review_thread = Thread(target=server.serve_forever, daemon=True)
        origin_thread.start()
        review_thread.start()

        def request(path, method="GET"):
            client = HTTPConnection("127.0.0.1", server.server_port, timeout=3)
            try:
                payload = "{}" if method == "POST" else None
                client.request(method, path, body=payload, headers={"Content-Type": "application/json"})
                response = client.getresponse()
                return response.status, json.loads(response.read())
            finally:
                client.close()

        try:
            with ReviewStream(server.server_port) as events:
                before = events.expect("catalog", lambda item: item["status"] == "ready")
                events.expect("download", lambda item: not item["indexing"])
                assert before["missing"] == {"New": "New scenes"}
                assert set(before["datasets"]) == {"Demo"}
                assert request("/api/download", "POST")[0] == 202
                assert request("/api/download", "POST")[0] == 409
                live = events.expect("download", lambda item: item["received"] > 0 and item["rate"])
                assert live["status"] == "running"
                assert live["total"] == len(source) * 2
                assert 0 < live["received"] < live["total"]
                assert live["seconds_remaining"] > 0
                assert not (root / "raw/New/download-manifest.json").exists()
                assert request("/api/scene?scene=" + before["scenes"][0]["id"])[0] == 200

                gate.set()
                after = events.expect("catalog", lambda item: not item["missing"])
                finished = events.expect("download", lambda item: item["status"] == "complete")
                assert set(after["datasets"]) == {"Demo", "New"}
                assert after["revision"] > before["revision"]
                assert finished["received"] == finished["total"]
                assert {item["dataset"] for item in after["scenes"]} == {"Demo", "New"}

            # A reconnect receives snapshots even after the source has gone idle.
            with ReviewStream(server.server_port) as events:
                assert events.expect("catalog", lambda item: item["revision"] == after["revision"]) == after
                assert events.expect("download", lambda item: not item["indexing"])["status"] == "complete"
        finally:
            gate.set()
            server.app.downloads.close()
            server.shutdown()
            origin.shutdown()
            review_thread.join()
            origin_thread.join()


def test_running_review_keeps_one_version_of_static_assets(tmp_path, monkeypatch):
    for filename, _ in review_server.ASSETS.values():
        (tmp_path / filename).write_text("old " + filename)
    monkeypatch.setattr(review_server, "STATIC", tmp_path)

    with ReviewServer(("127.0.0.1", 0)) as server:
        (tmp_path / "app.js").write_text("new app")
        worker = Thread(target=server.serve_forever, kwargs={"poll_interval": 0.02})
        worker.start()
        client = HTTPConnection("127.0.0.1", server.server_port, timeout=3)
        try:
            client.request("GET", "/app.js")
            response = client.getresponse()
            assert response.status == 200
            assert response.read() == b"old app.js"
        finally:
            client.close()
            server.shutdown()
            worker.join()


def test_linux_rejects_zerene_and_invalid_stacker_settings(tmp_path, monkeypatch):
    monkeypatch.setattr(_zerene.platform, "system", lambda: "Linux")
    root, store = tmp_path / "data", tmp_path / "review"
    write_scene(root, 0.25)
    app = open_review(root, store)
    tools = {tool["id"]: tool for tool in app.catalog_data()["stackers"]}
    assert set(tools) == {"pancake", "focus-stack", "openfocus", "shinestacker"}
    for stacker in ("pancake", "focus-stack", "openfocus"):
        assert tools[stacker]["options"]["device"]["default"] == "auto"

    form = {"scope": "scene", "scene": app.scenes[0]["id"], "stackers": ["focus-stack"]}
    with pytest.raises(ValueError, match="supported stacker"):
        app.start({**form, "stackers": ["zerene"]})
    with pytest.raises(ValueError, match="selected stacking programs"):
        app.start({**form, "options": {"pancake": {"device": "cpu"}}})
    with pytest.raises(ValueError, match="Invalid settings"):
        app.start({**form, "options": {"focus-stack": {"flag": "--align-only"}}})
    for strength in ("0.5", True, float("nan"), -1):
        with pytest.raises(ValueError, match="denoise strength"):
            app.start({**form, "options": {"focus-stack": {"denoise": strength}}})
    assert app.runs.list() == []


@pytest.mark.parametrize("system", ["Darwin", "Windows"])
def test_zerene_uses_a_local_batch_and_normalizes_the_output(tmp_path, monkeypatch, system):
    preferences = tmp_path / "Zerene Preferences"
    preferences.mkdir()
    java = preferences / ("java" if system == "Darwin" else "java.exe")
    java.touch()
    launch = preferences / "zerenstk.launchcmd"
    java_launcher = java if system == "Darwin" else java.with_name("javaw.exe")
    recorded = f'"{java_launcher}" -classpath "Local Zerene Jar" {_zerene.MAIN_CLASS} -ignored'
    launch.write_text(recorded, encoding="utf-16" if system == "Windows" else "utf-8")
    monkeypatch.setattr(_zerene.platform, "system", lambda: system)
    monkeypatch.setattr(_zerene, "preferences_directory", lambda: preferences)
    if system == "Windows":
        def windows_argv(text):
            assert text == recorded
            return [str(java_launcher), "-classpath", "Local Zerene Jar", _zerene.MAIN_CLASS, "-ignored"]

        monkeypatch.setattr(_zerene, "_windows_argv", windows_argv)

    def local_java(command, work, log, stop, label, *, timeout):
        assert command[:3] == [str(java), "-classpath", "Local Zerene Jar"]
        assert ("-Dapple.awt.BackgroundOnly=true" in command) == (system == "Darwin")
        assert "-ignored" not in command and timeout == 3600
        batch, first, second = command[command.index("-batchScript") + 1:]
        assert first == str(work / "frames/000000.tif")
        assert second == str(work / "frames/000001.tif")
        assert tifffile.imread(first).dtype == np.uint16
        task = ET.parse(batch).find(".//Task")
        assert task.find("TaskIndicatorCode").attrib["value"] == "2"
        preferences = {entry.tag: entry.attrib["value"] for entry in task.find("Preferences")}
        assert preferences["DepthMapControl.ContrastThresholdPercentile"] == "67.5"
        assert preferences["DepthMapControl.EstimationRadius"] == "13"
        assert preferences["DepthMapControl.SmoothingRadius"] == "5"
        assert preferences["AlignmentControl.AddNewFilesAsAlreadyAligned"] == "true"
        output = Path(task.find("OutputImagesDesignatedFolder").attrib["value"])
        copyfile(first, output / "Zerene_dmap.tif")
        log.parent.mkdir(parents=True)
        log.write_text("Zerene test log")

    monkeypatch.setattr(runs, "run_logged", local_java)
    root, store = tmp_path / "data", tmp_path / "review"
    write_scene(root, 0.25)
    app = open_review(root, store)
    tool = next(tool for tool in app.catalog_data()["stackers"] if tool["id"] == "zerene")
    assert tool["available"] and tool["options"]["method"]["default"] == "pmax"
    scene = app.scenes[0]["id"]
    options = {"method": "dmap", "align": False, "dmap_percentile": 67.5, "dmap_estimation_radius": 13}
    job = app.start({"scope": "scene", "scene": scene, "stackers": ["zerene"], "options": {"zerene": options}})
    app.runs.worker.join(timeout=5)
    assert not app.runs.worker.is_alive()
    summary, = app.runs.list()
    assert summary["status"] == "completed", summary
    assert summary["options"]["zerene"] == {**options, "dmap_smoothing_radius": 5}
    choice = {"run": job, "scene": scene, "stacker": "zerene"}
    pixels = tifffile.imread(app.artifact(choice, "image"))
    assert pixels.dtype == np.float32 and np.allclose(pixels, 0.25, atol=2e-5)
    app.rate({**choice, "grade": 4})
    result, = app.scene_details(scene, job)["outputs"]
    assert result["rating"]["grade"] == 4


def test_review_pushes_and_caches_the_default_benchmark_score(tmp_path, monkeypatch):
    root, store = tmp_path / "data", tmp_path / "review"
    raw = root / "raw/Demo"
    folder = raw / "a1_0"
    folder.mkdir(parents=True)
    inputs = [f"image_ap_1.40_fd_{distance:.2f}.png" for distance in INFINIGEN_DISTANCES]
    files = []
    for name in [*inputs, "image_all_in_focus.png"]:
        image = folder / name
        Image.fromarray(np.full((6, 8, 3), 64, dtype=np.uint8)).save(image)
        files.append({"path": image.relative_to(raw).as_posix(), "sha256": sha256(image.read_bytes()).hexdigest()})
    (raw / "download-manifest.json").write_text(json.dumps({
        "id": "Demo", "status": "available", "layout": "infinigen_defocus", "transfer": "linear",
        "reference_type": "rendered_all_in_focus", "files": files,
    }))

    def render(tool, frames, work, log, stop):
        delivery = work / "generated.tif"
        tifffile.imwrite(delivery, np.full((6, 8, 3), 32000, dtype=np.uint16), photometric="rgb")
        return delivery

    gate = Event()
    calls = []
    score_saved = scores.score_saved_output

    def delayed_score(catalog, scene, path, *, details):
        calls.append(details)
        assert gate.wait(5)
        return score_saved(catalog, scene, path, details=details)

    monkeypatch.setattr(runs, "_render", render)
    monkeypatch.setattr(scores, "score_saved_output", delayed_score)
    app = open_review(root, store)
    reopened = None
    try:
        scene = app.scenes[0]["id"]
        run = app.start({"scope": "scene", "scene": scene, "stackers": ["pancake"]})
        app.runs.worker.join(timeout=5)
        assert not app.runs.worker.is_alive()
        with app.events.subscribe() as (reader, writer):
            reader.settimeout(5)
            result, = app.scene_details(scene, run)["outputs"]
            assert result["status"] == "ready"
            assert result["benchmark"]["status"] == "pending"
            assert app.scene_details(scene, run)["outputs"][0]["benchmark"]["status"] == "pending"
            gate.set()
            while ("scores", 1) not in map(event_payload, app.events.take(reader, writer)):
                pass

        result, = app.scene_details(scene, run)["outputs"]
        score = result["benchmark"]["score"]
        assert 0 < score < 100
        assert result["benchmark"]["profile"] == SCORE_VERSION
        assert calls == ["summary"]
        comparison = result["benchmark"]["explanation"]["placements"][0]["comparison"]
        assert len(comparison["families"]) == 6 and "tile_grids" not in comparison
        path = app.artifact({"run": run, "scene": scene, "stacker": "pancake"}, "image")
        report = tmp_path / "benchmark"
        assert bench(["--root", str(root), "--config", str(root / "sources.json"), "--candidate-dir",
                      str(path.parents[1]), "--output", str(report)]) == 0
        benchmark_row, = json.loads((report / "report.json").read_text())["scenes"]
        assert benchmark_row["scene_score"] == score

        reopened = open_review(root, store)
        assert reopened.scene_details(scene, run)["outputs"][0]["benchmark"] == result["benchmark"]
        assert reopened.scores.worker is None

        query = {"run": run, "scene": scene, "stacker": "pancake",
                 "rev": result["benchmark"]["artifact_revision"], "family": "tone", "side": "8"}
        with reopened.events.subscribe() as (reader, writer):
            reader.settimeout(5)
            assert reopened.score_tiles(query)["status"] == "pending"
            while ("scores", 1) not in map(event_payload, reopened.events.take(reader, writer)):
                pass
        tiles = reopened.score_tiles(query)
        assert tiles["artifact_revision"] == query["rev"]
        assert tiles["family"] == "tone" and tiles["grid"]["x_edges"] == [0, 8]
        assert len(tiles["grid"]["values"]) == 1 and tiles["grid"]["values"][0] > 0
        assert "grids" not in tiles and "explanation" not in tiles
        other = reopened.score_tiles({**query, "family": "color", "side": "32"})
        assert other["family"] == "color" and other["grid"]["tile_size"] == 32
        assert calls == ["summary", "tiles"]

        crop = path.with_suffix(".replacement.tif")
        tifffile.imwrite(crop, tifffile.imread(path)[:, :-1], photometric="rgb")
        crop.replace(path)
        assert reopened.score_tiles(query)["status"] == "stale"
        with pytest.raises(ValueError, match="evaluation image changed"):
            reopened.image(query)
        with reopened.events.subscribe() as (reader, writer):
            reader.settimeout(5)
            assert reopened.scene_details(scene, run)["outputs"][0]["benchmark"]["status"] == "pending"
            while ("scores", 2) not in map(event_payload, reopened.events.take(reader, writer)):
                pass
        cropped, = reopened.scene_details(scene, run)["outputs"]
        assert cropped["benchmark"]["status"] == "invalid_output"
        assert cropped["benchmark"]["score"] == 0
        assert "image is 7 × 6 pixels" in cropped["benchmark"]["error"]
        assert "native dimensions of 8 × 6 pixels" in cropped["benchmark"]["error"]
        assert calls == ["summary", "tiles", "summary"]

        reopened.scores.close()
        receipt = path.parents[2] / "scores" / f"{scene}.json"
        saved = json.loads(receipt.read_text())
        saved.pop("invalid_detail_version")
        saved["result"]["error"] = "Expected finite float32 linear-sRGB RGB on the (6, 8, 3) grid"
        receipt.write_text(json.dumps(saved))
        reopened = open_review(root, store)
        with reopened.events.subscribe() as (reader, writer):
            reader.settimeout(5)
            assert reopened.scene_details(scene, run)["outputs"][0]["benchmark"]["status"] == "pending"
            while ("scores", 1) not in map(event_payload, reopened.events.take(reader, writer)):
                pass
        assert reopened.scene_details(scene, run)["outputs"][0]["benchmark"] == cropped["benchmark"]
        assert calls == ["summary", "tiles", "summary", "summary"]
    finally:
        gate.set()
        app.scores.close()
        if reopened is not None:
            reopened.scores.close()
