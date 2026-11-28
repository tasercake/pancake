import json
from base64 import b64decode
from hashlib import sha256
import subprocess
import sys
from zlib import decompress

import numpy as np
import pytest
import tifffile
from PIL import Image

from bokehforge.data._canonical import INFINIGEN_DISTANCES


_PROBE = """\
import sys

class BlockPythonScoringDependencies:
    blocked = (
        "scipy", "torch", "torchvision", "triton", "transformers", "timm", "cv2", "onnxruntime",
        "bokehforge._cuda_damage", "bokehforge._mps_damage", "bokehforge.inspection_models",
    )

    def find_spec(self, name, path=None, target=None):
        if any(name == package or name.startswith(package + ".") for package in self.blocked):
            raise AssertionError(f"Native scoring tried to import {name}")

sys.meta_path.insert(0, BlockPythonScoringDependencies())

from bokehforge import _rust

def trace_native(name):
    operation = getattr(_rust, name)
    calls = []

    def traced(*args, **kwargs):
        calls.append(name)
        return operation(*args, **kwargs)

    setattr(_rust, name, traced)
    return calls
"""


def _run_probe(code, *args):
    process = subprocess.run([sys.executable, "-c", _PROBE + code, *map(str, args)],
                             capture_output=True, text=True, check=False, timeout=30)
    assert process.returncode == 0, process.stdout + process.stderr


def _reference_pixels():
    samples = np.random.default_rng(31).integers(32, 224, (24, 30, 3), dtype=np.uint8)
    return samples.repeat(5, axis=0).repeat(5, axis=1)


def test_public_measure_accepts_nonpacked_rgb():
    from bokehforge import measure

    reference = _reference_pixels().astype(np.float32) / 255
    candidate = reference.copy()
    candidate[25:83, 47:107, 0] += np.float32(.075)
    options = {"device": "cpu", "attention": False, "details": "summary"}
    expected = measure(candidate, reference, **options)
    unaligned = np.ndarray(candidate.shape, np.float32, buffer=bytearray(candidate.nbytes + 1), offset=1)
    unaligned[:] = candidate
    views = (
        candidate.copy(order="F"),
        candidate[::-1, :, ::-1].copy()[::-1, :, ::-1],
        candidate.byteswap().view(candidate.dtype.newbyteorder()),
        np.pad(candidate, ((0, 0), (2, 0), (0, 0)))[:, 2:],
        unaligned,
    )
    for view in views:
        assert measure(view, reference, **options) == expected


@pytest.mark.parametrize("dtype", [np.float16, np.float64, np.longdouble])
def test_public_measure_validates_inspection_before_narrowing(dtype):
    from bokehforge import measure

    reference = np.full((2, 3, 3), .3, dtype=np.float32)
    heatmap = np.linspace(0, 1, 6, dtype=dtype).reshape(reference.shape[:2])
    result = measure(reference, reference, inspection_heatmap=heatmap, device="cpu", details="attention")
    encoded = result.explanation["attention"]["data"]
    assert decompress(b64decode(encoded)) == heatmap.astype("<f4").tobytes()

    heatmap[-1, -1] = np.nextafter(dtype(1), dtype(2))
    with pytest.raises(ValueError, match="finite floating-point inspection map"):
        measure(reference, reference, inspection_heatmap=heatmap, device="cpu")


def test_public_measure_uses_native_runtime_without_python_scoring_dependencies(tmp_path):
    pixels = tmp_path / "reference.npy"
    np.save(pixels, _reference_pixels())
    _run_probe("""\
import numpy as np

calls = trace_native("measure_delivery")
from bokehforge import measure

reference = np.load(sys.argv[1]).astype(np.float32) / 255
candidate = reference.copy()
candidate[25:83, 47:107, 0] += np.float32(.075)
heatmap = np.linspace(.15, .65, reference.shape[0] * reference.shape[1], dtype=np.float32)
heatmap = heatmap.reshape(reference.shape[:2])

unweighted = measure(candidate, reference, details="tiles", attention=False, device="cpu")
weighted = measure(candidate, reference, details="full", inspection_heatmap=heatmap, device="cpu")
for result in (unweighted, weighted):
    assert 0 < result.score < 100
    assert result.alignment["status"] == "matched"
    assert result.features["native.color"] > 0
    placement, = result.explanation["placements"]
    assert len(placement["comparison"]["tile_grids"]) == 8
assert weighted.explanation["attention"]["encoding"] == "base64+zlib+float32-le"
assert weighted.score != unweighted.score
assert len(calls) == 2
assert "PIL" not in sys.modules
""", pixels)


def test_cli_replay_uses_native_runtime_without_python_scoring_dependencies(tmp_path):
    root = tmp_path / "data"
    raw = root / "raw" / "synthetic"
    folder = raw / "a1_0"
    folder.mkdir(parents=True)
    pixels = _reference_pixels()
    reference = folder / "image_all_in_focus.png"
    Image.fromarray(pixels).save(reference)
    for distance in INFINIGEN_DISTANCES:
        frame = folder / f"image_ap_1.40_fd_{distance:.2f}.png"
        Image.fromarray(np.full((2, 2, 3), 32, dtype=np.uint8)).save(frame)
    files = [{"path": path.relative_to(raw).as_posix(), "sha256": sha256(path.read_bytes()).hexdigest()}
             for path in sorted(folder.glob("*.png"))]
    (raw / "download-manifest.json").write_text(json.dumps({
        "id": "synthetic", "status": "available", "layout": "infinigen_defocus", "transfer": "linear",
        "reference_type": "synthetic_source_image_reference", "files": files,
    }), encoding="utf-8")
    config = root / "sources.json"
    config.write_text(json.dumps({"datasets": [{"id": "synthetic"}]}), encoding="utf-8")

    from bokehforge.data.lazy import SceneCatalog

    scene, = SceneCatalog(root, config=config).scene_ids
    candidates = tmp_path / "candidates"
    delivery = candidates / f"{scene}.tif"
    delivery.parent.mkdir(parents=True)
    candidate = pixels.astype(np.float32) / 255
    candidate[25:83, 47:107, 0] += np.float32(.075)
    tifffile.imwrite(delivery, candidate, photometric="rgb")

    output = tmp_path / "benchmark"
    _run_probe("""\
import runpy

calls = trace_native("benchmark")
sys.argv = ["bokehforge", "bench", *sys.argv[1:]]
try:
    runpy.run_module("bokehforge", run_name="__main__")
except SystemExit as result:
    assert result.code == 0
else:
    raise AssertionError("The CLI did not return an exit status")
assert len(calls) == 1
""", "--root", root, "--config", config, "--candidate-dir", candidates, "--output", output,
        "--device", "cpu", "--no-attention", "--explain-score")
    report = json.loads((output / "report.json").read_text(encoding="utf-8"))
    row, = report["scenes"]
    assert row["status"] == "ok" and row["reference_status"] == "eligible"
    assert row["frames_read"] == 0
    assert row["score_features"]["native.color"] > 0
    assert 0 < report["score"] == row["scene_score"] < 100
    explanation = json.loads((output / row["score_explanation_path"]).read_text(encoding="utf-8"))
    assert explanation["score"] == report["score"]
