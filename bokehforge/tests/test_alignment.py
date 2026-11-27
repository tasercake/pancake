from functools import partial
import json
from math import cos, radians, sin

import numpy as np
import pytest
from scipy.ndimage import affine_transform, gaussian_filter

from bokehforge.alignment import ALIGNMENT_VERSION
from bokehforge.scoring import measure as _measure


measure = partial(_measure, attention=False)


@pytest.fixture
def target():
    samples = gaussian_filter(np.random.default_rng(7).normal(size=(120, 150)), 2.8)
    light = np.clip(0.5 + 0.12 * samples / samples.std(), 0, 1).astype(np.float32)
    return np.repeat(light[..., None], 3, axis=2)


def render(target, shape, offset, scale=1, angle=0):
    c, s = cos(radians(angle)), sin(radians(angle))
    matrix = scale * np.array([[c, -s], [s, c]])
    return np.stack([affine_transform(plane, matrix, offset, output_shape=shape, order=3, mode="reflect")
                     for plane in np.moveaxis(target, -1, 0)], axis=-1)


@pytest.mark.parametrize("texture", ["textured", "near-flat", "flat"])
def test_exact_crops_earn_area_independent_of_offset_and_texture(target, texture):
    if texture == "near-flat":
        target = np.float32(0.49) + np.float32(1.2e-5) * target
    elif texture == "flat":
        target = np.full_like(target, 0.49)
    for y, x, h, w in [(0, 0, 120, 150), (1, 0, 119, 150), (0, 1, 120, 149), (21, 53, 77, 92),
                       (7, 1, 77, 92), (118, 147, 2, 3), (102, 133, 1, 1)]:
        delivery = target[y:y + h, x:x + w]
        area = h * w / (120 * 150)
        result = measure(delivery, target)
        placement = result.alignment["placements"][0]
        if texture == "textured" or (texture == "near-flat" and h * w > 1):
            assert placement["offset_yx"] == [y, x]
        assert placement["scale"] == 1
        assert placement["geometric_coverage"] == placement["resolution_cap"] == pytest.approx(area)
        assert placement["overlap_score"] == 100
        assert result.score == pytest.approx(100 * area)
        assert result.diagnostics["linear_rgb_rmse"] == 0


def test_tiny_constant_delivery_can_match_noncentral_region():
    target = np.zeros((33, 97, 3), dtype=np.float32)
    target[29:31, 58:60] = [0.2, 0.3, 0.45]
    result = measure(target[29:31, 58:60], target)
    assert result.alignment["placements"][0]["offset_yx"] == [29, 58]
    assert result.score == pytest.approx(100 * 4 / (33 * 97))


def test_exact_search_remains_exhaustive_when_sparse_anchors_are_repetitive():
    target = np.zeros((480, 640, 3), dtype=np.float32)
    target[311, 431] = [0.2, 0.5, 0.9]
    delivery = target[300:420, 400:550].copy()
    delivery[0, 0] = -np.float32(0)
    result = measure(delivery, target)
    assert result.score == pytest.approx(100 * 120 * 150 / (480 * 640))
    assert result.alignment["placements"][0]["offset_yx"] == [300, 400]
    assert result.alignment["stages"][0]["fallback"] == "rolling_rgb_rectangle_hash"


def test_native_sample_refinement_keeps_integer_crop_with_local_damage():
    rng = np.random.default_rng(517)
    field = gaussian_filter(rng.standard_normal((1050, 1370), dtype=np.float32), 3)
    field = 0.50 + 0.12 * field / field.std()
    target = np.stack((field * 0.75, field, field * 0.87), axis=2)
    clean = target[17:-2, 8:-9]
    delivery = clean.copy()
    delivery[123:145, 382:409] += np.float32(0.1)
    result = measure(delivery, target)
    placement = result.alignment["placements"][0]
    assert placement["offset_yx"] == [17, 8]
    assert placement["scale"] == 1 and placement["rotation_degrees"] == 0
    assert placement["geometric_coverage"] == pytest.approx(delivery.size / target.size)
    assert 60 < result.score < measure(clean, target).score
    assert any(stage["method"] == "native_sample_refinement" for stage in result.alignment["stages"])


