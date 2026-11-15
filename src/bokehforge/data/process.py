#!/usr/bin/env python3
"""Convert verified source stacks to lossless float32 linear-sRGB TIFF sequences."""

import argparse
import concurrent.futures
import csv
import fnmatch
import hashlib
import importlib.metadata
import json
from pathlib import Path
import platform
import re
import sys
import zipfile

import imagecodecs
import numpy as np
from PIL import Image, ImageCms, ImageOps
import tifffile

from .download import CONFIG_DIRECTORY, HERE, check_space, digest, natural_key, safe_path, save_json
from .layout import METADATA, data_run

IMAGES = {".png", ".jpg", ".jpeg", ".bmp", ".tif", ".tiff", ".pgm", ".ppm", ".webp", ".stk"}
CAMERA_RAW = {".rw2", ".dng", ".nef", ".cr3", ".cr2", ".arw", ".raf", ".orf"}
ARCHIVES = {".zip", ".rar", ".tar", ".tgz", ".gz", ".7z"}
CONVERTER_SHA256 = digest(Path(__file__))
HELPERS_SHA256 = digest(HERE / "download.py")
SUPPORTED_LAYOUTS = {"explicit_frames", "sood_raw", "lytro", "hrmff", "edmf", "mfifb", "cervix93",
                     "near_far", "realmff", "mfi_whu", "fod500", "lfsd", "directory_stacks", "archive_stacks", "exposure_stacks"}


def runtime_recipe(source):
    import rawpy
    packages = {n: importlib.metadata.version(n) for n in ["numpy", "pillow", "tifffile", "imagecodecs", "rawpy", "libarchive-c"]}
    definition = {"version": 1, "converter_sha256": CONVERTER_SHA256, "packages": packages,
                  "shared_helpers_sha256": HELPERS_SHA256,
                  "runtime": {"python": platform.python_version(), "system": platform.system(), "machine": platform.machine(),
                              "libraw": list(rawpy.libraw_version), "lcms": imagecodecs.cms_version()},
                  "layout": source.get("layout"), "transfer": source.get("transfer", "srgb_assumed"),
                  "source_contract": {k: source.get(k) for k in ["reference_type", "reference_semantics", "original_split", "evaluation_domain", "primary_url", "rights_evidence"]},
                  "conversion_inputs": source.get("conversion_inputs"),
                  "output": "float32_HWC_RGB_TIFF_deflate6_predictor3", "resize": False, "alignment": False,
                  "raw": {"gamma": [1, 1], "output_bps": 16, "use_camera_wb": True, "no_auto_bright": True,
                          "output_color": "sRGB", "demosaic": "AHD", "highlight_mode": "Clip", "adjust_maximum_thr": 0.0}}
    signature = hashlib.sha256(json.dumps(definition, sort_keys=True).encode()).hexdigest()
    return definition, signature


def extract_archive(path, destination, reserve):
    """Extract original bytes, never following archive links or unsafe member paths."""
    if path.suffix.lower() == ".zip":
        with zipfile.ZipFile(path) as archive:
            entries = archive.infolist()
            for entry in entries:
                safe_path(destination, entry.filename)
                if (entry.external_attr >> 16) & 0o170000 == 0o120000:
                    raise ValueError(f"Unsafe archive symlink: {entry.filename}")
            for entry in entries:
                if entry.is_dir():
                    continue
                target = safe_path(destination, entry.filename)
                # CRC and size are checked on every reuse; no timestamp-only trust.
                if target.is_file() and target.stat().st_size == entry.file_size:
                    import zlib
                    crc = 0
                    with target.open("rb") as handle:
                        for chunk in iter(lambda: handle.read(4 * 1024 * 1024), b""):
                            crc = zlib.crc32(chunk, crc)
                    if crc == entry.CRC:
                        continue
                check_space(destination, entry.file_size, reserve)
                target.parent.mkdir(parents=True, exist_ok=True)
                temporary = target.with_name(target.name + ".part")
                with archive.open(entry) as incoming, temporary.open("wb") as outgoing:
                    for chunk in iter(lambda: incoming.read(4 * 1024 * 1024), b""):
                        outgoing.write(chunk)
                temporary.replace(target)
    else:
        import libarchive
        # The source archive is content-verified before this function is called.
        completion = destination / ".extracted.json"
        if completion.exists():
            inventory = json.loads(completion.read_text())
            if all((destination / x["path"]).is_file() and digest(destination / x["path"]) == x["sha256"] for x in inventory):
                return
        inventory = []
        with libarchive.file_reader(str(path)) as archive:
            for entry in archive:
                target = safe_path(destination, entry.pathname)
                if entry.issym or entry.islnk:
                    raise ValueError(f"Unsafe archive link: {entry.pathname}")
                if entry.isdir:
                    continue
                check_space(destination, entry.size, reserve)
                target.parent.mkdir(parents=True, exist_ok=True)
                temporary = target.with_name(target.name + ".part")
                with temporary.open("wb") as handle:
                    for block in entry.get_blocks():
                        handle.write(block)
                temporary.replace(target)
                inventory.append({"path": str(target.relative_to(destination)), "sha256": digest(target)})
        save_json(completion, inventory)


