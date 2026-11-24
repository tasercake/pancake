"""Index native focus stacks and decode their frames on demand."""

from bisect import bisect_right
from contextlib import ExitStack, contextmanager
from dataclasses import dataclass, field
from hashlib import file_digest, sha256
from io import BytesIO
import json
from operator import index
from pathlib import Path, PurePosixPath
from stat import S_ISLNK
from zipfile import ZipFile

import numpy as np
import tifffile

from ._canonical import (
    ARCHIVES, CAMERA_RAW, IMAGES, RETIRED_LAYOUTS, TIFF_IMAGES, decode, encode_tiff,
    leakage_groups, plan_scenes, runtime_recipe, scene_metadata,
)
from ._files import digest, safe_path

_LOADER_SHA256 = digest(Path(__file__))


def _file_stamp(path):
    stat = path.stat()
    return stat.st_dev, stat.st_ino, stat.st_size, stat.st_mtime_ns, stat.st_ctime_ns


@dataclass(frozen=True)
class _Asset:
    path: Path
    sha256: str
    stamp: tuple[int, ...]

    def check(self):
        if _file_stamp(self.path) != self.stamp:
            raise ValueError(f"Native file changed after catalog construction: {self.path}")


@dataclass(frozen=True)
class _Original:
    asset: _Asset
    member: str | None
    suffix: str
    sha256: str
    pages: int


@dataclass(frozen=True)
class Scene:
    id: str
    frame_ids: tuple[str, ...]
    reference_ids: tuple[str, ...]
    metadata: dict


@dataclass(frozen=True)
class Frame:
    id: str
    scene_id: str
    source_path: str
    source_sha256: str
    page: int
    ordinal: int
    role: str
    metadata: dict
    _original: _Original = field(repr=False)
    _transfer: str = field(repr=False)
    _recipe_sha256: str = field(repr=False)


@dataclass(frozen=True)
class DecodedFrame:
    data: np.ndarray | bytes
    alpha: np.ndarray | bytes | None
    provenance: dict


def _page_count(source, suffix):
    if suffix in TIFF_IMAGES:
        with tifffile.TiffFile(source) as image:
            return len(image.pages)
    return 1


def _inventory_member(asset, name, members):
    path = PurePosixPath(name.replace("\\", "/"))
    if path.is_absolute() or not path.parts or ".." in path.parts:
        raise ValueError(f"Unsafe archive member: {asset.path}::{name}")
    if path in members:
        raise ValueError(f"Duplicate archive member: {asset.path}::{name}")
    members.add(path)
    return path


def _optical_member(path):
    return (path.suffix.lower() in IMAGES | CAMERA_RAW and not path.name.startswith(".")
            and "__MACOSX" not in path.parts and "scalebar" not in path.as_posix().lower())


def _archive_inventory(asset, relative):
    members = set()
    if asset.path.suffix.lower() == ".zip":
        with ZipFile(asset.path) as archive:
            for entry in archive.infolist():
                if S_ISLNK(entry.external_attr >> 16):
                    raise ValueError(f"Unsafe archive symlink: {asset.path}::{entry.filename}")
                path = _inventory_member(asset, entry.filename, members)
                if entry.is_dir() or not _optical_member(path):
                    continue
                with archive.open(entry) as stream:
                    checksum = file_digest(stream, "sha256").hexdigest()
                    if path.suffix.lower() in TIFF_IMAGES:
                        stream.seek(0)
                    pages = _page_count(stream, path.suffix.lower())
                yield _member_record(asset, relative, entry.filename, path, checksum, pages)
    else:
        import libarchive

        with libarchive.file_reader(str(asset.path)) as archive:
            for entry in archive:
                if entry.issym or entry.islnk:
                    raise ValueError(f"Unsafe archive link: {asset.path}::{entry.pathname}")
                path = _inventory_member(asset, entry.pathname, members)
                if entry.isdir or not _optical_member(path):
                    continue
                payload = b"".join(entry.get_blocks())
                with BytesIO(payload) as stream:
                    pages = _page_count(stream, path.suffix.lower())
                yield _member_record(asset, relative, entry.pathname, path, sha256(payload).hexdigest(), pages)


def _member_record(asset, relative, name, path, checksum, pages):
    member = path.as_posix()
    return {"path": path, "source_path": f"{relative}::{member}", "archive": relative,
            "member": member, "original": _Original(asset, name, path.suffix.lower(), checksum, pages)}


def _native_inventory(raw_root, source):
    inventory = []
    for item in source["files"]:
        relative = item["path"]
        path = safe_path(raw_root, relative)
        stamp = _file_stamp(path)
        checksum = item.get("sha256")
        if not checksum or digest(path) != checksum:
            raise ValueError(f"Missing or mismatched source SHA-256: {source['id']}/{relative}")
        asset = _Asset(path, checksum, stamp)
        suffix = path.suffix.lower()
        if suffix in ARCHIVES:
            inventory.extend(_archive_inventory(asset, relative))
        elif suffix in IMAGES | CAMERA_RAW:
            original = _Original(asset, None, suffix, checksum, _page_count(path, suffix))
            inventory.append({"path": path, "source_path": relative, "spec": item, "original": original})
        asset.check()
    return inventory


