import json

import numpy as np
import pytest
from scipy.ndimage import gaussian_filter, shift

from bokehforge import _rust, scoring
from bokehforge._device import cuda_available, damage_backend, mps_available
from bokehforge.scoring import (
    FAMILY_WEIGHTS, FEATURES, INSPECTION_SCORE_VERSION, SCORE_DAMAGE_SCALE, Measurement, measure, scene_score,
)
from scoring_reference import XYZ, integral, lab, lightness_damage


_ACCELERATOR_DEVICES = (
    pytest.param("cuda", marks=pytest.mark.hardware_accelerator),
    pytest.param("mps", marks=pytest.mark.hardware_accelerator, id="metal"),
)


def rgb(pixels):
    return np.repeat(np.asarray(pixels, dtype=np.float32)[..., None], 3, axis=-1)


def score(candidate, target):
    return aligned_measure(candidate, target).score


def aligned_measure(candidate, target):
    return Measurement(*_rust._measure_overlap(candidate, target))


@pytest.mark.parametrize("layout", ["packed", "readonly", "cropped", "strided", "byte-swapped", "unaligned"])
@pytest.mark.parametrize("shape", [(1, 1), (193, 97)])
def test_lab_preparation_keeps_numpy_rounding_and_layouts(layout, shape):
    rng = np.random.default_rng(709)
    # Cross both a comparison strip and the native working buffer boundary.
    candidate = rng.uniform(-0.1, 1.1, (*shape, 3)).astype(np.float32)
    target = rng.uniform(-0.1, 1.1, candidate.shape).astype(np.float32)
    threshold = np.float32((6 / 29) ** 3)
    edge = [np.nextafter(threshold, np.float32(0)), threshold,
            np.nextafter(threshold, np.float32(1)), -0.0, 1.0, np.finfo(np.float32).max]
    edge = np.asarray(edge[:shape[1]], dtype=np.float32)
    candidate[0, :len(edge)] = edge[:, None]

    def view(pixels):
        if layout == "readonly":
            pixels.setflags(write=False)
        elif layout == "cropped":
            pixels = np.pad(pixels, ((0, 0), (2, 3), (0, 0)))[:, 2:-3]
        elif layout == "strided":
            pixels = pixels[::-1, ::2, ::-1]
        elif layout == "byte-swapped":
            pixels = pixels.byteswap().view(pixels.dtype.newbyteorder())
        elif layout == "unaligned":
            buffer = np.empty(pixels.nbytes + 1, dtype=np.uint8)
            unaligned = np.ndarray(pixels.shape, dtype=pixels.dtype, buffer=buffer, offset=1)
            unaligned[:] = pixels
            pixels = unaligned
        return pixels

    candidate, target = view(candidate), view(target)

    reference_candidate, reference_target = lab(candidate), lab(target)
    size = candidate.shape[0] * candidate.shape[1]
    deltas = [c - t for c, t in zip(reference_candidate, reference_target, strict=True)]
    for images, expected in (
        ((candidate,), reference_candidate),
        ((candidate, target), (reference_candidate[0], reference_target[0], *deltas)),
    ):
        if layout in {"strided", "byte-swapped", "unaligned"}:
            images = tuple(np.require(image, dtype=np.float32, requirements=("C", "A")) for image in images)
        xyz = np.empty((len(images) * 3, size), dtype=np.float32)
        actual = np.empty((len(expected), size), dtype=np.float32)
        _rust.lab_xyz(images, XYZ, xyz, 0)
        _rust.lab_planes(xyz, np.cbrt(xyz), tuple(actual))
        for observed, reference in zip(actual, expected, strict=True):
            np.testing.assert_array_equal(observed.view(np.uint32), reference.ravel().view(np.uint32))

    diagnostic = aligned_measure(candidate, target).diagnostics["views"]["native"][0]
    for key, delta in zip(("mean_delta_lightness", "mean_delta_lab_a", "mean_delta_lab_b"), deltas, strict=True):
        # Float32 cube-root rounding is amplified by the Lab opponent-plane scale.
        assert diagnostic[key] == pytest.approx(delta.mean(dtype=np.float64), abs=1.2e-4, rel=0)


