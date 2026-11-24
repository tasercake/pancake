"""Canonical frame codecs, dataset layouts, and scene provenance."""

import hashlib
import importlib.metadata
import json
from pathlib import Path
import platform
import re

import imagecodecs
import numpy as np
from PIL import Image, ImageCms, ImageOps
import tifffile

from . import _files
from ._files import digest, natural_key

IMAGES = {".png", ".jpg", ".jpeg", ".bmp", ".tif", ".tiff", ".pgm", ".ppm", ".webp", ".stk"}
CAMERA_RAW = {".rw2", ".dng", ".nef", ".cr3", ".cr2", ".arw", ".raf", ".orf"}
TIFF_IMAGES = {".tif", ".tiff", ".stk"}
ARCHIVES = {".zip", ".rar", ".tar", ".tgz", ".gz", ".7z"}
CONVERTER_SHA256 = digest(Path(__file__))
HELPERS_SHA256 = digest(Path(_files.__file__))
SUPPORTED_LAYOUTS = {"explicit_frames", "sood_raw", "hrmff", "edmf", "fod500",
                     "infinigen_defocus", "directory_stacks", "archive_stacks"}
RETIRED_LAYOUTS = frozenset({"mfi_whu"})
INFINIGEN_APERTURES = (1.4, 2.0, 2.8, 4.0, 5.6)
INFINIGEN_DISTANCES = (0.8, 1.2, 1.7, 2.3, 3.0, 3.8, 4.7, 6.0, 8.0)


def runtime_recipe(source, *, loader_sha256):
    import rawpy
    packages = {n: importlib.metadata.version(n) for n in ["numpy", "pillow", "tifffile", "imagecodecs", "rawpy", "libarchive-c"]}
    definition = {"version": 1, "converter_sha256": CONVERTER_SHA256, "packages": packages,
                  "shared_helpers_sha256": HELPERS_SHA256, "loader_sha256": loader_sha256,
                  "runtime": {"python": platform.python_version(), "system": platform.system(), "machine": platform.machine(),
                              "libraw": list(rawpy.libraw_version), "lcms": imagecodecs.cms_version()},
                  "layout": source.get("layout"), "transfer": source.get("transfer", "srgb_assumed"),
                  "source_contract": {k: source.get(k) for k in ["reference_type", "reference_semantics", "original_split", "evaluation_domain", "primary_url", "rights_evidence"]},
                  "output": "float32_HWC_RGB_TIFF_deflate6_predictor3", "resize": False, "alignment": False,
                  "raw": {"gamma": [1, 1], "output_bps": 16, "use_camera_wb": True, "no_auto_bright": True,
                          "output_color": "sRGB", "demosaic": "AHD", "highlight_mode": "Clip", "adjust_maximum_thr": 0.0}}
    signature = hashlib.sha256(json.dumps(definition, sort_keys=True).encode()).hexdigest()
    return definition, signature


def plan_infinigen_scenes(inventory):
    views = {}
    for item in inventory:
        path = Path(item.get("member", item["source_path"]))
        if path.suffix != ".png" or not re.fullmatch(r"[0-9a-f]+_\d+", path.parent.name):
            continue
        if path.name.startswith("image_"):
            views.setdefault(path.parent.name, {})[path.name] = item

    scenes, quarantine = [], []
    for view, images in sorted(views.items()):
        reference = images.get("image_all_in_focus.png")
        for aperture in INFINIGEN_APERTURES:
            name = f"{view}/ap-{aperture:.2f}"
            frames = []
            missing = []
            for distance in INFINIGEN_DISTANCES:
                filename = f"image_ap_{aperture:.2f}_fd_{distance:.2f}.png"
                frame = images.get(filename)
                if frame is None:
                    missing.append(filename)
                else:
                    frames.append({**frame, "aperture_f_number": aperture, "focus_distance_m": distance})
            if reference is None:
                missing.append("image_all_in_focus.png")
            if missing:
                quarantine.append({"scene": name, "reason": "incomplete InfinigenDefocus stack", "missing": missing})
                continue
            scenes.append({"source_scene_id": name, "source_family": view.rsplit("_", 1)[0],
                           "frames": frames, "references": [reference]})
    return scenes, quarantine


def plan_scenes(source, inventory):
    layout = source.get("layout", "directory_stacks")
    if layout not in SUPPORTED_LAYOUTS:
        raise RuntimeError(f"No conversion adapter for layout {layout}; original assets remain unchanged")
    if layout == "infinigen_defocus":
        return plan_infinigen_scenes(inventory)
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
        elif layout == "hrmff":
            match = re.match(r"(.+)[-_]([AB])$", stem, re.I)
            if match:
                scene = match[1]
        elif layout == "edmf":
            match = re.fullmatch(r"bg-[AB]-(\d+)", stem, re.I)
            scene = f"pair-{int(match[1]):04d}" if match else None
        elif layout == "fod500":
            match = re.fullmatch(r"(\d+)_(\d+)All", stem, re.I)
            if match:
                scene = "sample-" + match[1]
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
        if count < 2 or (layout in {"edmf", "hrmff"} and count != 2):
            # A multi-page optical TIFF can contain an entire stack.
            if count != 1 or Path(record["frames"][0]["path"]).suffix.lower() not in TIFF_IMAGES:
                quarantine.append({"scene": name, "reason": f"incomplete stack ({count} source files)", "source_paths": [x["source_path"] for x in record["frames"]]})
                continue
        result.append(record)
    return result, quarantine


