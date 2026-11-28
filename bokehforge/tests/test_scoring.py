import numpy as np
import pytest
from scipy.ndimage import gaussian_filter, shift

from bokehforge import scoring
from bokehforge._device import cuda_available, damage_backend, mps_available
from bokehforge.scoring import (
    FAMILY_WEIGHTS, FEATURES, INSPECTION_SCORE_VERSION, SCORE_DAMAGE_SCALE, Measurement,
    _damage, _inspection_weights, _integral, _maps, _measure, _tile_integral, measure, scene_score,
)


def rgb(pixels):
    return np.repeat(np.asarray(pixels, dtype=np.float32)[..., None], 3, axis=-1)


def score(candidate, target):
    return scene_score(_measure(candidate, target))


@pytest.mark.parametrize("layout", ["packed", "readonly", "cropped", "strided", "byte-swapped", "unaligned"])
@pytest.mark.parametrize("shape", [(1, 1), (193, 97)])
def test_lab_preparation_keeps_numpy_rounding_and_layouts(layout, shape, monkeypatch):
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

    def planes():
        cl, tl, deltas, color = scoring._lab_comparison(candidate, target)
        return *scoring._lab(candidate), cl, tl, *deltas, color

    actual = planes()
    monkeypatch.setattr(scoring, "_rust", None)
    for observed, expected in zip(actual, planes(), strict=True):
        np.testing.assert_array_equal(observed.view(np.uint32), expected.view(np.uint32))


def test_device_selection_falls_back_only_in_auto(monkeypatch):
    monkeypatch.setattr("bokehforge._device.cuda_available", lambda: False)
    monkeypatch.setattr("bokehforge._device.mps_available", lambda: False)
    assert damage_backend("auto") == "cpu"
    assert damage_backend("cpu") == "cpu"
    with pytest.raises(RuntimeError, match="--device gpu requires CUDA"):
        damage_backend("gpu")
    with pytest.raises(RuntimeError, match="--device cuda requires CUDA"):
        damage_backend("cuda")
    with pytest.raises(RuntimeError, match="--device mps requires Apple Metal"):
        damage_backend("mps")


def test_device_selection_prefers_supported_gpu_backends(monkeypatch):
    monkeypatch.setattr("bokehforge._device.cuda_available", lambda: True)
    monkeypatch.setattr("bokehforge._device.find_spec", lambda _: None)
    monkeypatch.setattr("bokehforge._device.mps_available", lambda: True)
    assert damage_backend("auto") == "mps"
    assert damage_backend("gpu") == "mps"
    assert damage_backend("cpu") == "cpu"
    with pytest.raises(RuntimeError, match="--device cuda requires CUDA"):
        damage_backend("cuda")
    monkeypatch.setattr("bokehforge._device.find_spec", lambda _: object())
    assert damage_backend("auto") == "cuda"
    assert damage_backend("gpu") == "cuda"
    assert damage_backend("cuda") == "cuda"
    assert damage_backend("mps") == "mps"
    monkeypatch.setattr("bokehforge._device.mps_available", lambda: False)
    with pytest.raises(RuntimeError, match="--device mps requires Apple Metal"):
        damage_backend("mps")


@pytest.fixture
def small_delivery():
    target = np.random.default_rng(2943).uniform(.05, .8, size=(29, 43, 3)).astype(np.float32)
    candidate = target.copy()
    candidate[:, 5:16, 1] *= .65
    return candidate, target


@pytest.mark.parametrize("device", ["cuda", "mps"])
def test_measure_uses_native_accelerator_on_small_images(device, small_delivery, monkeypatch):
    try:
        damage_backend(device)
    except RuntimeError as exc:
        pytest.skip(str(exc))

    maps = getattr(scoring, f"_{device}_maps")
    shapes = []

    def record_maps(candidate, target, support):
        shapes.append(target.shape)
        return maps(candidate, target, support)

    monkeypatch.setattr(scoring, f"_{device}_maps", record_maps)
    candidate, target = small_delivery
    heatmap = np.linspace(0, 1, target.shape[0] * target.shape[1], dtype=np.float32).reshape(target.shape[:2])
    options = {"details": "tiles", "inspection_heatmap": heatmap}
    expected = measure(candidate, target, device="cpu", **options)
    assert measure(candidate, target, device=device, **options) == expected
    assert target.shape in shapes
    if damage_backend("auto") == device:
        shapes.clear()
        assert measure(candidate, target, **options) == expected
        assert target.shape in shapes


@pytest.mark.parametrize("exhausted", [True, False])
def test_only_auto_retries_accelerator_out_of_memory(small_delivery, monkeypatch, exhausted):
    backend = damage_backend("auto")
    if backend == "cpu":
        pytest.skip("Requires a scoring accelerator")

    if not exhausted:
        failure = RuntimeError("Accelerator operation failed")
    elif backend == "mps":
        failure = RuntimeError("MPS backend out of memory")
    else:
        from torch import OutOfMemoryError

        failure = OutOfMemoryError("CUDA out of memory")
    attempts = []

    def fail_maps(candidate, target, support):
        attempts.append(target.shape)
        raise failure

    monkeypatch.setattr(scoring, f"_{backend}_maps", fail_maps)
    candidate, target = small_delivery
    baseline = measure(candidate, target, attention=False, device="cpu")
    if exhausted:
        assert measure(candidate, target, attention=False) == baseline
    else:
        with pytest.raises(type(failure), match=str(failure)):
            measure(candidate, target, attention=False)
    assert attempts == [target.shape]

    for forced in (backend, "gpu"):
        with pytest.raises(type(failure), match=str(failure)):
            measure(candidate, target, attention=False, device=forced)
    assert attempts == [target.shape] * 3