def test_cpu_device_selection():
    assert damage_backend("cpu") == "cpu"


@pytest.mark.hardware_accelerator
def test_device_selection_prioritizes_the_available_accelerators(small_delivery):
    supported = {"cuda": cuda_available(), "mps": mps_available()}
    expected = next((device for device, available in supported.items() if available), "cpu")
    assert damage_backend("auto") == expected
    if expected == "cpu":
        with pytest.raises(RuntimeError, match="--device gpu requires CUDA"):
            damage_backend("gpu")
        with pytest.raises(RuntimeError, match="--device gpu requires CUDA"):
            measure(*small_delivery, attention=False, device="gpu")
    else:
        assert damage_backend("gpu") == expected

    for device, available in supported.items():
        if available:
            assert damage_backend(device) == device
        else:
            requirement = {"cuda": "CUDA", "mps": "Apple Metal"}[device]
            with pytest.raises(RuntimeError, match=f"--device {device} requires {requirement}"):
                damage_backend(device)
            with pytest.raises(RuntimeError, match=f"--device {device} requires {requirement}"):
                measure(*small_delivery, attention=False, device=device)


@pytest.fixture
def small_delivery():
    target = np.random.default_rng(2943).uniform(.05, .8, size=(29, 43, 3)).astype(np.float32)
    candidate = target.copy()
    candidate[:, 5:16, 1] *= .65
    return candidate, target


@pytest.mark.parametrize("score_device", _ACCELERATOR_DEVICES, indirect=True)
def test_measure_uses_native_accelerator_on_small_images(score_device, small_delivery):
    device = score_device
    candidate, target = small_delivery
    heatmap = np.linspace(0, 1, target.shape[0] * target.shape[1], dtype=np.float32).reshape(target.shape[:2])
    options = {"details": "tiles", "inspection_heatmap": heatmap}
    expected = measure(candidate, target, device="cpu", **options)
    assert expected.alignment["status"] == "matched"
    assert 0 < expected.score < 100
    (expected_placement,) = expected.explanation["placements"]
    expected_comparison = expected_placement["comparison"]
    requested_devices = (device, "gpu", "auto") if damage_backend("auto") == device else (device,)
    tolerance = 1e-6 if device == "cuda" else 1e-4
    for requested in requested_devices:
        actual = measure(candidate, target, device=requested, **options)
        assert actual.features == pytest.approx(expected.features, abs=tolerance, rel=0)
        assert actual.score / 100 == pytest.approx(expected.score / 100, abs=tolerance, rel=0)
        assert actual.alignment["status"] == "matched"
        (placement,) = actual.explanation["placements"]
        comparison = placement["comparison"]
        assert comparison["diagnostics"] == expected_comparison["diagnostics"]
        assert comparison["measured_pixels"] == expected_comparison["measured_pixels"]
        assert comparison["tile_grids"].keys() == expected_comparison["tile_grids"].keys()
        for name, grid in comparison["tile_grids"].items():
            expected_grid = expected_comparison["tile_grids"][name]
            for key in ("x_edges", "y_edges", "covered_pixels"):
                assert grid[key] == expected_grid[key]
            for family, values in grid["values"].items():
                assert values == pytest.approx(expected_grid["values"][family], abs=tolerance, rel=0)


def test_supplied_attention_is_reported_for_unmatched_delivery():
    rng = np.random.default_rng(7)
    samples = gaussian_filter(rng.normal(size=(120, 150)), 2.8)
    reference = rgb(np.clip(0.5 + 0.12 * samples / samples.std(), 0, 1))
    delivery = np.random.default_rng(19).uniform(0.1, 0.9, size=(75, 112, 3)).astype(np.float32)
    heatmap = np.full(reference.shape[:2], .5, dtype=np.float32)

    expected = scoring._measure_delivery(delivery, reference, details="summary", device="cpu", heatmap=heatmap)
    actual = scoring._measure_delivery(delivery, reference, details="summary", device="cpu", generate=lambda _: heatmap)
    assert actual == expected
    assert actual.explanation["attention"]["mean"] == .5
    assert actual.score == 0


