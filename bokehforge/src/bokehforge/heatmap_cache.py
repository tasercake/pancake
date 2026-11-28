"""Build and read cached heatmap galleries from verified dataset references."""

from contextlib import closing
from hashlib import sha256
from io import BytesIO
import json
import sqlite3
from threading import Lock

import numpy as np
from PIL import Image
from scipy.interpolate import PchipInterpolator
from scipy.ndimage import gaussian_filter

from .bench import has_benchmark_reference
from .data._canonical import RETIRED_LAYOUTS
from .data.download import download_datasets
from .data.layout import METADATA, exclusive, local_source
from .data.lazy import SceneCatalog
from .inspection import INSPECTION_MAP_VERSION, _model_directory, reference_preview
from .inspection_models import blend_components, detail_light, inspection_components
from .scoring import validate_heatmap

_MODELS = {
    "inspection": ("Inspection blend", "Soft union of four signals"),
    "reliqs": ("ReLIQS", "Learned patch importance"),
    "sam2": ("SAM 2.1", "Detail-weighted segmentation masks"),
    "tempsal": ("TempSAL", "Predicted attention over time"),
    "detail-light": ("Texture & light", "Texture, shading and color contrast"),
    "center": ("Center only", "Image-center reference"),
}
PROFILES = {"baseline": "Texture & light", "inspection": "Inspection models"}
_PALETTE = ((15, 20, 39), (34, 48, 110), (36, 92, 150), (57, 149, 178), (90, 205, 180), (252, 234, 121))
_LEVELS = PchipInterpolator([0, .1, .2, .5, .8, .9, 1], [0, .2, .5, .8, .9, .9, 1])
_RENDER_VERSION = 1


def _colorize(values):
    stops = (0, .18, .40, .58, .78, 1)
    channels = [np.interp(values, stops, channel) for channel in zip(*_PALETTE)]
    return Image.fromarray(np.stack(channels, axis=-1).round().astype(np.uint8))


def _processed(raw):
    scale = max(raw.shape) / 640
    spread = 1 + np.log(gaussian_filter(np.exp(18 * (raw - 1)), 6 * scale)) / 18
    dilated = gaussian_filter(np.maximum(raw, spread), 2 * scale)
    return .5 * (raw + _LEVELS(np.clip(dilated, 0, 1)))


def _maps(preview, predict):
    if predict is None:
        maps = {"detail-light": detail_light(preview)}
    else:
        maps = predict(preview)
        maps["inspection"] = blend_components(maps)
    y, x = np.ogrid[-1:1:preview.height * 1j, -1:1:preview.width * 1j]
    maps["center"] = np.exp(-2 * (x ** 2 + y ** 2)).astype(np.float32)
    return maps


def _png(image):
    with BytesIO() as output:
        image.save(output, format="PNG")
        return output.getvalue()


def _render_scene(scene, reference, predict, url_prefix, group):
    preview = reference_preview(reference)
    key = sha256(scene.id.encode()).hexdigest()[:24]
    pngs = [(f"{key}-photo", _png(preview))]
    variants = {}
    for model, raw in _maps(preview, predict).items():
        raw = validate_heatmap(raw, (preview.height, preview.width))
        variants[model] = {}
        for mode, values in (("raw", raw), ("processed", _processed(raw))):
            name = f"{key}-{model}-{mode}"
            pngs.append((name, _png(_colorize(values))))
            variants[model][mode] = f"{url_prefix}/{name}.png"
    return {
        "id": scene.id, "title": scene.metadata["source_scene_id"], "group": group,
        "width": preview.width, "height": preview.height, "photo": f"{url_prefix}/{key}-photo.png", "maps": variants,
    }, pngs


def _read_cache(path, statement, parameters=()):
    try:
        with closing(sqlite3.connect(f"{path.as_uri()}?mode=ro", uri=True)) as connection:
            return connection.execute(statement, parameters).fetchone()
    except sqlite3.DatabaseError:
        return None


