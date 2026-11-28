import numpy as np
from PIL import Image
import pytest

from bokehforge.inspection import _preview_encoder, reference_preview


@pytest.mark.parametrize("shape, dtype, packed", [
    ((37, 41, 3), "f4", True),
    ((1001, 1025, 3), "f4", True),
    ((1001, 1025, 3), ">f4", False),
])
def test_reference_preview_matches_interleaved_encoding_and_resize(shape, dtype, packed):
    cutoffs = (np.arange(255) + .5) / 255
    boundaries = np.where(cutoffs <= .04045, cutoffs / 12.92, ((cutoffs + .055) / 1.055) ** 2.4).astype(np.float32)
    samples = np.concatenate((boundaries, np.nextafter(boundaries, np.float32(-np.inf)),
                              np.nextafter(boundaries, np.float32(np.inf)),
                              [-np.finfo(np.float32).max, -1, 0, .0031308, 1, np.finfo(np.float32).max]))
    reference = np.resize(samples, shape).astype(dtype)
    if not packed:
        reference = reference[::-1, :, ::-1]
    height, width = shape[:2]
    pixels = np.empty(shape, dtype=np.uint8)
    for start in range(0, height, 128):
        rows = slice(start, start + 128)
        linear = np.clip(reference[rows], 0, 1)
        srgb = np.where(linear <= .0031308, 12.92 * linear, 1.055 * linear ** (1 / 2.4) - .055)
        pixels[rows] = np.rint(srgb * 255)
    scale = min(1, 640 / max(height, width))
    size = (round(width * scale), round(height * scale))
    expected = Image.fromarray(pixels).resize(size, Image.Resampling.LANCZOS)

    preview = reference_preview(reference)

    assert preview.mode == expected.mode == "RGB"
    assert preview.size == size
    assert preview.tobytes() == expected.tobytes()


def test_native_preview_encoder_matches_float32_rounding_at_every_transition():
    from bokehforge import _rust

    thresholds = (np.arange(255) + .5) / 255
    linear = np.where(thresholds <= .04045, thresholds / 12.92, ((thresholds + .055) / 1.055) ** 2.4)
    bits = linear.astype(np.float32).view(np.int32)[:, None] + np.arange(-128, 129, dtype=np.int32)
    samples = bits.view(np.float32).reshape(-1, 1, 3)
    srgb = np.where(samples <= .0031308, 12.92 * samples, 1.055 * samples ** (1 / 2.4) - .055)
    encoder = _preview_encoder()
    assert isinstance(encoder, _rust.PreviewEncoder)
    np.testing.assert_array_equal(encoder.encode(samples), np.rint(srgb * 255).astype(np.uint8))


def test_inspection_export_uses_current_version_and_repairs_corruption(tmp_path, monkeypatch):
    from bokehforge import _inspection_export as exporter

    tmp_path = tmp_path.resolve()
    calls = []

    def export(source, destination):
        calls.append(source)
        for name in exporter._GRAPHS:
            (destination / name).write_bytes(b"graph")

    old = tmp_path / "onnx-v1"
    old.mkdir()
    export(tmp_path, old)
    (old / exporter._MANIFEST).write_text("".join(exporter._record(path) for path in sorted(old.iterdir())))
    calls.clear()
    monkeypatch.setattr(exporter, "_export", export)

    current = exporter.prepare_models(tmp_path)
    assert current != old
    assert exporter.prepare_models(tmp_path) == current
    assert calls == [tmp_path]

    (current / "sam_encoder.onnx").write_bytes(b"stale")
    assert exporter.prepare_models(tmp_path) == current
    assert calls == [tmp_path, tmp_path]
    assert (current / "sam_encoder.onnx").read_bytes() == b"graph"