def inventory_source(root, source, reserve):
    files = []
    raw_root = root / "raw" / source["id"]
    for item in source["files"]:
        if source.get("conversion_inputs") and not any(fnmatch.fnmatch(item["path"], pattern) for pattern in source["conversion_inputs"]):
            continue
        original = safe_path(raw_root, item["path"])
        if not original.is_file() or not item.get("sha256") or digest(original) != item["sha256"]:
            raise RuntimeError(f"Source checksum missing or mismatch: {source['id']}/{item['path']}; run bokehforge data download")
        if original.suffix.lower() in ARCHIVES:
            extracted = root / METADATA / "work" / source["id"] / (original.stem + "-" + item["sha256"][:12])
            extract_archive(original, extracted, reserve)
            for p in sorted(extracted.rglob("*"), key=lambda p: natural_key(str(p))):
                if p.is_file() and not p.name.startswith("."):
                    files.append({"path": p, "source_path": item["path"] + "::" + p.relative_to(extracted).as_posix(),
                                  "archive": item["path"], "member": p.relative_to(extracted).as_posix()})
        else:
            files.append({"path": original, "source_path": item["path"], "spec": item})
    save_json(root / METADATA / "work" / source["id"] / "inventory.json", {"files": [{k: str(v) for k, v in x.items() if k not in {"path", "spec"}} for x in files]})
    return files