def decode(source, transfer, *, suffix=None, page=None):
    """Decode a path or binary stream, optionally selecting one TIFF page."""
    suffix = source.suffix.lower() if suffix is None else suffix.lower()
    if suffix in CAMERA_RAW:
        import rawpy
        with rawpy.imread(str(source) if isinstance(source, Path) else source) as raw:
            array = raw.postprocess(gamma=(1, 1), output_bps=16, use_camera_wb=True, no_auto_bright=True,
                                    output_color=rawpy.ColorSpace.sRGB, demosaic_algorithm=rawpy.DemosaicAlgorithm.AHD,
                                    highlight_mode=rawpy.HighlightMode.Clip, user_flip=0, adjust_maximum_thr=0.0)
            metadata = {"decoder": "LibRaw/rawpy", "native_bit_depth": "camera-dependent", "transfer": "linear",
                        "black_level_per_channel": list(raw.black_level_per_channel), "camera_white_level": int(raw.white_level),
                        "camera_whitebalance": list(raw.camera_whitebalance), "original_orientation_flip": int(raw.sizes.flip),
                        "raw_development": "linear sRGB, camera WB, no auto brightness, AHD, output16; clipped sensor/highlight information not restored"}
        yield array.astype(np.float32) / 65535.0, None, metadata
        return
    if suffix in TIFF_IMAGES:
        with tifffile.TiffFile(source) as image:
            pages = enumerate(image.pages) if page is None else ((page, image.pages[page]),)
            for index, image_page in pages:
                array = image_page.asarray()
                if image_page.planarconfig == 2 and array.ndim == 3:
                    array = np.moveaxis(array, 0, -1)
                orientation = image_page.tags.get("Orientation")
                if orientation and orientation.value != 1:
                    raise RuntimeError("Nontrivial TIFF orientation requires an explicit source adapter")
                profile = image_page.tags.get("InterColorProfile")
                yield normalize(array, transfer, profile.value if profile else None, {"decoder": "tifffile", "page": index})
        return
    with Image.open(source) as image:
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


def encode_tiff(destination, array):
    """Encode an array to a path or binary stream using the canonical TIFF settings."""
    tifffile.imwrite(destination, np.ascontiguousarray(array, dtype=np.float32),
                     photometric="rgb" if array.ndim == 3 else "minisblack",
                     compression="deflate", compressionargs={"level": 6}, predictor=3,
                     metadata=None, datetime=None, software="focus-stacking-standardize/1", maxworkers=1)


def scene_directory(source_scene_id):
    slug = re.sub(r"[^A-Za-z0-9._-]+", "-", source_scene_id).strip("-.")[-100:] or "scene"
    return slug + "-" + hashlib.sha256(source_scene_id.encode()).hexdigest()[:8]


def scene_metadata(source, scene, recipe, recipe_hash, has_references):
    scene_name = scene_directory(scene["source_scene_id"])
    family = source["id"]
    if source["id"] not in {"D02", "D04"}:
        family += "/" + scene.get("source_family", scene["source_scene_id"])
    focus_order = ("increasing focus distance in meters" if source.get("layout") == "infinigen_defocus"
                   else "natural source filename order; physical order requires capture metadata")
    return {"schema_version": 1, "dataset_id": source["id"], "scene_id": source["id"] + "/" + scene_name,
            "scene_directory": scene_name, "source_scene_id": scene["source_scene_id"],
            "leakage_group_id": family, "source_family_id": family,
            "split_role": "external_stress" if source["id"] in {"D02", "D04"} else "external_regression",
            "original_split": source.get("original_split", "unknown"), "new_train_val_test_assigned": False,
            "reference_type": source.get("reference_type", "none") if has_references else "none",
            "dataset_reference_type": source.get("reference_type", "none"), "reference_available": has_references,
            "reference_semantics": source.get("reference_semantics", "unknown"),
            "evaluation_domain": source.get("evaluation_domain", "focus_stacking"), "frame_role": "focal",
            "format": {"container": "TIFF", "dtype": "float32", "channels": "RGB", "layout": "HWC",
                       "color_space": "linear-sRGB", "compression": "lossless deflate", "registration": "none",
                       "resize": "none", "focus_order": focus_order},
            "target_contract_status": "not_calibrated", "reference_independence": "not_verified", "recipe": recipe,
            "recipe_sha256": recipe_hash, "source_primary_url": source.get("primary_url"),
            "rights_evidence": source.get("rights_evidence", "unknown")}


def leakage_groups(scenes):
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
    assignment = {}
    for members in groups.values():
        group = "group-" + hashlib.sha256("\n".join(sorted(members)).encode()).hexdigest()[:20]
        assignment.update(dict.fromkeys(members, group))
    return assignment
