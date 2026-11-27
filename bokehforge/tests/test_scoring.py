import numpy as np
import pytest
from scipy.ndimage import gaussian_filter, shift

from bokehforge.scoring import (
    FAMILY_WEIGHTS, FEATURES, INSPECTION_SCORE_VERSION, SCORE_DAMAGE_SCALE, Measurement,
    _inspection_weights, _measure, measure, scene_score,
)


def rgb(pixels):
    return np.repeat(np.asarray(pixels, dtype=np.float32)[..., None], 3, axis=-1)


def score(candidate, target):
    return scene_score(_measure(candidate, target))


def test_exact_match_preserves_grain_bokeh_and_headroom():
    random = np.random.default_rng(4)
    target = random.uniform(-0.03, 1.4, (112, 150, 3)).astype(np.float32)
    target[:55] = gaussian_filter(target[:55], (4, 4, 0))
    measurement = measure(target.copy(), target, attention=False)
    assert scene_score(measurement) == 100
    assert set(measurement.features.values()) == {0}
    assert measurement.diagnostics["linear_rgb_rmse"] == 0
    assert set(measurement.diagnostics["views"]) == {"native"}


def test_missing_nyquist_detail_grows_continuously_in_four_directions():
    y, x = np.indices((32, 48))
    colors = np.full_like(x, 0.45, dtype=np.float32)
    for carrier in (x, y, x + y, x - y):
        values = []
        for amplitude in (0.014, 0.019, 0.021, 0.028):
            lost = _measure(rgb(colors), rgb(colors + amplitude * (2 * (carrier % 2) - 1)))
            assert lost.features["native.lost_detail"] > 0
            values.append(scene_score(lost))
        assert all(a > b for a, b in zip(values, values[1:]))


def test_color_and_exposure_both_penalize_uniform_fields():
    target = np.full((42, 67, 3), 0.3, dtype=np.float32)
    tone = target + np.float32(0.07)
    tint = np.array([0.04, -0.04 * (0.2126729 / 0.7151522), 0], dtype=np.float32)
    baseline, tinted, both = [measure(delivery, target, attention=False) for delivery in (tone, target + tint, tone + tint)]
    assert tinted.features["native.color"] > 0
    assert both.features["native.color"] > baseline.features["native.color"]
    assert both.features["native.tone"] == pytest.approx(baseline.features["native.tone"], abs=2e-6)
    assert scene_score(both) < min(scene_score(baseline), scene_score(tinted)) < 100


@pytest.mark.parametrize("artifact", ["blur", "displacement", "ghost", "halo", "noise"])
def test_controlled_optical_artifact_severity(artifact):
    y, x = np.indices((112, 144))
    target = rgb(0.17 + 0.49 * (x >= 64) + 0.043 * np.sin(x * 1.25 + y * 0.53))
    blurred = gaussian_filter(target, (1.8, 1.8, 0))
    shifted = shift(target, (0, 3, 0), order=1, mode="nearest")
    noise = np.random.default_rng(7).normal(0, 0.2, target.shape).astype(np.float32)
    damage = {"blur": blurred - target, "displacement": shifted - target, "ghost": 0.5 * (shifted - target),
              "halo": target - blurred, "noise": noise}[artifact]
    if artifact == "displacement":
        # Fractional-pixel fixed-grid error, with the same interpolation at each severity.
        outputs = [shift(target, (0, amount, 0), order=1, mode="nearest") for amount in (0.125, 0.25, 0.5, 1)]
    else:
        outputs = [target + np.float32(amount) * damage for amount in (0.125, 0.25, 0.5, 1)]
    values = [100, *(score(output, target) for output in outputs)]
    assert all(a > b for a, b in zip(values, values[1:]))


def test_range_loss_does_not_hide_in_display_clipping():
    target = np.full((24, 32, 3), 0.61, dtype=np.float32)
    scores = [score(np.full_like(target, level), target) for level in (0.98, 1, 1.02, 4)]
    assert scores == sorted(scores, reverse=True)
    white = np.ones_like(target)
    error = measure(white * 4, white, attention=False)
    assert error.features["native.range"] > 0
    assert error.features["native.tone"] == 0
    extreme = np.finfo(np.float32).max
    assert np.isfinite(score(np.full_like(target, -extreme), np.full_like(target, extreme)))