@pytest.mark.parametrize("masked", [False, True])
def test_tile_prefix_keeps_rounding_order_and_support(masked):
    rng = np.random.default_rng(129)
    damage = np.exp2(-rng.integers(0, 128, (213, 83)).astype(np.float32))
    support = rng.random(damage.shape) > 0.2 if masked else None
    y, x = np.r_[0:213:4, 213], np.r_[0:83:4, 83]
    expected = integral(damage if support is None else damage * support)[np.ix_(y, x)]
    mask = None if support is None else support.view(np.uint8)
    actual, affected = _rust.damage_tile_prefix(damage, mask, (y, x), .25)
    assert affected == np.count_nonzero(damage >= .25 if support is None else (damage >= .25) & support)
    np.testing.assert_array_equal(actual, expected)


def test_parallel_damage_uses_full_image_halos():
    rng = np.random.default_rng(67)
    candidate, target = (rng.uniform(-.05, 1.1, size=(517, 1941, 3)).astype(np.float32) for _ in range(2))
    support = rng.random(target.shape[:2]) > 0.08
    cl, tl = lab(candidate)[0], lab(target)[0]
    delta = cl - tl
    expected = lightness_damage(cl, tl, delta, support)
    actual = _rust._accelerated_lightness(cl, tl, delta, support.view(np.uint8), "cpu")
    for left, right in zip(actual, expected, strict=True):
        np.testing.assert_allclose(left, right, rtol=2e-6, atol=8e-6)