def plan_scenes(source, inventory):
    layout = source.get("layout", "directory_stacks")
    if layout not in SUPPORTED_LAYOUTS:
        raise RuntimeError(f"No conversion adapter for layout {layout}; original assets remain unchanged")
    grouped, quarantine = {}, []
    for item in inventory:
        p = Path(item.get("member", item["source_path"]))
        if p.suffix.lower() not in IMAGES | CAMERA_RAW:
            continue
        if "__MACOSX" in p.parts or "scalebar" in item["source_path"].lower():
            continue
        role = "frame"
        scene = None
        stem = p.stem
        if layout == "explicit_frames":
            scene = item.get("spec", {}).get("scene_id")
        elif layout == "sood_raw":
            if p.suffix.lower() not in CAMERA_RAW:
                continue
            scene = "macro-sweep"
        elif layout in {"lytro", "hrmff"}:
            match = re.match(r"(.+)[-_]([ABC])$", stem, re.I)
            if match:
                prefix = "triple-" if "Triple Series" in str(p) else ""
                scene = prefix + match[1]
        elif layout == "edmf":
            match = re.fullmatch(r"bg-[AB]-(\d+)", stem, re.I)
            scene = f"pair-{int(match[1]):04d}" if match else None
        elif layout == "mfifb":
            scene = p.parent.name
        elif layout == "cervix93":
            match = re.search(r"(frame\d+)_stack", str(p))
            if match:
                scene = match[1]
            elif "EDF" in p.parts:
                scene, role = stem, "reference"
            else:
                continue
        elif layout in {"near_far", "realmff", "mfi_whu"}:
            parts = list(p.parts)
            labels = {"near", "far", "a", "b", "source_a", "source_b", "source_1", "source_2", "sourcea", "sourceb", "imagea", "imageb", "inputa", "inputb", "gt", "groundtruth", "ground_truth", "reference", "full_clear", "fusion", "focus1", "focus2"}
            indices = [i for i, x in enumerate(parts[:-1]) if x.lower() in labels]
            if indices:
                idx = indices[-1]
                role = "reference" if parts[idx].lower() in {"gt", "groundtruth", "ground_truth", "reference", "full_clear", "fusion"} else "frame"
                normalized = re.sub(r"_[ABF]$", "", stem, flags=re.I) if layout == "realmff" else stem
                scene = "/".join(parts[:idx] + parts[idx + 1:-1] + [normalized])
            else:
                match = re.match(r"(.+?)[-_]([AB12])$", stem, re.I)
                if match:
                    scene = str(p.parent / match[1])
        elif layout == "fod500":
            match = re.fullmatch(r"(\d+)_(\d+)All", stem, re.I)
            if match:
                scene = "sample-" + match[1]
        elif layout == "lfsd":
            if "focus_stack" not in item["source_path"].lower():
                continue
            match = re.fullmatch(r"(.+?)__refocus_(\d+)", stem, re.I)
            scene = "scene-" + match[1] if match else str(p.parent)
        elif layout == "exposure_stacks":
            if "source image sequences" not in p.parts:
                continue
            scene = str(p.parent)
        else:
            scene = str(p.parent)
            if item.get("archive"):
                scene = Path(item["archive"]).stem + "/" + scene
        if scene is None:
            quarantine.append({"source_path": item["source_path"], "reason": "unrecognized source layout"})
            continue
        record = grouped.setdefault(scene, {"source_scene_id": scene, "frames": [], "references": []})
        record["references" if role == "reference" else "frames"].append(item)
    result = []
    for name, record in sorted(grouped.items(), key=lambda x: natural_key(x[0])):
        record["frames"].sort(key=lambda x: natural_key(x["source_path"]))
        record["references"].sort(key=lambda x: natural_key(x["source_path"]))
        count = len(record["frames"])
        required = 3 if layout == "lytro" and name.startswith("triple-") else 2
        if count < required or (layout in {"edmf", "hrmff", "mfifb"} and count != 2):
            # A multi-page optical TIFF can contain an entire stack.
            if count != 1 or Path(record["frames"][0]["path"]).suffix.lower() not in {".tif", ".tiff", ".stk"}:
                quarantine.append({"scene": name, "reason": f"incomplete stack ({count} source files)", "source_paths": [x["source_path"] for x in record["frames"]]})
                continue
        result.append(record)
    return result, quarantine


def decode(path, transfer):
    """Yield (RGB float32, optional alpha, metadata) for every optical image page."""
    if path.suffix.lower() in CAMERA_RAW:
        import rawpy
        with rawpy.imread(str(path)) as raw:
            array = raw.postprocess(gamma=(1, 1), output_bps=16, use_camera_wb=True, no_auto_bright=True,
                                    output_color=rawpy.ColorSpace.sRGB, demosaic_algorithm=rawpy.DemosaicAlgorithm.AHD,
                                    highlight_mode=rawpy.HighlightMode.Clip, user_flip=0, adjust_maximum_thr=0.0)
            metadata = {"decoder": "LibRaw/rawpy", "native_bit_depth": "camera-dependent", "transfer": "linear",
                        "black_level_per_channel": list(raw.black_level_per_channel), "camera_white_level": int(raw.white_level),
                        "camera_whitebalance": list(raw.camera_whitebalance), "original_orientation_flip": int(raw.sizes.flip),
                        "raw_development": "linear sRGB, camera WB, no auto brightness, AHD, output16; clipped sensor/highlight information not restored"}
        yield array.astype(np.float32) / 65535.0, None, metadata
        return
    if path.suffix.lower() in {".tif", ".tiff", ".stk"}:
        with tifffile.TiffFile(path) as image:
            for index, page in enumerate(image.pages):
                array = page.asarray()
                if page.planarconfig == 2 and array.ndim == 3:
                    array = np.moveaxis(array, 0, -1)
                orientation = page.tags.get("Orientation")
                if orientation and orientation.value != 1:
                    raise RuntimeError("Nontrivial TIFF orientation requires an explicit source adapter")
                profile = page.tags.get("InterColorProfile")
                yield normalize(array, transfer, profile.value if profile else None, {"decoder": "tifffile", "page": index})
        return
    with Image.open(path) as image:
        profile = image.info.get("icc_profile")
        orientation = image.getexif().get(274, 1)
        image = ImageOps.exif_transpose(image)
        if image.mode == "P":
            image = image.convert("RGBA" if "transparency" in image.info else "RGB")
        if image.mode not in {"RGB", "RGBA", "L", "LA", "I;16", "I;16B", "I", "F", "CMYK"}:
            raise RuntimeError(f"Unsupported image mode {image.mode}")
        if image.mode == "CMYK" and not profile:
            raise RuntimeError("CMYK input requires an ICC profile")
        array = np.asarray(image)
        yield normalize(array, transfer, profile, {"decoder": "Pillow", "original_orientation": orientation, "source_mode": image.mode})