class SceneCatalog:
    """Index a verified native corpus without decoding pixels or writing files."""

    def __init__(self, root, *, datasets=None, lock=None, config=None):
        raw_root = Path(root).resolve() / "raw"
        selected = {datasets} if isinstance(datasets, str) else set(datasets or ())
        if lock is None:
            manifests = sorted(raw_root.glob("*/download-manifest.json"))
            sources = [json.loads(path.read_text()) for path in manifests
                       if not selected or path.parent.name in selected]
        else:
            sources = json.loads(Path(lock).read_text())["datasets"]
        # Older locks and download manifests may still list the retired MFI-WHU source.
        sources = [source for source in sources if source.get("layout") not in RETIRED_LAYOUTS]
        missing = selected - {source["id"] for source in sources}
        if missing:
            raise ValueError(f"Unknown or undownloaded datasets: {sorted(missing)}")
        if not sources:
            raise ValueError(f"No downloaded datasets found in {raw_root}")
        configured = json.loads(Path(config).read_text()).get("datasets", []) if config is not None else []
        overrides = {source["id"]: source for source in configured}

        self.decoder = FrameDecoder()
        self.quarantine = {}
        self._scenes = {}
        self._frames = {}
        grouping_records = []
        for source in sorted(sources, key=lambda item: item["id"]):
            dataset_id = source["id"]
            if selected and dataset_id not in selected:
                continue
            policy = overrides.get(dataset_id, {})
            if source["status"] == "disabled" or policy.get("provider") == "disabled":
                continue
            if source["status"] != "available":
                raise ValueError(f"Dataset {dataset_id} is {source['status']}")
            source = {**source, **{key: policy[key] for key in ("layout", "transfer", "evaluation_domain") if key in policy}}
            recipe, recipe_hash = runtime_recipe(source, loader_sha256=_LOADER_SHA256)
            plans, quarantine = plan_scenes(source, _native_inventory(safe_path(raw_root, dataset_id), source))
            self.quarantine[dataset_id] = quarantine
            for plan in plans:
                if sum(item["original"].pages for item in plan["frames"]) < 2:
                    quarantine.append({"scene": plan["source_scene_id"], "reason": "fewer than two focal pages"})
                    continue
                metadata = scene_metadata(source, plan, recipe, recipe_hash,
                                          any(item["original"].pages for item in plan["references"]))
                scene_id = metadata["scene_id"]
                frames = self._index_frames(metadata, plan["frames"], "frames")
                references = self._index_frames(metadata, plan["references"], "references")
                self._scenes[scene_id] = Scene(scene_id, frames, references, metadata)
                grouping_records.append({
                    **metadata,
                    **{role: [{"source_sha256": self.frame(frame_id).source_sha256} for frame_id in ids]
                       for role, ids in (("frames", frames), ("references", references))},
                })
        for scene_id, group in leakage_groups(grouping_records).items():
            self.scene(scene_id).metadata["leakage_group_id"] = group

    def _index_frames(self, metadata, items, role):
        ids = []
        scene_id = metadata["scene_id"]
        for item in items:
            original = item["original"]
            focus = {key: item[key] for key in ("aperture_f_number", "focus_distance_m") if key in item}
            for page in range(original.pages):
                ordinal = len(ids)
                frame_id = f"{scene_id}/{role}/{ordinal:06d}.tif"
                self._frames[frame_id] = Frame(
                    frame_id, scene_id, item["source_path"], original.sha256, page, ordinal, role,
                    focus, original, metadata["recipe"]["transfer"], metadata["recipe_sha256"],
                )
                ids.append(frame_id)
        return tuple(ids)

    @property
    def scene_ids(self):
        return tuple(self._scenes)

    def scene(self, scene_id: str) -> Scene:
        return self._scenes[scene_id]

    def frame(self, frame_id: str) -> Frame:
        return self._frames[frame_id]

    def decode(self, frame_id: str, *, output: str = "array") -> DecodedFrame:
        """Load one focal frame or reference by its catalog ID."""
        return self.decoder.decode(self.frame(frame_id), output=output)


@contextmanager
def _frame_source(original, archives, stack):
    asset = original.asset
    if original.member is None:
        yield asset.path
        return
    if asset.path.suffix.lower() == ".zip":
        if asset.path not in archives:
            archives[asset.path] = stack.enter_context(ZipFile(asset.path))
        with archives[asset.path].open(original.member) as stream:
            yield stream
        return
    import libarchive

    with libarchive.file_reader(str(asset.path)) as archive:
        for entry in archive:
            if entry.pathname == original.member:
                payload = b"".join(entry.get_blocks())
                if sha256(payload).hexdigest() != original.sha256:
                    raise ValueError(f"Archive member checksum changed: {asset.path}::{original.member}")
                with BytesIO(payload) as stream:
                    yield stream
                return
    raise ValueError(f"Archive member missing: {asset.path}::{original.member}")