def test_native_coverage_finds_localized_defects_and_preserves_coordinates():
    target = np.full((151, 740, 3), 0.28, dtype=np.float32)
    candidate = target.copy()
    candidate[28:44, 196:208] = 0.62
    measurement = measure(candidate, target, attention=False)
    views = measurement.diagnostics["views"]
    assert set(views) == {"native"}
    view, = views["native"]
    assert view["view_size_wh"] == (740, 151)
    assert set(measurement.features) == set(FEATURES)
    native = view["families"]["tone"]
    left, top, right, bottom = native["worst_box_native_xyxy"]
    assert left < 208 and right > 196 and top < 44 and bottom > 28
    assert native["tail"] > native["mean"]
    assert scene_score(measurement) < 100


def test_fixed_profile_combines_native_family_weights():
    features = dict.fromkeys(FEATURES, 0.0)
    features.update({"native.color": 0.2, "native.boundary": 0.6, "native.lost_detail": 0.3})
    damage = 0.16 * 0.2 + 0.23 * 0.6 + 0.18 * 0.3
    assert scene_score(Measurement(features, {})) == pytest.approx(100 * np.exp(-5 * damage))
    assert scene_score(Measurement(dict.fromkeys(FEATURES, 1.0), {})) == pytest.approx(100 * np.exp(-5))


def test_explanation_recomposes_score_from_real_tiles():
    target = np.full((113, 151, 3), 0.28, dtype=np.float32)
    candidate = target.copy()
    candidate[3:31, 131:151] = (0.61, 0.43, 0.30)
    baseline = measure(candidate, target, attention=False)
    summary = measure(candidate, target, details="summary", attention=False)
    detailed = measure(candidate, target, details="tiles", attention=False)
    assert baseline.explanation is None
    assert summary.score == detailed.score == baseline.score
    assert summary.features == detailed.features == baseline.features
    assert summary.diagnostics == detailed.diagnostics == baseline.diagnostics
    explanation = detailed.explanation["placements"][0]["comparison"]
    assert explanation["reference_size_wh"] == [151, 113]
    assert "tile_grids" not in summary.explanation
    assert explanation["weighted_damage"] == summary.explanation["placements"][0]["comparison"]["weighted_damage"]
    grids = explanation["tile_grids"]
    assert len(grids) == 8
    for grid in grids.values():
        assert grid["x_edges"][0] == grid["y_edges"][0] == 0
        assert grid["x_edges"][-1] == 151 and grid["y_edges"][-1] == 113
        tile_count = (len(grid["x_edges"]) - 1) * (len(grid["y_edges"]) - 1)
        assert grid["covered_pixels"] is None
        assert all(len(values) == tile_count for values in grid["values"].values())

    for name, family in explanation["families"].items():
        pool = family["pool"]
        assert family["weight"] == FAMILY_WEIGHTS[name]
        assert family["burden"] == pytest.approx(sum(term["contribution"] for term in pool.values()))
        assert family["weighted_damage"] == family["burden"] * family["weight"]
        assert pool["mean"]["value"] == baseline.diagnostics["views"]["native"][0]["families"][name]["mean"]
        tail = pool["tail"]
        values = grids[tail["grid_id"]]["values"][name]
        selected = [values[index] for index in tail["tile_indices"]]
        assert sorted(selected) == sorted(values)[-tail["tile_count"]:]
        assert np.mean(selected) == pytest.approx(tail["value"])
        grid_tails = []
        for grid in grids.values():
            values = sorted(grid["values"][name])
            count = max(1, int(np.ceil(0.01 * len(values))))
            grid_tails.append(np.mean(values[-count:]))
        assert tail["value"] == pytest.approx(max(grid_tails))
        worst = pool["worst"]
        assert worst["value"] == max(max(grid["values"][name]) for grid in grids.values())
        if worst["tile_index"] is not None:
            grid = grids[worst["grid_id"]]
            y, x = divmod(worst["tile_index"], len(grid["x_edges"]) - 1)
            assert worst["box_xyxy"] == [grid["x_edges"][x], grid["y_edges"][y],
                                         grid["x_edges"][x + 1], grid["y_edges"][y + 1]]
    damage = sum(family["weighted_damage"] for family in explanation["families"].values())
    assert explanation["weighted_damage"] == damage
    assert detailed.score == 100 * np.exp(-SCORE_DAMAGE_SCALE * damage)


