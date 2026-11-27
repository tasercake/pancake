"""Score device selection, without a Torch dependency on the CPU-only path."""

from functools import lru_cache
from importlib.util import find_spec
from typing import Literal


Device = Literal["auto", "cpu", "gpu", "cuda", "mps"]
DEVICES = ("auto", "cpu", "gpu", "cuda", "mps")
DamageBackend = Literal["cpu", "cuda", "mps"]


def validate_device(device):
    if device not in DEVICES:
        raise ValueError(f"Expected a scoring device in {DEVICES}, got {device!r}")


@lru_cache(maxsize=1)
def cuda_available():
    if find_spec("torch") is None:
        return False
    import torch

    return torch.cuda.is_available()


@lru_cache(maxsize=1)
def mps_available():
    if find_spec("torch") is None:
        return False
    import torch

    return torch.backends.mps.is_available()


def damage_backend(device: Device) -> DamageBackend:
    validate_device(device)
    if device == "cpu":
        return "cpu"
    if device in ("auto", "gpu", "cuda") and cuda_available() and find_spec("triton") is not None:
        return "cuda"
    if device in ("auto", "gpu", "mps") and mps_available():
        return "mps"
    if device == "auto":
        return "cpu"
    requirements = {
        "gpu": "CUDA with Triton or Apple Metal (MPS) with PyTorch",
        "cuda": "CUDA with PyTorch and Triton",
        "mps": "Apple Metal (MPS) with PyTorch",
    }
    raise RuntimeError(f"--device {device} requires {requirements[device]}")
