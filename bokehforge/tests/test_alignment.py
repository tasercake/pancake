import numpy as np
import pytest
from scipy.ndimage import gaussian_filter, rotate, zoom

from bokehforge.alignment import evaluate_crop
from bokehforge.scoring import measure, scene_score


@pytest.fixture
def target():
    samples = gaussian_filter(np.random.default_rng(7).normal(size=(118, 151)), 2.4)
    light = np.clip(0.5 + 0.12 * samples / samples.std(), 0, 1).astype(np.float32)
    return np.repeat(light[..., None], 3, axis=2)


def test_perfect_off_center_crops_score_in_proportion_to_area(target):
    for top, bottom, left, right in [(3, 110, 1, 147), (45, 90, 53, 117), (98, 118, 120, 151)]:
        delivered = target[top:bottom, left:right].copy()
        expected = delivered.size / target.size
        score, measurement, alignment = evaluate_crop(delivered, target)
        placement = alignment["placements"][0]
        assert placement["offset_yx"] == [top, left]
        assert placement["rotation_degrees"] == 0
        assert placement["coverage"] == pytest.approx(expected)
        assert placement["overlap_score"] == 100
        assert score == pytest.approx(100 * expected)
        assert measurement.diagnostics["linear_rgb_rmse"] == 0


def test_off_center_crop_is_measured_on_the_native_reference_grid():
    rng = np.random.default_rng(44)
    target = rng.uniform(0.25, 0.75, size=(230, 650, 3)).astype(np.float32)
    delivered = target[99:121, 113:138]
    score, measurement, alignment = evaluate_crop(delivered, target)
    assert alignment["placements"][0]["offset_yx"] == [99, 113]
    assert set(measurement.diagnostics["views"]) == {"native"}
    native, = measurement.diagnostics["views"]["native"]
    assert native["view_size_wh"] == (650, 230)
    coverage = delivered.size / target.size
    assert native["covered_fraction"] == pytest.approx(coverage)
    assert score == pytest.approx(100 * coverage)


def test_small_roll_is_recovered_and_costs_coverage(target):
    delivered = rotate(target, 2.35, axes=(1, 0), reshape=False, order=1, mode="reflect")
    score, _, alignment = evaluate_crop(delivered, target)
    placement = alignment["placements"][0]
    assert placement["offset_yx"] == [0, 0]
    assert placement["rotation_degrees"] == pytest.approx(-2.35, abs=0.06)
    assert 0.95 < placement["coverage"] < 1
    assert score > scene_score(measure(delivered, target)) + 30
    assert score == pytest.approx(placement["coverage"] * placement["overlap_score"])


def test_cropping_keeps_real_color_and_movement_errors(target):
    clean = target[8:112, 16:140]
    delivered = clean.copy()
    delivered[..., 0] += 0.05
    # Replace a patch with a shifted piece of the subject.
    delivered[34:59, 30:62] = clean[34:59, 33:65]
    score, measurement, alignment = evaluate_crop(delivered, target)
    assert alignment["placements"][0]["offset_yx"] == pytest.approx([8, 16], abs=0.3)
    assert score < 0.9 * 100 * clean.size / target.size
    assert measurement.features["native.color"] > 0
    assert measurement.features["native.boundary"] > 0


def test_unrelated_crop_and_scaled_thumbnail_do_not_earn_high_scores(target):
    unrelated = np.random.default_rng(14).random((59, 75, 3), dtype=np.float32)
    assert evaluate_crop(unrelated, target)[0] == 0
    thumbnail = zoom(target, (0.5, 0.5, 1), order=1)
    assert evaluate_crop(thumbnail, target)[0] < 10


def test_equivalent_repeating_placements_share_credit():
    tile = np.random.default_rng(42).uniform(0.2, 0.8, size=(24, 25, 3)).astype(np.float32)
    target = np.tile(tile, (6, 7, 1))
    delivered = target[52:116, 29:107]
    score, _, alignment = evaluate_crop(delivered, target)
    placements = alignment["placements"]
    assert len(placements) > 1
    assert sum(placement["weight"] for placement in placements) == pytest.approx(1)
    assert all(placement["overlap_score"] == 100 for placement in placements)
    assert score == pytest.approx(100 * delivered.size / target.size)


def test_same_grid_and_uniform_crop(target):
    for delivered in [target.copy(), np.clip(target + 0.06, 0, 1)]:
        score, _, alignment = evaluate_crop(delivered, target)
        assert score == scene_score(measure(delivered, target))
        assert alignment["placements"][0]["rotation_degrees"] == 0

    flat = np.full((11, 11, 3), 0.25, dtype=np.float32)
    score, _, alignment = evaluate_crop(flat[:8, :9], flat)
    assert score == pytest.approx(100 * 8 * 9 / (11 * 11))
    assert alignment["placements"][0]["coverage"] == pytest.approx(8 * 9 / (11 * 11))
