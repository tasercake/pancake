"""Scoring report metadata shared by the native driver and Python APIs."""

from typing import Literal

ALIGNMENT_VERSION = 1
MAX_ROTATION_DEGREES = 5.0
SCORE_VERSION = "reference-fidelity-v3"
INSPECTION_SCORE_VERSION = "reference-fidelity-v3-inspection-v2"
INSPECTION_MAP_VERSION = "inspection-blend-v1"
INSPECTION_MIX = 0.5
INSPECTION_MAX_MULTIPLIER = 2.0
SCORE_DAMAGE_SCALE = 5
FAMILY_WEIGHTS = {
    "tone": 0.17, "color": 0.16, "lost_detail": 0.18,
    "added_structure": 0.15, "boundary": 0.23, "range": 0.11,
}
SCORE_EXPLANATION_VERSION = 2
SCORE_DETAIL_LEVELS = ("summary", "tiles", "attention", "full")
ScoreDetails = Literal["summary", "tiles", "attention", "full"]
