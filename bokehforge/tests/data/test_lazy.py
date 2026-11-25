from hashlib import sha256
from io import BytesIO
import json
import pickle
from zipfile import ZipFile

import numpy as np
from PIL import Image
import pytest
import tifffile

from bokehforge import FrameWindowDataset, FusionPairDataset, SceneCatalog


def write_lock(tmp_path, dataset, layout, paths, **metadata):
    root = tmp_path / "data"
    raw = root / "raw" / dataset
    source = {
        "id": dataset, "status": "available", "layout": layout, "reference_type": "all_in_focus",
        "files": [{"path": path.relative_to(raw).as_posix(), "sha256": sha256(path.read_bytes()).hexdigest()} for path in paths],
        **metadata,
    }
    (raw / "download-manifest.json").write_text(json.dumps(source))
    config = tmp_path / "sources.json"
    config.write_text(json.dumps({"datasets": [{"id": dataset}]}))
    lock = tmp_path / "sources.lock.json"
    lock.write_text(json.dumps({"datasets": [source]}))
    return root, lock


def test_catalog_and_views_load_infinigen_on_demand(tmp_path):
    native = tmp_path / "data/raw/D29/InfinigenDefocus.zip"
    native.parent.mkdir(parents=True)
    distances = (0.8, 1.2, 1.7, 2.3, 3.0, 3.8, 4.7, 6.0, 8.0)
    with ZipFile(native, "w") as archive:
        for i, distance in reversed(list(enumerate(distances))):
            with BytesIO() as payload:
                Image.new("RGB", (7, 5), (10 * i, 24, 87)).save(payload, format="PNG")
                archive.writestr(f"data/abc123_1/image_ap_1.40_fd_{distance:.2f}.png", payload.getvalue())
        with BytesIO() as payload:
            Image.new("RGB", (7, 5), (120, 84, 9)).save(payload, format="PNG")
            archive.writestr("data/abc123_1/image_all_in_focus.png", payload.getvalue())
    root, lock = write_lock(tmp_path, "D29", "infinigen_defocus", [native])
    legacy = {"id": "D04", "status": "available", "layout": "archive_stacks",
              "files": [{"path": "missing.zip"}]}
    manifest = root / "raw/D04/download-manifest.json"
    manifest.parent.mkdir()
    manifest.write_text(json.dumps(legacy))
    snapshot = json.loads(lock.read_text())
    snapshot["datasets"].append(legacy)
    lock.write_text(json.dumps(snapshot))

    catalog = SceneCatalog(root)
    assert SceneCatalog(root, lock=lock).scene_ids == catalog.scene_ids
    with pytest.raises(ValueError, match="Unknown or undownloaded datasets"):
        SceneCatalog(root, datasets="D04")
    scene, = map(catalog.scene, catalog.scene_ids)
    assert len(scene.frame_ids) == 9 and len(scene.reference_ids) == 1
    assert [catalog.frame(frame_id).metadata["focus_distance_m"] for frame_id in scene.frame_ids] == list(distances)
    assert len(catalog.quarantine["D29"]) == 4
    windows = FrameWindowDataset(catalog)
    assert len(windows) == 2 and windows[-1]["frame_ids"] == scene.frame_ids[1:]
    with pytest.raises(IndexError):
        windows[2]
    with pytest.raises(ValueError, match="positive"):
        FrameWindowDataset(catalog, stride=0)
    pairs = pickle.loads(pickle.dumps(FusionPairDataset(catalog, output="tiff")))
    assert len(pairs) == 8 and pairs[-1]["frame_ids"] == scene.frame_ids[-2:]
    assert not (root / "standardized").exists() and not (root / ".bokehforge").exists()

    window = windows[0]
    assert window["frame_ids"] == scene.frame_ids[:8]
    assert window["frames"][0].dtype == np.float32
    assert [frame[0, 0, 0] for frame in window["frames"]] == sorted(frame[0, 0, 0] for frame in window["frames"])
    pair = pairs[0]
    reference_id, = scene.reference_ids
    assert pair["reference_id"] == reference_id
    reference = catalog.decode(reference_id)
    assert reference.provenance["role"] == "references"
    assert reference.provenance["recipe_sha256"] == scene.metadata["recipe_sha256"]
    assert np.array_equal(reference.data, tifffile.imread(BytesIO(pair["reference"])))
    for expected, encoded in zip(window["frames"][:2], pair["frames"], strict=True):
        with tifffile.TiffFile(BytesIO(encoded)) as image:
            page = image.pages[0]
            assert page.compression.name in {"DEFLATE", "ADOBE_DEFLATE"}
            assert page.predictor.name == "FLOATINGPOINT"
            assert np.array_equal(expected, page.asarray())


