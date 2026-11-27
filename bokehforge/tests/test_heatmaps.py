from concurrent.futures import ThreadPoolExecutor
import json
import os
from pathlib import Path
import subprocess
import sys
from urllib.error import HTTPError
from urllib.request import Request, urlopen

import pytest


def test_heatmaps_serves_packaged_viewer_and_rendered_gallery(tmp_path):
    gallery = tmp_path / "gallery"
    images = gallery / "interest-v1"
    images.mkdir(parents=True)
    manifest = {"scenes": [{"id": "flower", "photo": "interest-v1/flower-photo.png"}], "models": {}}
    (gallery / "interest-data.js").write_text(f"const gallery = {json.dumps(manifest)};\n")
    image = b"\x89PNG\r\n\x1a\n"
    (images / "flower-photo.png").write_bytes(image)
    (gallery / "index.html").write_text("Old viewer")
    (gallery / "private.txt").write_text("Private file")
    hidden_paths = ["private.txt", "interest-data.js", "interest-v1/"]
    if sys.platform != "win32":
        (images / "outside.png").symlink_to(gallery / "private.txt")
        hidden_paths.append("interest-v1/outside.png")

    command = Path(sys.executable).with_name("bokehforge")
    environment = os.environ.copy()
    environment.pop("PYTHONPATH", None)
    with subprocess.Popen(
        [str(command), "heatmaps", str(gallery), "--port", "0"],
        cwd=tmp_path, env=environment, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True,
    ) as process, ThreadPoolExecutor(max_workers=1) as output:
        try:
            line = output.submit(process.stdout.readline).result(timeout=5)
            address = line.removeprefix("BokehForge heatmaps at ").strip()
            assert address.startswith("http://127.0.0.1:")

            with urlopen(address, timeout=5) as response:
                assert response.headers.get_content_type() == "text/html"
                assert b"BokehForge image interest" in response.read()
            with urlopen(address + "gallery.json", timeout=5) as response:
                assert response.headers.get_content_type() == "application/json"
                assert json.load(response) == manifest
            image_url = address + manifest["scenes"][0]["photo"] + "?v=1"
            with urlopen(image_url, timeout=5) as response:
                assert response.headers.get_content_type() == "image/png"
                assert response.read() == image
            with urlopen(Request(image_url, method="HEAD"), timeout=5) as response:
                assert int(response.headers["Content-Length"]) == len(image)
                assert response.read() == b""
            for path in hidden_paths:
                with pytest.raises(HTTPError, match="404") as error:
                    urlopen(address + path, timeout=5)
                error.value.close()
        finally:
            process.terminate()
            process.wait(timeout=5)


def test_heatmaps_rejects_script_as_gallery_data(tmp_path):
    (tmp_path / "interest-data.js").write_text('const gallery = {}; alert("hello");')
    result = subprocess.run(
        [sys.executable, "-m", "bokehforge", "heatmaps", str(tmp_path)], capture_output=True, text=True,
    )
    assert result.returncode == 2
    assert not result.stdout
    assert "ERROR:" in result.stderr