class HeatmapCache:
    def __init__(self, root, sources):
        self.root = root
        self.directory = root / METADATA / "heatmaps"
        self.sources = {
            item["id"]: item for item in sources
            if item.get("provider") != "disabled" and item.get("layout") not in RETIRED_LAYOUTS
        }
        self.revisions = {
            key: sha256(json.dumps([_RENDER_VERSION, INSPECTION_MAP_VERSION, source], sort_keys=True).encode()).hexdigest()[:20]
            for key, source in self.sources.items()
        }

    def path(self, dataset, profile):
        return self.directory / profile / f"{dataset}-{self.revisions[dataset]}.sqlite3"

    def gallery(self, dataset, profile):
        row = _read_cache(self.path(dataset, profile), "SELECT gallery FROM manifest")
        if row:
            try:
                gallery = json.loads(row[0])
                if isinstance(gallery, dict) and gallery.get("scenes") and isinstance(gallery.get("models"), dict):
                    return gallery
            except ValueError:
                pass
        return None

    def image(self, dataset, profile, name):
        row = _read_cache(self.path(dataset, profile), "SELECT png FROM images WHERE name = ?", (name,))
        return row[0] if row else None

    def references(self, dataset, report):
        source = self.sources[dataset]
        name = source["name"]
        if local_source(self.root, dataset, pins=source) is None:
            report(f"Preparing download for {name}")
            files = {}
            lock = Lock()

            def on_download(event):
                kind = event["type"]
                with lock:
                    if kind == "inventory":
                        files.update({item["path"]: (0, item["total"]) for item in event["files"]})
                    elif kind == "file":
                        files[event["path"]] = (event["received"], event["total"])
                        received, totals = zip(*files.values())
                        if all(total is not None for total in totals):
                            report(f"Downloading and verifying {name}", sum(received), sum(totals), "bytes")

            if download_datasets(root=self.root, datasets=[dataset], only_missing=True, progress=on_download):
                raise RuntimeError(f"Could not download {name}. See the BokehForge download log for details.")
        report(f"Verifying and indexing {name}")
        catalog = SceneCatalog(self.root, datasets=[dataset])
        scenes = [catalog.scene(scene_id) for scene_id in catalog.scene_ids]
        scenes = [scene for scene in scenes if has_benchmark_reference(scene)]
        if not scenes:
            raise ValueError(f"{name} has no all-in-focus RGB references")
        return catalog, scenes

    def generate(self, datasets, profile, report, on_ready):
        with exclusive(self.directory / "locks", "generation.lock"):
            pending = []
            for dataset in datasets:
                if gallery := self.gallery(dataset, profile):
                    on_ready(dataset, gallery)
                else:
                    pending.append(dataset)
            references = [(dataset, *self.references(dataset, report)) for dataset in pending]
            total = sum(len(scenes) for _, _, scenes in references)
            if not total:
                return

            predict = None
            if profile == "inspection":
                report("Preparing Inspection models")

                def model_progress(path, phase, received, size):
                    if phase == "downloading":
                        report(f"Downloading {path.rsplit('/', 1)[-1]}", received, size, "bytes")
                    else:
                        report(f"Preparing {path.rsplit('/', 1)[-1]}")

                predict = inspection_components(_model_directory(), progress=model_progress)
            done = 0
            for dataset, catalog, scenes in references:
                name = self.sources[dataset]["name"]

                def progress(completed):
                    report(f"Generating heatmaps for {name}", done + completed, total)

                progress(0)
                gallery = self._write_dataset(dataset, profile, catalog, scenes, predict, progress)
                on_ready(dataset, gallery)
                done += len(scenes)

    def _write_dataset(self, dataset, profile, catalog, scenes, predict, progress):
        path = self.path(dataset, profile)
        path.parent.mkdir(parents=True, exist_ok=True)
        name = self.sources[dataset]["name"]
        url_prefix = f"/images/{profile}/{dataset}"
        # Publish the manifest and images together after the database is complete.
        staging = path.with_suffix(".pending")
        journal = staging.with_name(staging.name + "-journal")
        staging.unlink(missing_ok=True)
        journal.unlink(missing_ok=True)
        try:
            with closing(sqlite3.connect(staging)) as database, database:
                database.execute("CREATE TABLE images (name TEXT PRIMARY KEY, png BLOB NOT NULL)")
                database.execute("CREATE TABLE manifest (gallery TEXT NOT NULL)")
                items = []
                models = set()
                for completed, scene in enumerate(scenes, 1):
                    reference = catalog.decode(scene.reference_ids[0]).data
                    item, pngs = _render_scene(scene, reference, predict, url_prefix, name)
                    database.executemany("INSERT INTO images VALUES (?, ?)", pngs)
                    models.update(item["maps"])
                    items.append(item)
                    progress(completed)
                gallery = {"scenes": items, "models": {model: title for model, title in _MODELS.items() if model in models}}
                database.execute("INSERT INTO manifest VALUES (?)", (json.dumps(gallery),))
            staging.replace(path)
        finally:
            staging.unlink(missing_ok=True)
            journal.unlink(missing_ok=True)
        return gallery