@pytest.mark.parametrize(("delivery", "reason"), [
    (np.zeros((5, 4, 3)), "uses float64 samples"),
    (np.full((5, 4, 3), np.nan, np.float32), "non-finite pixel values"),
    (np.zeros((5, 4, 4), np.float32), "requires a nonempty RGB image with three channels"),
])
def test_bad_deliveries_fail_before_measurement(delivery, reason):
    with pytest.raises(ValueError, match=reason):
        measure(delivery, np.zeros((5, 4, 3), np.float32))


def test_inspection_weights_match_the_formula_and_bound_sparse_maps():
    heatmap = np.array([[0.2, 0.4], [0.8, 1]], dtype=np.float32)
    weights, mix = _inspection_weights(heatmap)
    assert mix == 0.5
    np.testing.assert_allclose(weights, (1 + heatmap / heatmap.mean()) / 2, rtol=1e-6)

    sparse = np.zeros((4, 4), np.float32)
    sparse[0, 0] = 1
    weights, mix = _inspection_weights(sparse)
    assert mix < 0.5
    assert 1.9 < weights.max() < 2
    assert weights.mean() == pytest.approx(1)
    assert weights.min() >= 0.5


def test_inspection_cap_has_a_smooth_shoulder():
    def peak_weight(ratio):
        background = (4 / ratio - 1) / 3
        heatmap = np.array([1, background, background, background], np.float32)
        return float(_inspection_weights(heatmap)[0].max())

    step = 0.001
    at_shoulder = peak_weight(2)
    assert at_shoulder == pytest.approx(1.5)
    assert (at_shoulder - peak_weight(2 - step)) / step == pytest.approx(.5, abs=.002)
    assert (peak_weight(2 + step) - at_shoulder) / step == pytest.approx(.5, abs=.002)
    assert at_shoulder < peak_weight(3) < peak_weight(4) < 2


def test_inspection_weights_feed_the_score_and_its_tiles(monkeypatch):
    target = np.full((8, 16, 3), 0.2, np.float32)
    candidate = target + np.float32(0.5)
    heatmap = np.zeros(target.shape[:2], np.float32)
    heatmap[:, :8] = 1
    monkeypatch.setattr("bokehforge.scoring._maps", lambda *_: (
        (np.ones(target.shape[:2], np.float32), *[np.zeros(target.shape[:2], np.float32) for _ in range(5)]), {}
    ))
    weights = _inspection_weights(heatmap)
    result = _measure(candidate, target, details="tiles", inspection=weights)
    assert result.explanation["inspection_mix"] == 0.5
    assert result.explanation["tile_grids"]["8:0:0"]["values"]["tone"] == [1.5, 0.5]
    assert result.features["native.tone"] == pytest.approx(.4 * 1 + .6 * 1.5)
    assert result.score == pytest.approx(100 * np.exp(-5 * FAMILY_WEIGHTS["tone"] * result.features["native.tone"]))


def test_flat_inspection_maps_are_neutral_and_perfect_outputs_stay_perfect():
    target = np.full((16, 24, 3), .3, np.float32)
    candidate = target + .04
    baseline = measure(candidate, target, attention=False).score
    for level in (0, .3):
        heatmap = np.full(target.shape[:2], level, np.float32)
        result = measure(candidate, target, inspection_heatmap=heatmap, details="summary")
        assert result.score == baseline
        assert result.explanation["profile"] == INSPECTION_SCORE_VERSION
    heatmap[0, 0] = 1
    assert measure(target, target, inspection_heatmap=heatmap, details="tiles").score == 100
    with pytest.raises(ValueError, match="finite floating-point inspection map"):
        measure(candidate, target, inspection_heatmap=heatmap[:-1])
