"""Score device selection, without a Torch dependency on the CPU-only path."""

from functools import lru_cache
from importlib.util import find_spec
from typing import Literal


Device = Literal["auto", "cpu", "gpu"]
DEVICES = ("auto", "cpu", "gpu")


def validate_device(device):
    if device not in DEVICES:
        raise ValueError(f"Expected a scoring device in {DEVICES}, got {device!r}")


@lru_cache(maxsize=1)
def cuda_available():
    if find_spec("torch") is None:
        return False
    import torch

    return torch.cuda.is_available()


def use_cuda_damage(device):
    validate_device(device)
    if device == "cpu":
        return False
    if cuda_available() and find_spec("triton") is not None:
        return True
    if device == "gpu":
        raise RuntimeError("--device gpu requires CUDA and the inspection dependencies with Triton")
    return False
