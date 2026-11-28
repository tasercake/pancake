"""Score focus stacks and load reproducible focus-stacking datasets."""

from importlib import import_module

_exports = {
    "DATASET_ALL": ".data.datasets",
    "DATASET_INFINIGEN_DEFOCUS": ".data.datasets",
    "Batch": ".data.datasets",
    "Dataset": ".data.datasets",
    "DecodedFrame": ".data.lazy",
    "Frame": ".data.lazy",
    "FrameDecoder": ".data.lazy",
    "FrameWindowDataset": ".data.lazy",
    "FusionPairDataset": ".data.lazy",
    "Sample": ".data.datasets",
    "Scene": ".data.lazy",
    "SceneCatalog": ".data.lazy",
    "load_dataset": ".data.datasets",
    "measure": ".scoring",
}
__all__ = list(_exports)


def __getattr__(name):
    if name not in _exports:
        raise AttributeError(f"module {__name__!r} has no attribute {name!r}")
    value = getattr(import_module(_exports[name], __name__), name)
    globals()[name] = value
    return value


def __dir__():
    return sorted(globals().keys() | _exports.keys())