@pytest.mark.parametrize(("shape", "coverage"), [
    ((1, 1), "exact"), ((1, 2), "full"), ((33, 43), "empty"),
    ((509, 1967), "partial"), ((2048, 1957), "full"),
])
@pytest.mark.parametrize("score_device", ("cpu", *_ACCELERATOR_DEVICES), indirect=True)
def test_native_lightness_matches_independent_cpu_maps(score_device, shape, coverage):
    rng = np.random.default_rng(135)
    target = rng.uniform(-.1, 1.1, (*shape, 3)).astype(np.float32)
    candidate = target.copy()
    if coverage != "exact":
        candidate[:, :shape[1] // 2, 2] -= np.float32(.24)
        extreme = np.finfo(np.float32).max
        candidate[0, 0, 0], target[0, 0, 0] = -extreme, extreme
    if coverage == "empty":
        support = np.zeros(shape, dtype=bool)
    elif coverage == "partial":
        support = rng.random(shape) > .1
        support[0, -1] = True
    else:
        support = None
    cl = lab(candidate[:, ::-1])[0]
    tl = lab(target[:, ::-1])[0]
    delta = cl - tl
    expected = lightness_damage(cl, tl, delta, support)
    mask = None if support is None else support.view(np.uint8)
    device = score_device
    tolerance = 1e-4 if device == "mps" else 1e-5
    if coverage == "exact":
        tolerance = 0
    actual = _rust._accelerated_lightness(cl, tl, delta, mask, device)
    for observed, reference in zip(actual, expected, strict=True):
        np.testing.assert_allclose(observed, reference, atol=tolerance, rtol=0, err_msg=device)
    if device != "cpu":
        cpu = _rust._accelerated_lightness(cl, tl, delta, mask, "cpu")
        for observed, native in zip(actual, cpu, strict=True):
            np.testing.assert_allclose(observed, native, atol=tolerance, rtol=0, err_msg=device)


def test_exact_match_preserves_grain_bokeh_and_headroom():
    random = np.random.default_rng(4)
    target = random.uniform(-0.03, 1.4, (112, 150, 3)).astype(np.float32)
    target[:55] = gaussian_filter(target[:55], (4, 4, 0))
    measurement = measure(target.copy(), target, attention=False, device="cpu")
    assert scene_score(measurement) == 100
    assert set(measurement.features.values()) == {0}
    assert measurement.diagnostics["linear_rgb_rmse"] == 0
    assert set(measurement.diagnostics["views"]) == {"native"}


def test_exact_crop_full_explanation_serializes_to_json():
    reference = np.random.default_rng(201).random((72, 96, 3), dtype=np.float32)
    candidate = reference[8:-6, 14:-12]
    result = measure(candidate, reference, details="full", attention=False, device="cpu")
    assert result.alignment["placements"][0]["evidence"]["source"] == "exact_native_delivery_in_reference"

    explanation = json.loads(json.dumps(result.explanation, allow_nan=False))
    comparison = explanation["placements"][0]["comparison"]
    tail = comparison["families"]["tone"]["pool"]["tail"]
    covered = comparison["tile_grids"][tail["grid_id"]]["covered_pixels"]
    assert tail["eligible_tile_count"] == sum(pixels > 0 for pixels in covered)


def test_missing_nyquist_detail_grows_continuously_in_four_directions():
    y, x = np.indices((32, 48))
    colors = np.full_like(x, 0.45, dtype=np.float32)
    for carrier in (x, y, x + y, x - y):
        values = []
        for amplitude in (0.014, 0.019, 0.021, 0.028):
            lost = aligned_measure(rgb(colors), rgb(colors + amplitude * (2 * (carrier % 2) - 1)))
            assert lost.features["native.lost_detail"] > 0
            values.append(scene_score(lost))
        assert all(a > b for a, b in zip(values, values[1:]))


def test_color_and_exposure_both_penalize_uniform_fields():
    target = np.full((42, 67, 3), 0.3, dtype=np.float32)
    tone = target + np.float32(0.07)
    tint = np.array([0.04, -0.04 * (0.2126729 / 0.7151522), 0], dtype=np.float32)
    baseline, tinted, both = [measure(delivery, target, attention=False, device="cpu")
                              for delivery in (tone, target + tint, tone + tint)]
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
    error = measure(white * 4, white, attention=False, device="cpu")
    assert error.features["native.range"] > 0
    assert error.features["native.tone"] == 0
    extreme = np.finfo(np.float32).max
    assert np.isfinite(score(np.full_like(target, -extreme), np.full_like(target, extreme)))


def test_native_coverage_finds_localized_defects_and_preserves_coordinates():
    target = np.full((151, 740, 3), 0.28, dtype=np.float32)
    candidate = target.copy()
    candidate[28:44, 196:208] = 0.62
    measurement = measure(candidate, target, attention=False, device="cpu")
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


def test_public_measure_export_preserves_measurement_contract():
    from bokehforge import measure as package_measure

    assert package_measure is measure
    target = np.full((13, 17, 3), 0.28, dtype=np.float32)
    candidate = target + np.float32(0.04)
    measured = package_measure(candidate, target, details="summary", attention=False, device="cpu")
    assert 0 < measured.score < 100
    assert measured.score == measured.explanation["score"]
    assert measured.score == sum(row["score_contribution"] for row in measured.alignment["placements"])


def test_explanation_recomposes_score_from_real_tiles():
    target = np.full((113, 151, 3), 0.28, dtype=np.float32)
    candidate = target.copy()
    candidate[3:31, 131:151] = (0.61, 0.43, 0.30)
    baseline = measure(candidate, target, attention=False, device="cpu")
    summary = measure(candidate, target, details="summary", attention=False, device="cpu")
    detailed = measure(candidate, target, details="tiles", attention=False, device="cpu")
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
        measure(delivery, np.zeros((5, 4, 3), np.float32), device="cpu")


def attention_description(heatmap):
    reference = np.full((*heatmap.shape, 3), .3, dtype=np.float32)
    return measure(reference, reference, inspection_heatmap=heatmap, details="summary",
                   device="cpu").explanation["attention"]


def test_inspection_weights_match_the_formula_and_bound_sparse_maps():
    heatmap = np.array([[0.2, 0.4], [0.8, 1]], dtype=np.float32)
    attention = attention_description(heatmap)
    assert attention["mix"] == 0.5
    expected = (1 + heatmap / heatmap.mean()) / 2
    assert attention["damage_multiplier_range"] == pytest.approx([expected.min(), expected.max()])

    sparse = np.zeros((4, 4), np.float32)
    sparse[0, 0] = 1
    attention = attention_description(sparse)
    assert attention["mix"] < 0.5
    low, high = attention["damage_multiplier_range"]
    assert 1.9 < high < 2
    assert (15 * low + high) / 16 == pytest.approx(1)
    assert low >= 0.5


def test_inspection_weights_match_on_large_contiguous_and_strided_maps():
    heatmap = np.tile(np.array([0.0, 1.0], dtype=np.float32), (1024, 512))
    contiguous = attention_description(heatmap)
    strided = attention_description(heatmap.T[::-1])
    assert contiguous == strided
    assert contiguous["mix"] == 0.5
    assert contiguous["damage_multiplier_range"] == [0.5, 1.5]


def test_inspection_weights_accept_unaligned_float32_heatmaps():
    heatmap = np.ndarray((2, 2), np.float32, buffer=bytearray(17), offset=1)
    heatmap[:] = [[0, 1], [0, 1]]
    assert not heatmap.flags.aligned
    attention = attention_description(heatmap)
    assert attention["mix"] == 0.5
    assert attention["damage_multiplier_range"] == [0.5, 1.5]


def test_inspection_cap_has_a_smooth_shoulder():
    def peak_weight(ratio):
        background = (4 / ratio - 1) / 3
        heatmap = np.array([[1, background, background, background]], np.float32)
        return attention_description(heatmap)["damage_multiplier_range"][1]

    step = 0.001
    at_shoulder = peak_weight(2)
    assert at_shoulder == pytest.approx(1.5)
    assert (at_shoulder - peak_weight(2 - step)) / step == pytest.approx(.5, abs=.002)
    assert (peak_weight(2 + step) - at_shoulder) / step == pytest.approx(.5, abs=.002)
    assert at_shoulder < peak_weight(3) < peak_weight(4) < 2


def test_inspection_weights_feed_the_score_and_its_tiles():
    target = np.full((8, 16, 3), 0.2, np.float32)
    candidate = target + np.float32(0.5)
    heatmap = np.zeros(target.shape[:2], np.float32)
    heatmap[:, :8] = 1
    baseline = measure(candidate, target, attention=False, device="cpu")
    result = measure(candidate, target, inspection_heatmap=heatmap, details="tiles", device="cpu")
    comparison = result.explanation["placements"][0]["comparison"]
    assert comparison["inspection_mix"] == 0.5
    tone = baseline.features["native.tone"]
    assert comparison["tile_grids"]["8:0:0"]["values"]["tone"] == pytest.approx([1.5 * tone, 0.5 * tone])
    assert result.features["native.tone"] == pytest.approx(.4 * tone + .6 * 1.5 * tone)
    assert result.score == pytest.approx(100 * np.exp(-5 * comparison["weighted_damage"]))


def test_flat_inspection_maps_are_neutral_and_perfect_outputs_stay_perfect():
    target = np.full((16, 24, 3), .3, np.float32)
    candidate = target + .04
    baseline = measure(candidate, target, attention=False, device="cpu").score
    for level in (0, .3):
        heatmap = np.full(target.shape[:2], level, np.float32)
        result = measure(candidate, target, inspection_heatmap=heatmap, details="summary", device="cpu")
        assert result.score == baseline
        assert result.explanation["profile"] == INSPECTION_SCORE_VERSION
    heatmap[0, 0] = 1
    assert measure(target, target, inspection_heatmap=heatmap, details="tiles", device="cpu").score == 100
    with pytest.raises(ValueError, match="finite floating-point inspection map"):
        measure(candidate, target, inspection_heatmap=heatmap[:-1], device="cpu")