def test_archived_tiff_indexes_pages_and_preserves_alpha(tmp_path):
    native = tmp_path / "data/raw/fixture/source.zip"
    native.parent.mkdir(parents=True)
    with BytesIO() as payload:
        with tifffile.TiffWriter(payload) as writer:
            for value in (50, 200):
                array = np.full((5, 7, 4), value, np.uint8)
                writer.write(array, photometric="rgb", extrasamples=["unassalpha"])
        with ZipFile(native, "w") as archive:
            archive.writestr("scene/stack.tif", payload.getvalue())
    root, _ = write_lock(tmp_path, "fixture", "archive_stacks", [native])
    catalog = SceneCatalog(root, config=tmp_path / "sources.json")
    scene = catalog.scene(catalog.scene_ids[0])
    sample = FrameWindowDataset(catalog, window_size=2)[0]
    assert len(scene.frame_ids) == 2 and len(FusionPairDataset(catalog)) == 0
    second = catalog.frame(scene.frame_ids[1])
    frame = catalog.decoder.decode(second)
    assert frame.provenance["page"] == 1
    assert frame.provenance["decode"]["page"] == 1
    assert frame.provenance["source_path"] == "source.zip::scene/stack.tif"
    assert np.allclose(sample["alpha"][1], 200 / 255)
    assert np.array_equal(frame.data, sample["frames"][1])
    encoded = catalog.decoder.decode(second, output="tiff")
    assert np.array_equal(tifffile.imread(BytesIO(encoded.alpha)), frame.alpha)
    with native.open("ab") as handle:
        handle.write(b"changed")
    with pytest.raises(ValueError, match="changed after catalog"):
        catalog.decoder.decode(second)
    with pytest.raises(ValueError, match="mismatched source SHA-256"):
        SceneCatalog(root, config=tmp_path / "sources.json")


def test_catalog_rejects_unsafe_archive_members(tmp_path):
    native = tmp_path / "data/raw/fixture/source.zip"
    native.parent.mkdir(parents=True)
    with ZipFile(native, "w") as archive:
        archive.writestr("../outside.png", b"pixels")
    root, _ = write_lock(tmp_path, "fixture", "archive_stacks", [native])
    with pytest.raises(ValueError, match="Unsafe archive member"):
        SceneCatalog(root, config=tmp_path / "sources.json")


def test_conversion_policy_preserves_source_identity_and_natural_order(tmp_path):
    raw = tmp_path / "data/raw/fixture/scene"
    raw.mkdir(parents=True)
    originals = [raw / "frame10.png", raw / "frame2.png"]
    for path, value in zip(originals, (160, 128), strict=True):
        Image.new("RGB", (9, 8), (value,) * 3).save(path)
    root, lock = write_lock(tmp_path, "fixture", "directory_stacks", originals, transfer="srgb")
    source_lock = lock.read_bytes()
    srgb = SceneCatalog(root, lock=lock, config=tmp_path / "sources.json")
    scene = srgb.scene(srgb.scene_ids[0])
    frame_id = scene.frame_ids[0]
    assert srgb.frame(frame_id).source_path == "scene/frame2.png"

    config = tmp_path / "config.json"
    config.write_text(json.dumps({"datasets": [{"id": "fixture", "transfer": "linear"}]}))
    linear = SceneCatalog(root, lock=lock, config=config)
    assert np.allclose(linear.decode(frame_id).data, 128 / 255, atol=1e-7)
    assert np.allclose(srgb.decode(frame_id).data, ((128 / 255 + 0.055) / 1.055) ** 2.4, atol=1e-7)
    assert linear.frame(frame_id).source_sha256 == srgb.frame(frame_id).source_sha256
    assert linear.decode(frame_id).provenance["recipe_sha256"] != srgb.decode(frame_id).provenance["recipe_sha256"]
    assert lock.read_bytes() == source_lock
    config.write_text(json.dumps({"datasets": [{"id": "fixture", "provider": "disabled"}]}))
    assert SceneCatalog(root, lock=lock, config=config).scene_ids == ()


def test_float_headroom_and_negative_values_survive_tiff_encoding(tmp_path):
    raw = tmp_path / "data/raw/fixture/scene"
    raw.mkdir(parents=True)
    originals = [raw / "frame0.tif", raw / "frame1.tif"]
    pixels = np.array([[[-0.1, 0.3, 1.7]]], dtype=np.float32)
    for path in originals:
        tifffile.imwrite(path, pixels, photometric="rgb")
    root, _ = write_lock(tmp_path, "fixture", "directory_stacks", originals, transfer="linear")
    catalog = SceneCatalog(root, config=tmp_path / "sources.json")
    frames = FrameWindowDataset(catalog, window_size=2, output="tiff")[0]["frames"]
    assert all(np.array_equal(tifffile.imread(BytesIO(frame)), pixels) for frame in frames)


def test_edmf_catalog_quarantines_incomplete_pairs(tmp_path):
    raw = tmp_path / "data/raw/fixture"
    originals = [raw / name for name in ("Source_A/bg-A-1.png", "Source_B/bg-B-1.png", "Source_B/bg-B-2.png")]
    for path in originals:
        path.parent.mkdir(parents=True, exist_ok=True)
        Image.new("RGB", (8, 8)).save(path)
    root, _ = write_lock(tmp_path, "fixture", "edmf", originals)
    catalog = SceneCatalog(root, config=tmp_path / "sources.json")
    assert len(catalog.scene_ids) == 1
    assert len(catalog.quarantine["fixture"]) == 1
    assert catalog.quarantine["fixture"][0]["scene"] == "pair-0002"
