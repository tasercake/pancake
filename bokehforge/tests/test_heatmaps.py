from concurrent.futures import ThreadPoolExecutor
from contextlib import contextmanager
from functools import partial
from http.server import ThreadingHTTPServer
from io import BytesIO
import json
import os
from pathlib import Path
import subprocess
import sys
from threading import Event, Thread
from types import SimpleNamespace
from urllib.error import HTTPError
from urllib.request import Request, urlopen

import numpy as np
from PIL import Image
import pytest

from bokehforge import heatmap_cache, heatmaps
from bokehforge.data._config import read_config, DEFAULT_CONFIG


def event(stream):
    assert stream.readline() == b"event: heatmaps\n"
    payload = json.loads(stream.readline().removeprefix(b"data: "))
    assert stream.readline() == b"\n"
    return payload


def generate(url, datasets, **headers):
    request = Request(url + "api/generate", json.dumps({"profile": "baseline", "datasets": datasets}).encode(),
                      {"Content-Type": "application/json", **headers})
    return urlopen(request, timeout=5)


@contextmanager
def serve(app):
    handler = partial(heatmaps.GalleryHandler, app=app, host="127.0.0.1")
    with ThreadingHTTPServer(("127.0.0.1", 0), handler) as server:
        thread = Thread(target=server.serve_forever, daemon=True)
        thread.start()
        try:
            yield f"http://127.0.0.1:{server.server_port}/"
        finally:
            app.events.close()
            server.shutdown()
            thread.join()


def test_generate_stream_progress_and_reuse_cached_gallery(tmp_path, monkeypatch):
    entered, resume = Event(), Event()
    reference = np.linspace(0, 1, 12 * 16 * 3, dtype=np.float32).reshape((12, 16, 3))
    scene = SimpleNamespace(id="D29/flower", metadata={"source_scene_id": "flower"}, reference_ids=("reference",))

    def decode(frame_id):
        assert frame_id == "reference"
        entered.set()
        assert resume.wait(10)
        return SimpleNamespace(data=reference)

    def references(self, dataset, report):
        assert dataset == "D29"
        return SimpleNamespace(decode=decode), [scene]

    monkeypatch.setattr(heatmaps, "_inspection_available", lambda: False)
    monkeypatch.setattr(heatmap_cache.HeatmapCache, "references", references)
    cache = heatmap_cache.HeatmapCache(tmp_path, read_config(DEFAULT_CONFIG)["datasets"])
    app = heatmaps.Gallery(cache)
    gallery_path = "api/gallery?profile=baseline&dataset=D29"

    with serve(app) as url, urlopen(url + "api/events", timeout=5) as stream:
        assert event(stream)["datasets"][0]["cached"] == []
        with urlopen(url + gallery_path, timeout=5) as response:
            assert json.load(response)["scenes"] == []

        for datasets, headers, status in ((["unknown"], {}, 400), (["D29"], {"Origin": "http://foreign.invalid"}, 403)):
            with pytest.raises(HTTPError) as error:
                generate(url, datasets, **headers)
            assert error.value.code == status
            error.value.close()

        with generate(url, ["D29"]) as response:
            assert response.status == 202
        try:
            assert entered.wait(5)
            assert cache.gallery("D29", "baseline") is None
            with urlopen(url + "api/events", timeout=5) as reconnect:
                job = event(reconnect)["job"]
                assert (job["status"], job["completed"], job["total"]) == ("running", 0, 1)
            with pytest.raises(HTTPError, match="409") as error:
                generate(url, ["D29"])
            error.value.close()
        finally:
            resume.set()

        while (snapshot := event(stream))["job"]["status"] == "running":
            pass
        assert snapshot["job"]["status"] == "complete"
        assert snapshot["datasets"][0]["cached"] == ["baseline"]
        with urlopen(url + gallery_path, timeout=5) as response:
            gallery = json.load(response)
        assert set(gallery["models"]) == {"detail-light", "center"}
        photo = gallery["scenes"][0]
        images = [photo["photo"], *(url for variants in photo["maps"].values() for url in variants.values())]
        for path in images:
            with urlopen(url + path.removeprefix("/"), timeout=5) as response:
                assert response.headers.get_content_type() == "image/png"
                with Image.open(BytesIO(response.read())) as image:
                    assert image.size == (16, 12)
        with urlopen(Request(url + photo["photo"].removeprefix("/"), method="HEAD"), timeout=5) as response:
            assert int(response.headers["Content-Length"]) > 0
            assert response.read() == b""

    command = Path(sys.executable).with_name("bokehforge")
    environment = {**os.environ, "BOKEHFORGE_DATA_ROOT": str(tmp_path)}
    environment.pop("PYTHONPATH", None)
    with subprocess.Popen(
        [str(command), "heatmaps", "--port", "0"], cwd=tmp_path, env=environment,
        stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True,
    ) as process, ThreadPoolExecutor(max_workers=1) as output:
        try:
            url = output.submit(process.stdout.readline).result(timeout=5).removeprefix("BokehForge heatmaps at ").strip()
            with urlopen(url, timeout=5) as response:
                assert b"Generate heatmaps" in response.read()
            with urlopen(url + gallery_path, timeout=5) as response:
                assert json.load(response) == gallery
            with urlopen(url + "api/events", timeout=5) as reconnect:
                assert event(reconnect)["job"]["status"] == "idle"
        finally:
            process.terminate()
            process.wait(timeout=5)


def test_failed_dataset_generation_can_be_retried_without_rebuilding_cached_datasets(tmp_path, monkeypatch):
    sources = [{"id": dataset, "name": dataset} for dataset in ("D01", "D02")]
    cache = heatmap_cache.HeatmapCache(tmp_path, sources)
    called = []
    fail = True

    def references(dataset, report):
        called.append(dataset)
        scene = SimpleNamespace(id=f"{dataset}/flower", metadata={"source_scene_id": "flower"},
                                reference_ids=("reference",))

        def decode(_):
            if fail and dataset == "D02":
                raise ValueError("Reference decode failed")
            return SimpleNamespace(data=np.ones((2, 2, 3), dtype=np.float32))

        return SimpleNamespace(decode=decode), [scene]

    monkeypatch.setattr(cache, "references", references)
    ready = {}

    def report(*_):
        pass

    with pytest.raises(ValueError, match="Reference decode failed"):
        cache.generate(["D01", "D02"], "baseline", report, ready.__setitem__)
    assert list(ready) == ["D01"]
    assert cache.gallery("D02", "baseline") is None
    assert not list(cache.directory.rglob("*.pending*"))

    fail = False
    cache.generate(["D01", "D02"], "baseline", report, ready.__setitem__)
    assert list(ready) == ["D01", "D02"]
    assert called == ["D01", "D02", "D02"]
