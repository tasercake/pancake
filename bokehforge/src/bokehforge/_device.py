"""Device selection for the native score engine."""

from typing import Literal

from . import _rust


Device = Literal["auto", "cpu", "gpu", "cuda", "mps"]
DEVICES = ("auto", "cpu", "gpu", "cuda", "mps")
DamageBackend = Literal["cpu", "cuda", "mps"]


def validate_device(device):
    if device not in DEVICES:
        raise ValueError(f"Expected a scoring device in {DEVICES}, got {device!r}")


def cuda_available():
    return _rust.damage_available("cuda")


def mps_available():
    return _rust.damage_available("mps")


def damage_backend(device: Device) -> DamageBackend:
    validate_device(device)
    return _rust.damage_backend(device)