def normalize(array, transfer, profile, metadata):
    native_dtype = str(array.dtype)
    alpha = None
    if array.ndim == 2:
        array = np.repeat(array[..., None], 3, axis=2)
    if array.ndim != 3 or array.shape[2] not in {2, 3, 4}:
        raise RuntimeError(f"Unsupported optical frame shape {array.shape}")
    cmyk = metadata.get("source_mode") == "CMYK"
    if array.shape[2] in {2, 4} and not cmyk:
        alpha = array[..., -1].astype(np.float32)
        if np.issubdtype(array.dtype, np.unsignedinteger):
            alpha /= np.iinfo(array.dtype).max
        array = array[..., :-1]
        if array.shape[2] == 1:
            array = np.repeat(array, 3, axis=2)
    if profile:
        output_profile = ImageCms.ImageCmsProfile(ImageCms.createProfile("sRGB")).tobytes()
        array = imagecodecs.cms_transform(np.ascontiguousarray(array), profile=profile, outprofile=output_profile,
                                         colorspace="cmyk" if cmyk else "rgb", outcolorspace="rgb", outdtype=np.float32, intent=1)
        metadata["source_icc_sha256"] = hashlib.sha256(profile).hexdigest()
        metadata["icc_transform"] = "LittleCMS float32, relative colorimetric, sRGB output"
        transfer = "srgb"
    else:
        if np.issubdtype(array.dtype, np.unsignedinteger):
            array = array.astype(np.float32) / np.iinfo(array.dtype).max
        elif np.issubdtype(array.dtype, np.floating):
            array = array.astype(np.float32)
        else:
            raise RuntimeError(f"Ambiguous integer signal range: {array.dtype}")
    if transfer in {"srgb", "srgb_assumed"}:
        rgb = np.empty(array.shape, np.float32)
        mask = array <= 0.04045
        rgb[mask] = array[mask] / 12.92
        rgb[~mask] = ((array[~mask] + 0.055) / 1.055) ** 2.4
    elif transfer == "linear":
        rgb = array.astype(np.float32)
    else:
        raise RuntimeError(f"Unsupported transfer contract: {transfer}; specify srgb, srgb_assumed or linear")
    if not np.isfinite(rgb).all():
        raise RuntimeError("Input contains NaN/Inf")
    return rgb, alpha, {**metadata, "native_dtype": native_dtype, "input_transfer_contract": transfer,
                        "color_space": "linear-sRGB", "range_policy": "dtype normalization only; no per-image normalization or clipping"}