def _tiff_bytes(array):
    with BytesIO() as buffer:
        encode_tiff(buffer, array)
        return buffer.getvalue()


def _validate_output(output):
    if output not in ("array", "tiff"):
        raise ValueError("output must be 'array' or 'tiff'")


class FrameDecoder:
    """Decode frames from a SceneCatalog and optionally encode canonical TIFF bytes."""

    def decode(self, frame, *, output="array"):
        return self.decode_many((frame,), output=output)[0]

    def decode_many(self, frames, *, output="array"):
        """Decode the requested frames in order, sharing ZIP handles for this call."""
        _validate_output(output)
        with ExitStack() as stack:
            archives = {}
            return tuple(self._decode_frame(frame, output, archives, stack) for frame in frames)

    def _decode_frame(self, frame, output, archives, stack):
        original = frame._original
        original.asset.check()
        with _frame_source(original, archives, stack) as source:
            [(rgb, alpha, metadata)] = decode(source, frame._transfer, suffix=original.suffix, page=frame.page)
        original.asset.check()
        provenance = {
            "frame_id": frame.id, "scene_id": frame.scene_id, "role": frame.role,
            "source_path": frame.source_path, "source_sha256": frame.source_sha256,
            "asset_sha256": original.asset.sha256, "recipe_sha256": frame._recipe_sha256,
            "page": frame.page, "ordinal": frame.ordinal, "shape": tuple(rgb.shape),
            "decode": metadata, **frame.metadata,
        }
        if alpha is not None:
            provenance["alpha"] = {"meaning": "source alpha, not verified focal coverage"}
        if output == "tiff":
            rgb = _tiff_bytes(rgb)
            provenance["sha256"] = sha256(rgb).hexdigest()
            if alpha is not None:
                alpha = _tiff_bytes(alpha)
                provenance["alpha"]["sha256"] = sha256(alpha).hexdigest()
        return DecodedFrame(rgb, alpha, provenance)


class FrameWindowDataset:
    """Index complete sliding windows within each scene."""

    def __init__(self, catalog, *, window_size=8, stride=1, scene_ids=None, output="array"):
        self._init_windows(catalog, window_size, stride, scene_ids, output, require_reference=False)

    def _init_windows(self, catalog, window_size, stride, scene_ids, output, *, require_reference):
        self.window_size, self.stride = index(window_size), index(stride)
        if self.window_size < 1 or self.stride < 1:
            raise ValueError("window_size and stride must be positive")
        _validate_output(output)
        self.catalog = catalog
        self.output = output
        self._require_reference = require_reference
        self._scenes = []
        self._ends = []
        end = 0
        if scene_ids is None:
            scene_ids = catalog.scene_ids
        elif isinstance(scene_ids, str):
            scene_ids = (scene_ids,)
        for scene_id in scene_ids:
            scene = catalog.scene(scene_id)
            if require_reference and len(scene.reference_ids) != 1:
                continue
            count = (len(scene.frame_ids) - self.window_size) // self.stride + 1
            if count > 0:
                end += count
                self._scenes.append(scene)
                self._ends.append(end)

    def __len__(self):
        return self._ends[-1] if self._ends else 0

    def __getitem__(self, item):
        item = index(item)
        if item < 0:
            item += len(self)
        if item < 0 or item >= len(self):
            raise IndexError(item)
        scene_index = bisect_right(self._ends, item)
        first = self._ends[scene_index - 1] if scene_index else 0
        start = (item - first) * self.stride
        scene = self._scenes[scene_index]
        ids = scene.frame_ids[start:start + self.window_size]
        requested = ids + scene.reference_ids if self._require_reference else ids
        frames = self.catalog.decoder.decode_many((self.catalog.frame(frame_id) for frame_id in requested), output=self.output)
        focal = frames[:self.window_size]
        if len({frame.provenance["shape"] for frame in focal}) != 1:
            raise ValueError(f"Focal frame grids differ within {scene.id}")
        sample = {"scene_id": scene.id, "frame_ids": ids, "frames": tuple(frame.data for frame in focal)}
        if any(frame.alpha is not None for frame in focal):
            sample["alpha"] = tuple(frame.alpha for frame in focal)
        if self._require_reference:
            reference = frames[-1]
            sample.update(reference=reference.data, reference_id=scene.reference_ids[0])
            if reference.alpha is not None:
                sample["reference_alpha"] = reference.alpha
        return sample


class FusionPairDataset(FrameWindowDataset):
    """Index adjacent focal pairs from scenes with one unambiguous reference."""

    def __init__(self, catalog, *, scene_ids=None, output="array"):
        self._init_windows(catalog, 2, 1, scene_ids, output, require_reference=True)