def test_off_center_resized_delivery_and_same_sized_reexport_keep_real_coverage(target):
    for shape, offset, scale in [((120, 150), (1.95, 12.95), 0.90), ((62, 93), (71, 81.5), 0.66)]:
        result = measure(render(target, shape, offset, scale), target)
        placement = result.alignment["placements"][0]
        area = np.prod(shape) * scale**2 / np.prod(target.shape[:2])
        assert placement["offset_yx"] == pytest.approx(offset, abs=0.65)
        assert placement["scale"] == pytest.approx(scale, abs=0.012)
        assert placement["geometric_coverage"] == pytest.approx(area, abs=0.017)
        assert placement["overlap_score"] > 72
        assert result.score <= 100 * (area + 0.012)


def test_half_size_full_view_has_separate_sample_cap(target):
    delivery = render(target, (60, 75), (0.5, 0.5), 2)
    result = measure(delivery, target)
    placement = result.alignment["placements"][0]
    assert placement["scale"] == pytest.approx(2, abs=0.04)
    assert placement["geometric_coverage"] > 0.98
    assert placement["resolution_cap"] == pytest.approx(0.25, abs=0.002)
    assert 15 < result.score <= 25


def test_small_roll_and_fractional_shift_have_one_similarity_and_fractional_coverage(target):
    offset, angle = (-4.13, 2.37), -2.35
    result = measure(render(target, target.shape[:2], offset, angle=angle), target, details="summary")
    placement = result.alignment["placements"][0]
    assert placement["offset_yx"] == pytest.approx(offset, abs=0.4)
    assert placement["rotation_degrees"] == pytest.approx(angle, abs=0.22)
    assert placement["scale"] == pytest.approx(1, abs=0.008)
    assert 0.92 < placement["geometric_coverage"] < 0.99
    matrix = np.array(placement["candidate_to_reference_xy"])
    assert matrix @ [0, 0, 1] == pytest.approx(placement["offset_yx"][::-1])
    assert matrix[:, :2].T @ matrix[:, :2] == pytest.approx(np.eye(2) * placement["scale"]**2)
    support = result.explanation["placements"][0]["comparison"]["measured_pixels"]
    assert placement["geometric_coverage"] * target.shape[0] * target.shape[1] != pytest.approx(support, abs=0.001)
    assert result.score > 75


@pytest.mark.parametrize("mode, kind", [("constant", "flat_fill"), ("edge", "repeated_edge")])
def test_verified_padding_and_edge_extension_on_oversized_canvas(target, mode, kind):
    delivery = np.pad(target, ((7, 13), (11, 5), (0, 0)), mode=mode)
    result = measure(delivery, target, details="summary")
    placement = result.alignment["placements"][0]
    assert placement["candidate_content_box_xyxy"] == [11, 7, 161, 127]
    assert placement["offset_yx"] == [-7, -11]
    assert placement["geometric_coverage"] == placement["resolution_cap"] == 1
    assert {p["kind"] for p in placement["padding"]} == {kind}
    assert all(p["decision"] == "excluded" for p in placement["padding"])
    assert result.score == 100


def test_oversize_genuine_reference_and_black_damage_remain_in_quality_comparison(target):
    target = np.pad(target, ((8, 5), (12, 0), (0, 0)), mode="constant")
    assert measure(target, target).score == 100
    delivery = target.copy()
    delivery[60:78, 70:92] *= np.float32(0.97)
    result = measure(delivery, target)
    placement = result.alignment["placements"][0]
    assert placement["offset_yx"] == [0, 0]
    assert placement["geometric_coverage"] == 1
    assert placement["candidate_content_box_xyxy"] == [0, 0, target.shape[1], target.shape[0]]
    delivery[-6:] = 0
    border_damage = measure(delivery, target)
    assert border_damage.alignment["placements"][0]["geometric_coverage"] == 1
    assert border_damage.score < result.score


