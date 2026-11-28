"""Delivery-to-reference similarity and projection backed by the native scorer."""

from dataclasses import dataclass
from math import cos, radians, sin

import numpy as np

from . import _rust
from ._score_contract import ALIGNMENT_VERSION, MAX_ROTATION_DEGREES


@dataclass(frozen=True)
class Pose:
    """Similarity from delivery pixel centers to reference pixel centers in (y, x) order."""

    y: float
    x: float
    angle: float = 0.0
    scale: float = 1.0

    @property
    def matrix(self):
        c, s = cos(radians(self.angle)), sin(radians(self.angle))
        return self.scale * np.array([[c, -s], [s, c]])

    @property
    def offset(self):
        return np.array([self.y, self.x])

    def move_origin(self, yx):
        return Pose(*(self.offset + self.matrix @ yx), self.angle, self.scale)

    def describe(self):
        a, b = self.matrix[1, ::-1]
        c, d = self.matrix[0, ::-1]
        return {"offset_yx": [float(self.y), float(self.x)], "rotation_degrees": float(self.angle),
                "scale": float(self.scale),
                "candidate_to_reference_xy": [[float(a), float(b), float(self.x)],
                                              [float(c), float(d), float(self.y)]]}


@dataclass(frozen=True)
class Placement:
    pose: Pose
    box: tuple
    weight: float
    evidence: dict
    padding: tuple


def _pose_values(pose):
    return pose.y, pose.x, pose.angle, pose.scale


def align(candidate, target):
    """Infer placements and return their registration evidence."""
    rows, trace = _rust.registration_align(candidate, target)
    placements = [Placement(Pose(*pose), tuple(box), weight, evidence, tuple(padding))
                  for pose, box, weight, evidence, padding in rows]
    return placements, trace


def overlap(placement, shape):
    """Return geometric coverage, resolution cap, and the overlap polygon."""
    geometry, resolution, polygon = _rust.registration_overlap(
        _pose_values(placement.pose), placement.box, shape[:2],
    )
    return geometry, resolution, [list(vertex) for vertex in polygon]


def native_overlap(placement, shape):
    """Reference bounds and delivery offsets for an integer-grid placement."""
    y, x, y1, x1 = placement.box
    dy, dx = int(placement.pose.y), int(placement.pose.x)
    return (max(0, y + dy), max(0, x + dx), min(shape[0], y1 + dy), min(shape[1], x1 + dx))


def project(candidate, shape, placement):
    """Project a delivery onto the reference grid with pixel-center support."""
    return _rust.registration_project(candidate, shape[:2], _pose_values(placement.pose), placement.box)
