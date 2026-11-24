"""Python interface for algorithms run by `bokehforge bench`."""

from collections.abc import Iterator, Sequence
from dataclasses import dataclass
from typing import Protocol

import numpy as np
from numpy.typing import NDArray


@dataclass(frozen=True)
class StackFrame:
    """Canonical focal pixels and optional native alpha, both read-only float32."""

    id: str
    ordinal: int
    pixels: NDArray[np.float32]
    alpha: NDArray[np.float32] | None
    focus_distance_m: float | None
    aperture_f_number: float | None


@dataclass(frozen=True)
class StackInput:
    """One scene. Consume the single-pass stream in catalog order during the call."""

    scene_id: str
    frame_ids: Sequence[str]
    target_shape: tuple[int, int, int]
    frames: Iterator[StackFrame]


class Stacker(Protocol):
    """Return finite, linear-sRGB float32 HWC RGB on the declared target grid."""

    def __call__(self, stack: StackInput, /) -> NDArray[np.float32]: ...


def mean(stack: StackInput, /) -> NDArray[np.float32]:
    """Uniform blend baseline for equal-sized frames, without alignment or focus selection."""
    total = np.zeros(stack.target_shape, dtype=np.float64)
    for frame in stack.frames:
        if frame.pixels.shape != stack.target_shape:
            raise ValueError("The mean stacker requires equal-sized focal grids")
        total += frame.pixels
    total /= len(stack.frame_ids)
    return total.astype(np.float32)