def test_color_only_detail_can_register_and_spatial_distortions_still_cost_points(target):
    signal = target[..., 0] - 0.5
    rgb = 0.5 + signal[..., None] * np.array([0.9, -0.9 * 0.2126 / 0.7152, 0], dtype=np.float32)
    clean = render(rgb, (82, 115), (25, 8), 0.91, 1.24)
    measured = measure(clean, rgb)
    placement = measured.alignment["placements"][0]
    assert placement["offset_yx"] == pytest.approx([25, 8], abs=0.75)
    assert placement["rotation_degrees"] == pytest.approx(1.24, abs=0.3)
    assert placement["scale"] == pytest.approx(0.91, abs=0.015)
    faulty = clean.copy()
    faulty[30:66, 35:79] = clean[30:66, 41:85]
    faulty[::2, ::3, 2] += 0.04
    damaged = measure(faulty, rgb)
    assert damaged.score < measured.score - 4
    assert damaged.features["native.color"] > measured.features["native.color"]


def test_repeat_equivalent_locations_share_full_credit_and_explain_reference_tiles():
    tile = np.random.default_rng(42).uniform(0.2, 0.8, size=(17, 19, 3)).astype(np.float32)
    target = np.tile(tile, (6, 7, 1))
    delivered = target[32:86, 30:101]
    plain = measure(delivered, target)
    detailed = measure(delivered, target, details="tiles")
    alignment = plain.alignment
    assert detailed.alignment == alignment
    assert alignment["schema_version"] == ALIGNMENT_VERSION
    assert alignment["stages"][0]["method"] == "exact_native_delivery_in_reference"
    placements = alignment["placements"]
    assert len(placements) > 1
    assert sum(p["weight"] for p in placements) == pytest.approx(1)
    assert detailed.score == plain.score == pytest.approx(100 * delivered.size / target.size)
    assert sum(p["score_contribution"] for p in placements) == detailed.score
    for placement in detailed.explanation["placements"]:
        y, x = placement["offset_yx"]
        assert placement["candidate_to_reference_xy"] == [[1, 0, x], [0, 1, y]]
        comparison = placement["comparison"]
        pixel_count = delivered.shape[0] * delivered.shape[1]
        assert comparison["measured_pixels"] == pixel_count
        for grid in comparison["tile_grids"].values():
            assert sum(grid["covered_pixels"]) == pixel_count
            assert 0 in grid["covered_pixels"]
    json.dumps(detailed.explanation, allow_nan=False)


def test_unrelated_image_reports_failed_held_out_search_without_damage_tiles(target):
    unrelated = np.random.default_rng(19).uniform(0.1, 0.9, size=(75, 112, 3)).astype(np.float32)
    result = measure(unrelated, target)
    assert result.score == 0
    assert result.explanation is None
    alignment = result.alignment
    assert alignment["status"] == "unmatched"
    assert alignment["reason"] == "weak_image_agreement"
    assert alignment["confidence"] == 0
    assert alignment["placements"] == []
    stages = {stage["method"]: stage for stage in alignment["stages"]}
    assert len(stages["rgb_patch_correlation"]["scale_hypotheses"]) > 8
    assert stages["coarse_to_fine_fit"]["fine_starts"] > 0
    assert all("held_out" in attempt and "decision" in attempt for attempt in alignment["candidates"])
    assert len(json.dumps(alignment, allow_nan=False)) < 55000


def test_no_resolvable_pose_uses_deterministic_near_native_fallback():
    target = np.full((80, 100, 3), 0.26, dtype=np.float32)
    shifted_exposure = target[1:, 1:] + np.float32(0.002)
    result = measure(shifted_exposure, target)
    assert result.alignment["status"] == "uncertain"
    assert result.alignment["reason"] == "low_information_native_fallback"
    assert 90 < result.score < 97.8