def write_frame(path, array, identity, state_dir, recipe_hash, reserve, reuse_pixels=False):
    state = state_dir / (path.name + ".json")
    if path.is_file() and state.is_file():
        previous = json.loads(state.read_text())
        if previous.get("identity") == identity and previous.get("recipe_sha256") == recipe_hash and digest(path) == previous["sha256"]:
            return previous
    unchanged = False
    if reuse_pixels and path.is_file() and state.is_file():
        try:
            old = tifffile.imread(path)
            unchanged = (old.dtype == np.float32 and old.shape == array.shape
                         and digest(path) == json.loads(state.read_text())["sha256"]
                         and np.array_equal(old.view(np.uint32), array.astype(np.float32).view(np.uint32)))
        except (ValueError, OSError, tifffile.TiffFileError):
            pass
    if not unchanged:
        check_space(path.parent, array.nbytes + 1024 * 1024, reserve)
        path.parent.mkdir(parents=True, exist_ok=True)
        temporary = path.with_name(path.name + ".part")
        tifffile.imwrite(temporary, np.ascontiguousarray(array, dtype=np.float32), photometric="rgb" if array.ndim == 3 else "minisblack",
                         compression="deflate", compressionargs={"level": 6}, predictor=3,
                         metadata=None, datetime=None, software="focus-stacking-standardize/1", maxworkers=1)
        temporary.replace(path)
    record = {"sha256": digest(path), "identity": identity, "recipe_sha256": recipe_hash, "shape": list(array.shape), "dtype": "float32"}
    save_json(state, record)
    return record


def converted_outputs_valid(directory, records):
    return all((directory / x["path"]).is_file() and digest(directory / x["path"]) == x["sha256"]
               and (not x.get("alpha") or ((directory / x["alpha"]["path"]).is_file() and digest(directory / x["alpha"]["path"]) == x["alpha"]["sha256"])) for x in records)


def convert_original(source, original, kind, start, directory, state_dir, recipe_hash, reserve, reuse_pixels):
    original_hash = digest(original["path"])
    identity_key = hashlib.sha256(f"{kind}:{start}:{original['source_path']}".encode()).hexdigest()
    receipt = state_dir / "originals" / (identity_key + ".json")
    if receipt.exists():
        cached = json.loads(receipt.read_text())
        if cached.get("source_sha256") == original_hash and cached.get("recipe_sha256") == recipe_hash and converted_outputs_valid(directory, cached["records"]):
            return cached["records"]
    records = []
    for page, (rgb, alpha, decode_metadata) in enumerate(decode(original["path"], source.get("transfer", "srgb_assumed"))):
        ordinal = start + page
        relative = f"{kind}/{ordinal:06d}.tif"
        identity = {"source_sha256": original_hash, "page": page, "kind": kind}
        output = write_frame(directory / relative, rgb, identity, state_dir / kind, recipe_hash, reserve, reuse_pixels)
        record = {"path": relative, "sha256": output["sha256"], "source_path": original["source_path"],
                  "source_sha256": original_hash, "page": page, "ordinal": ordinal, "shape": output["shape"], "decode": decode_metadata}
        if alpha is not None:
            alpha_path = f"alpha/{kind}-{ordinal:06d}.tif"
            alpha_record = write_frame(directory / alpha_path, alpha, {**identity, "alpha": True}, state_dir / "alpha", recipe_hash, reserve, reuse_pixels)
            record["alpha"] = {"path": alpha_path, "sha256": alpha_record["sha256"], "meaning": "source alpha, not verified focal coverage"}
        records.append(record)
    save_json(receipt, {"source_sha256": original_hash, "recipe_sha256": recipe_hash, "records": records})
    return records


