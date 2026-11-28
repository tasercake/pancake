"""Download and load reproducible focus-stacking datasets."""

from .data.datasets import DATASET_ALL, DATASET_INFINIGEN_DEFOCUS, Batch, Dataset, Sample, load_dataset
from .data.lazy import DecodedFrame, Frame, FrameDecoder, FrameWindowDataset, FusionPairDataset, Scene, SceneCatalog

__all__ = [
    "DATASET_ALL",
    "DATASET_INFINIGEN_DEFOCUS",
    "Batch",
    "Dataset",
    "DecodedFrame",
    "Frame",
    "FrameDecoder",
    "FrameWindowDataset",
    "FusionPairDataset",
    "Sample",
    "Scene",
    "SceneCatalog",
    "load_dataset",
    "measure",
]


def __getattr__(name):
    if name == "measure":
        from .scoring import measure

        globals()[name] = measure
        return measure
    raise AttributeError(f"module {__name__!r} has no attribute {name!r}")
