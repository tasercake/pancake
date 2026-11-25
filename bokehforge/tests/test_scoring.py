import numpy as np
import pytest
from scipy.ndimage import gaussian_filter, shift

from bokehforge.scoring import FEATURES, Measurement, measure, scene_score


def rgb(pixels):
    return np.repeat(np.asarray(pixels, dtype=np.float32)[..., None], 3, axis=-1)


def score(candidate, target):
    return scene_score(measure(candidate, target))


def test_exact_match_preserves_grain_bokeh_and_headroom():
    random = np.random.default_rng(4)
    target = random.uniform(-0.03, 1.4, (112, 150, 3)).astype(np.float32)
    target[:55] = gaussian_filter(target[:55], (4, 4, 0))
    measurement = measure(target.copy(), target)
    assert scene_score(measurement) == 100
    assert set(measurement.features.values()) == {0}
    assert measurement.diagnostics["linear_rgb_rmse"] == 0
    assert len(measurement.diagnostics["views"]["zoom"]) == 5


def test_missing_nyquist_detail_grows_continuously_in_four_directions():
    y, x = np.indices((32, 48))
    colors = np.full_like(x, 0.45, dtype=np.float32)
    for carrier in (x, y, x + y, x - y):
        values = []
        for amplitude in (0.014, 0.019, 0.021, 0.028):
            lost = measure(rgb(colors), rgb(colors + amplitude * (2 * (carrier % 2) - 1)))
            assert lost.features["native.lost_detail"] > 0
            values.append(scene_score(lost))
        assert all(a > b for a, b in zip(values, values[1:]))


def test_color_and_exposure_both_penalize_uniform_fields():
    target = np.full((42, 67, 3), 0.3, dtype=np.float32)
    tone = target + np.float32(0.07)
    tint = np.array([0.04, -0.04 * (0.2126729 / 0.7151522), 0], dtype=np.float32)
    baseline, tinted, both = [measure(delivery, target) for delivery in (tone, target + tint, tone + tint)]
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
    error = measure(white * 4, white)
    assert error.features["native.range"] > 0
    assert error.features["native.tone"] == 0
    extreme = np.finfo(np.float32).max
    assert np.isfinite(score(np.full_like(target, -extreme), np.full_like(target, extreme)))


def test_native_coverage_finds_localized_defects_and_preserves_coordinates():
    target = np.full((151, 740, 3), 0.28, dtype=np.float32)
    candidate = target.copy()
    candidate[28:44, 196:208] = 0.62
    measurement = measure(candidate, target)
    views = measurement.diagnostics["views"]
    assert views["fit"][0]["view_size_wh"][0] == 640
    assert max(zoom["view_size_wh"][0] for zoom in views["zoom"]) == 192
    native = views["native"][0]["families"]["tone"]
    left, top, right, bottom = native["worst_box_native_xyxy"]
    assert left < 208 and right > 196 and top < 44 and bottom > 28
    assert native["tail"] > native["mean"]
    assert scene_score(measurement) < 100


def test_fixed_profile_combines_view_and_family_weights():
    features = dict.fromkeys(FEATURES, 0.0)
    features.update({"fit.color": 0.2, "native.boundary": 0.6, "zoom.lost_detail": 0.3})
    damage = 0.30 * 0.16 * 0.2 + 0.50 * 0.23 * 0.6 + 0.20 * 0.18 * 0.3
    assert scene_score(Measurement(features, {})) == pytest.approx(100 * np.exp(-5 * damage))
    assert scene_score(Measurement(dict.fromkeys(FEATURES, 1.0), {})) == pytest.approx(100 * np.exp(-5))


@pytest.mark.parametrize("delivery", [np.zeros((5, 4, 3)), np.zeros((4, 4, 3), np.float32),
                                       np.full((5, 4, 3), np.nan, np.float32), np.zeros((5, 4, 4), np.float32)])
def test_bad_deliveries_fail_before_measurement(delivery):
    with pytest.raises(ValueError, match="finite float32 linear-sRGB"):
        measure(delivery, np.zeros((5, 4, 3), np.float32))