def convert_scene(root, source, scene, recipe, recipe_hash, reserve, workers=2):
    slug = re.sub(r"[^A-Za-z0-9._-]+", "-", scene["source_scene_id"]).strip("-.")[-100:] or "scene"
    scene_name = slug + "-" + hashlib.sha256(scene["source_scene_id"].encode()).hexdigest()[:8]
    directory = root / "standardized" / source["id"] / scene_name
    state_dir = root / METADATA / "work" / "conversion-state" / source["id"] / scene_name
    cached_path = directory / "scene.json"
    cached = None
    if cached_path.is_file():
        cached = json.loads(cached_path.read_text())
        expected = {x["source_path"]: digest(x["path"]) for kind in ["frames", "references"] for x in scene[kind]}
        previous = {x["source_path"]: x["source_sha256"] for kind in ["frames", "references"] for x in cached[kind]}
        outputs_ok = converted_outputs_valid(directory, cached["frames"] + cached["references"])
        if cached.get("recipe_sha256") == recipe_hash and expected == previous and outputs_ok:
            return cached
    frames, references = [], []
    reuse_pixels = bool(cached and cached.get("recipe", {}).get("packages") == recipe["packages"] and cached.get("recipe", {}).get("output") == recipe["output"])
    for kind in ["frames", "references"]:
        jobs, start = [], 0
        for original in scene[kind]:
            count = 1
            if original["path"].suffix.lower() in {".tif", ".tiff", ".stk"}:
                with tifffile.TiffFile(original["path"]) as image:
                    count = len(image.pages)
            jobs.append((original, start)); start += count
        pool = concurrent.futures.ThreadPoolExecutor(max_workers=workers)
        try:
            results = pool.map(lambda job: convert_original(source, job[0], kind, job[1], directory, state_dir, recipe_hash, reserve, reuse_pixels), jobs)
            for result in results:
                (frames if kind == "frames" else references).extend(result)
        except Exception:
            pool.shutdown(wait=True, cancel_futures=True)
            raise
        else:
            pool.shutdown(wait=True)
    if len({tuple(x["shape"]) for x in frames}) > 1:
        raise RuntimeError(f"Frame grids differ within {scene['source_scene_id']}; no resizing performed")
    if len(frames) < 2:
        raise RuntimeError("Optical stack contains fewer than two decoded focal frames")
    # Shared specimen/slide ancestry is conservative; public data stay external.
    family = source["id"] if source["id"] in {"D02", "D03", "D04", "D22"} else source["id"] + "/" + scene["source_scene_id"]
    record = {"schema_version": 1, "dataset_id": source["id"], "scene_id": source["id"] + "/" + scene_name,
              "scene_directory": scene_name, "source_scene_id": scene["source_scene_id"], "leakage_group_id": family, "source_family_id": family,
              "split_role": "external_stress" if source["id"] in {"D02", "D03", "D04"} else "external_regression",
              "original_split": source.get("original_split", "unknown"), "new_train_val_test_assigned": False,
              "reference_type": source.get("reference_type", "none") if references else "none",
              "dataset_reference_type": source.get("reference_type", "none"), "reference_available": bool(references),
              "reference_semantics": source.get("reference_semantics", "unknown"),
              "evaluation_domain": source.get("evaluation_domain", "focus_stacking"),
              "frame_role": "exposure" if source.get("layout") == "exposure_stacks" else "focal",
              "format": {"container": "TIFF", "dtype": "float32", "channels": "RGB", "layout": "HWC", "color_space": "linear-sRGB",
                         "compression": "lossless deflate", "registration": "none", "resize": "none", "focus_order": "natural source filename order; physical order requires capture metadata"},
              "target_contract_status": "not_calibrated", "reference_independence": "not_verified", "recipe": recipe,
              "recipe_sha256": recipe_hash, "frames": frames, "references": references,
              "source_primary_url": source.get("primary_url"), "rights_evidence": source.get("rights_evidence", "unknown")}
    save_json(directory / "scene.json", record)
    return record


