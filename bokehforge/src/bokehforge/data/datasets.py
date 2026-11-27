"""Load built-in focus-stacking datasets as a stream of scenes or batches."""

from collections.abc import Iterable
from dataclasses import dataclass
from itertools import batched
import sys
from typing import Final

import numpy as np
from numpy.typing import NDArray

from ._config import DEFAULT_CONFIG, data_root, read_config
from .layout import METADATA, local_source
from .lazy import SceneCatalog

DATASET_INFINIGEN_DEFOCUS: Final = "D29"
DATASET_ALL: Final = "all"


@dataclass(frozen=True)
class Sample:
    """One scene's focal frames in focus order and its reference, if available."""

    scene_id: str
    frames: tuple[NDArray[np.float32], ...]
    reference: NDArray[np.float32] | None


class Batch(tuple[Sample, ...]):
    """An indexable batch of samples at their native resolutions."""

    def frames(self) -> NDArray[np.float32]:
        """Stack equally shaped samples into (B, F, H, W, C)."""
        if not self or not self[0].frames:
            raise ValueError("Cannot stack an empty batch of frames")
        count = len(self[0].frames)
        shape = self[0].frames[0].shape
        if any(len(sample.frames) != count or any(frame.shape != shape for frame in sample.frames)
               for sample in self):
            raise ValueError("Batch frames require the same frame count and shape in every sample")
        return np.stack([frame for sample in self for frame in sample.frames]).reshape(len(self), count, *shape)

    def references(self) -> NDArray[np.float32]:
        """Stack equally shaped references into (B, H, W, C)."""
        references = [sample.reference for sample in self if sample.reference is not None]
        if not references or len(references) != len(self):
            raise ValueError("Batch references require one reference per sample")
        if any(reference.shape != references[0].shape for reference in references):
            raise ValueError("Batch references require the same shape in every sample")
        return np.stack(references)


class Dataset:
    """A reusable scene index that decodes pixels as samples are requested."""

    def __init__(self, catalog: SceneCatalog):
        self._catalog = catalog

    def __len__(self):
        return len(self._catalog.scene_ids)

    def __iter__(self):
        catalog = self._catalog
        for scene_id in catalog.scene_ids:
            scene = catalog.scene(scene_id)
            references = scene.reference_ids if len(scene.reference_ids) == 1 else ()
            frame_ids = scene.frame_ids + references
            decoded = catalog.decoder.decode_many(catalog.frame(frame_id) for frame_id in frame_ids)
            yield Sample(scene_id, tuple(frame.data for frame in decoded[:len(scene.frame_ids)]),
                         decoded[-1].data if references else None)

    def batches(self, batch_size: int):
        """Yield samples in catalog order, including a shorter final batch."""
        return map(Batch, batched(self, batch_size))


def load_dataset(datasets: str | Iterable[str], *, download: bool = True) -> Dataset:
    """Open one or more built-in datasets, downloading any missing originals."""
    configured = read_config(DEFAULT_CONFIG)
    policies = {source["id"]: source for source in configured["datasets"]}
    selected = {datasets} if isinstance(datasets, str) else set(datasets)
    if not selected:
        raise ValueError("Select at least one dataset")
    if unknown := selected - policies.keys() - {DATASET_ALL}:
        raise ValueError(f"Unknown built-in datasets: {', '.join(sorted(unknown))}")
    if DATASET_ALL in selected:
        selected = set(policies)

    root = data_root(DEFAULT_CONFIG, configured.get("defaults", {}))
    missing = sorted(key for key in selected if local_source(root, key, pins=policies[key]) is None)
    if missing:
        names = ", ".join(missing)
        if not download:
            raise FileNotFoundError(f"Datasets {names} are missing or incomplete in {root}; enable downloads to fetch them")
        from .download import download_datasets

        if download_datasets(datasets=missing, root=root, config=DEFAULT_CONFIG, only_missing=True):
            raise RuntimeError(f"Could not download datasets {names}; see {root / METADATA / 'logs/download.log'}")
        print(f"Verifying and indexing datasets {', '.join(sorted(selected))}", file=sys.stderr, flush=True)
    return Dataset(SceneCatalog(root, datasets=sorted(selected), config=DEFAULT_CONFIG))