def test_measure_rejects_cuda_inspection_for_mps(small_delivery):
    with pytest.raises(RuntimeError, match="Inspection profile requires CUDA"):
        measure(*small_delivery, device="mps")


def test_generated_attention_is_reported_for_unmatched_delivery(monkeypatch):
    pixels = rgb(np.zeros((1000, 1000), dtype=np.float32))
    heatmap = np.full(pixels.shape[:2], .5, dtype=np.float32)

    def align(candidate, reference):
        return [], {
            "status": "unmatched", "reason": "unmatched", "confidence": 0.,
            "candidate_size_wh": [1000, 1000], "reference_size_wh": [1000, 1000],
        }

    def unexpected_backend(device):
        pytest.fail("An unmatched delivery does not need a damage backend")

    monkeypatch.setattr(scoring, "align", align)
    monkeypatch.setattr(scoring, "damage_backend", unexpected_backend)
    expected = scoring._measure_delivery(pixels, pixels, details="summary", device="auto", heatmap=heatmap)
    actual = scoring._measure_delivery(pixels, pixels, details="summary", device="auto", generate=lambda _: heatmap)
    assert actual == expected
    assert actual.explanation["attention"]["mean"] == .5
    assert actual.score == 0


@pytest.mark.parametrize("masked", [False, True])
def test_tile_prefix_keeps_rounding_order_and_support(masked):
    rng = np.random.default_rng(129)
    damage = np.exp2(-rng.integers(0, 128, (213, 83)).astype(np.float32))
    support = rng.random(damage.shape) > 0.2 if masked else None
    y, x = np.r_[0:213:4, 213], np.r_[0:83:4, 83]
    expected = _integral(damage if support is None else damage * support)[np.ix_(y, x)]
    np.testing.assert_array_equal(_tile_integral(damage, support, (y, x)), expected)


def test_parallel_damage_uses_full_image_halos():
    rng = np.random.default_rng(67)
    candidate, target = (rng.uniform(-.05, 1.1, size=(517, 1941, 3)).astype(np.float32) for _ in range(2))
    support = rng.random(target.shape[:2]) > 0.08
    expected, _ = _damage(candidate, target, support)
    actual, _ = _maps(candidate, target, support)
    for left, right in zip(actual, expected, strict=True):
        np.testing.assert_array_equal(left, right)


@pytest.mark.parametrize(("shape", "masked"), [((29, 43), False), ((509, 1967), True)])
def test_gpu_preserves_tile_values_diagnostics_and_scores(shape, masked):
    if damage_backend("auto") != "cuda":
        pytest.skip("Requires CUDA and Triton")
    rng = np.random.default_rng(65)
    target = rng.uniform(-.05, 1.1, size=(*shape, 3)).astype(np.float32)
    candidate = target.copy()
    candidate[:, 5:16, 1] *= .65
    candidate[23, 20, 0], target[23, 20, 0] = -np.finfo(np.float32).max, np.finfo(np.float32).max
    support = (rng.random(target.shape[:2]) > .08) if masked else None
    attention = _inspection_weights(rng.random(target.shape[:2], dtype=np.float32))
    cpu = _measure(candidate, target, support, details="tiles", inspection=attention)
    gpu = _measure(candidate, target, support, details="tiles", inspection=attention, backend="cuda")
    assert cpu == gpu
    assert cpu.score == gpu.score


@pytest.mark.parametrize("shape", [(1, 2), (33, 43), (215, 89)])
def test_mps_edge_path_preserves_scores_and_explanations(monkeypatch, shape):
    pytest.importorskip("torch")
    from bokehforge import _mps_damage

    # Exercise the same tensor operations on MPS when present, and otherwise
    # check their CPU or CUDA implementation against the reference scorer.
    tensor_device = "mps" if mps_available() else "cuda" if cuda_available() else "cpu"
    for name in ("lightness_extrema", "neighborhood_strength"):
        operation = getattr(_mps_damage, name)
        monkeypatch.setattr(_mps_damage, name, lambda values, operation=operation: operation(values, device=tensor_device))
    rng = np.random.default_rng(135)
    target = rng.uniform(-.1, 1.1, (*shape, 3)).astype(np.float32)
    candidate = target.copy()
    candidate[:, :shape[1] // 2, 2] -= np.float32(.24)
    support = rng.random(shape) > .1
    support[0, 0] = True
    attention = _inspection_weights(rng.random(shape, dtype=np.float32))
    expected = _measure(candidate, target, support, details="tiles", inspection=attention)
    actual = _measure(candidate, target, support, details="tiles", inspection=attention, backend="mps")
    assert actual == expected
    assert actual.score == expected.score


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


def test_inspection_weights_match_on_large_contiguous_and_strided_maps():
    heatmap = np.tile(np.array([0.0, 1.0], dtype=np.float32), (1024, 512))
    contiguous, mix = _inspection_weights(heatmap)
    strided, strided_mix = _inspection_weights(heatmap.T[::-1])
    assert mix == strided_mix == 0.5
    np.testing.assert_array_equal(contiguous, heatmap + 0.5)
    np.testing.assert_array_equal(strided[::-1].T, contiguous)


def test_inspection_weights_accept_unaligned_float32_heatmaps():
    heatmap = np.ndarray((2, 2), np.float32, buffer=bytearray(17), offset=1)
    heatmap[:] = [[0, 1], [0, 1]]
    assert not heatmap.flags.aligned
    weights, mix = _inspection_weights(heatmap)
    assert mix == 0.5
    np.testing.assert_array_equal(weights, heatmap + 0.5)


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
    monkeypatch.setattr("bokehforge.scoring._maps", lambda *_, **__: (
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