def update_index(root):
    manifests = []
    for dataset_path in sorted((root / "standardized").glob("*/dataset.json")):
        dataset = json.loads(dataset_path.read_text())
        manifests.extend(dataset_path.parent / name / "scene.json" for name in dataset["active_scene_directories"])
    scenes = [json.loads(p.read_text()) for p in manifests]
    parents = list(range(len(scenes)))
    def find(i):
        while parents[i] != i:
            parents[i] = parents[parents[i]]; i = parents[i]
        return i
    def union(a, b):
        a, b = find(a), find(b)
        if a != b:
            parents[max(a, b)] = min(a, b)
    owners, families = {}, {}
    for i, scene in enumerate(scenes):
        family = scene.get("source_family_id", scene["leakage_group_id"])
        if family in families:
            union(i, families[family])
        families[family] = i
        for frame in scene["frames"] + scene["references"]:
            value = frame["source_sha256"]
            if value in owners:
                union(i, owners[value])
            owners[value] = i
    groups = {}
    for i, scene in enumerate(scenes):
        groups.setdefault(find(i), []).append(scene["scene_id"])
    index = []
    for i, scene in enumerate(scenes):
        group = "group-" + hashlib.sha256("\n".join(sorted(groups[find(i)])).encode()).hexdigest()[:20]
        source_family = scene.get("source_family_id", scene["leakage_group_id"])
        scene["source_family_id"] = source_family
        scene["leakage_group_id"] = group
        save_json(manifests[i], scene)
        index.append({"scene_id": scene["scene_id"], "dataset_id": scene["dataset_id"], "path": str(manifests[i].relative_to(root / "standardized")),
                      "leakage_group_id": group, "source_family_id": source_family, "frame_count": len(scene["frames"]),
                      "split_role": scene["split_role"], "reference_type": scene["reference_type"]})
    save_json(root / "standardized/index.json", {"schema_version": 1, "scenes": index,
              "grouping_policy": "transitive exact source-byte overlap plus known/conservative specimen families; semantic ancestry remains unverified",
              "new_train_val_test_assigned": False})


def retire_obsolete(root, dataset_id, active):
    parent = root / "standardized" / dataset_id
    for path in sorted(parent.glob("*/scene.json")):
        if path.parent.name in active or path.parent.is_symlink():
            continue
        record = json.loads(path.read_text())
        if record.get("dataset_id") != dataset_id or record.get("scene_directory") != path.parent.name:
            raise RuntimeError(f"Refusing to retire unrecognized generated directory: {path.parent}")
        destination = root / METADATA / "work" / "retired" / dataset_id / (path.parent.name + "-" + digest(path)[:12])
        if not destination.exists():
            destination.parent.mkdir(parents=True, exist_ok=True)
            path.parent.rename(destination)


def main(argv=None):
    parser = argparse.ArgumentParser(prog="bokehforge data process", description=__doc__,
                                     epilog="Logs and reports are saved under <root>/.bokehforge/.")
    parser.add_argument("--root", type=Path)
    parser.add_argument("--config", type=Path, default=CONFIG_DIRECTORY / "sources.json")
    parser.add_argument("--lock", type=Path, default=CONFIG_DIRECTORY / "sources.lock.json")
    parser.add_argument("--dataset", action="append")
    parser.add_argument("--reserve-gib", type=float)
    parser.add_argument("--inventory-only", action="store_true")
    parser.add_argument("--workers", type=int, default=2)
    args = parser.parse_args(argv)
    if not args.config.is_file():
        parser.error(f"Source configuration not found: {args.config}; supply --config")
    configured = json.loads(args.config.read_text())
    defaults = configured.get("defaults", {})
    args.root = (args.root or args.config.parent / defaults.get("data_root", ".")).resolve()
    args.reserve_gib = args.reserve_gib if args.reserve_gib is not None else defaults.get("reserve_gib", 15)
    if args.workers < 1 or args.reserve_gib < 0:
        parser.error("workers must be positive and reserve nonnegative")
    selected = set(args.dataset or [])
    with data_run(args.root, "process"):
        lock = json.loads(args.lock.read_text())
        overrides = {x["id"]: x for x in configured.get("datasets", [])}
        for source in lock["datasets"]:
            if overrides.get(source["id"], {}).get("provider") == "disabled":
                source.update(status="disabled", files=[], reason=overrides[source["id"]].get("reason", "Disabled by configuration"))
            for key in ["layout", "transfer", "conversion_inputs", "conversion_priority", "evaluation_domain"]:
                if key in overrides.get(source["id"], {}):
                    source[key] = overrides[source["id"]][key]
        reserve = int(args.reserve_gib * 1024**3)
        if selected - {x["id"] for x in lock["datasets"]}:
            parser.error("Unknown dataset IDs")
        reports = []
        plan_path = args.root / METADATA / "state/download-plan.json"
        planned = None
        if plan_path.exists():
            planned = {d["id"]: set(d["selected_paths"]) for d in json.loads(plan_path.read_text())["datasets"]}
        elif defaults.get("max_download_bytes") is not None and any(d["id"] in overrides for d in lock["datasets"]):
            raise RuntimeError("Download plan missing; run bokehforge data download before bokehforge data process")
        for source in sorted(lock["datasets"], key=lambda x: (x.get("conversion_priority", 10), x["id"])):
            if selected and source["id"] not in selected:
                continue
            record = {"id": source["id"], "status": source["status"], "scenes": 0, "frames": 0, "quarantined": 0, "errors": []}
            active_scenes = []
            if planned is not None and source["status"] == "available":
                paths = planned.get(source["id"], set())
                if paths - {x["path"] for x in source["files"]}:
                    raise RuntimeError(f"Download plan contains unlocked paths for {source['id']}")
                source = {**source, "files": [x for x in source["files"] if x["path"] in paths]}
                if not source["files"]:
                    record.update(status="budget_excluded", reason="No complete source unit selected within configured budget")
                    reports.append(record)
                    if not args.inventory_only:
                        retire_obsolete(args.root, source["id"], [])
                        save_json(args.root / "standardized" / source["id"] / "dataset.json", {"schema_version": 1,
                                  "dataset_id": source["id"], "status": "budget_excluded", "active_scene_directories": [],
                                  "scene_count": 0, "frame_count": 0, "quarantined": 0})
                    continue
            if source["status"] != "available":
                record["reason"] = source.get("reason", "Source unavailable")
                if source["status"] == "disabled" and not args.inventory_only:
                    retire_obsolete(args.root, source["id"], [])
                    save_json(args.root / "standardized" / source["id"] / "dataset.json", {"schema_version": 1,
                              "dataset_id": source["id"], "status": "disabled", "active_scene_directories": [],
                              "scene_count": 0, "frame_count": 0, "quarantined": 0})
                reports.append(record); continue
            try:
                inventory = inventory_source(args.root, source, reserve)
                if args.inventory_only:
                    record["status"] = "inventoried"
                else:
                    scenes, quarantine = plan_scenes(source, inventory)
                    save_json(args.root / METADATA / "work" / source["id"] / "quarantine.json", quarantine)
                    record["quarantined"] = len(quarantine)
                    recipe, recipe_hash = runtime_recipe(source)
                    for scene in scenes:
                        try:
                            value = convert_scene(args.root, source, scene, recipe, recipe_hash, reserve, args.workers)
                            active_scenes.append(value["scene_directory"])
                            record["scenes"] += 1; record["frames"] += len(value["frames"])
                            print(f"READY {value['scene_id']}: {len(value['frames'])} frames", flush=True)
                        except Exception as exc:
                            record["errors"].append({"scene": scene["source_scene_id"], "error": str(exc)})
                            print(f"ERROR {source['id']}/{scene['source_scene_id']}: {exc}", file=sys.stderr, flush=True)
                            if "disk space" in str(exc):
                                break
                    if not scenes:
                        record["errors"].append({"error": "No complete supported source stacks recognized; inspect work inventory and select an explicit adapter"})
                    record["status"] = "complete" if not record["errors"] else "incomplete"
            except Exception as exc:
                record.update(status="incomplete", errors=[{"error": str(exc)}])
                print(f"ERROR {source['id']}: {exc}", file=sys.stderr, flush=True)
            reports.append(record)
            if not args.inventory_only:
                retire_obsolete(args.root, source["id"], active_scenes)
                save_json(args.root / "standardized" / source["id"] / "dataset.json", {"schema_version": 1,
                          "dataset_id": source["id"], "status": record["status"], "active_scene_directories": active_scenes,
                          "scene_count": record["scenes"], "frame_count": record["frames"], "quarantined": record["quarantined"]})
            save_json(args.root / METADATA / "reports/process-report.json", {"schema_version": 1, "datasets": reports})
        if not args.inventory_only:
            update_index(args.root)
        save_json(args.root / METADATA / "reports/process-report.json", {"schema_version": 1, "datasets": reports})
        return 0 if all(x["status"] in {"complete", "inventoried", "budget_excluded", "disabled"} for x in reports) else 2


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, RuntimeError, ValueError) as exc:
        print(f"ERROR: {exc}", file=sys.stderr)
        raise SystemExit(2)
