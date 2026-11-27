"""Infer one delivery-to-reference similarity from RGB evidence."""

from dataclasses import dataclass
from functools import lru_cache
from itertools import batched, product
from math import ceil, cos, exp, log, radians, sin

import numpy as np
from PIL import Image
from scipy import fft, ndimage as ndi, optimize

from ._workers import workers

ALIGNMENT_VERSION = 1
MAX_ROTATION_DEGREES = 5.0
_COARSE_SIDE = 160
_FINE_SIDE = 960
_MAX_PLACEMENTS = 3
_MAX_SEEDS = 8
_MIN_SUPPORT = 0.32
_AMBIGUITY_MARGIN = 0.037
_LUMA = np.array([0.2126, 0.7152, 0.0722], dtype=np.float32)


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


def _box(shape):
    return (0, 0, shape[0], shape[1])


def _cut(image, box):
    y, x, y1, x1 = box
    return image[y:y1, x:x1]


def _xyxy(box):
    y, x, y1, x1 = box
    return [int(x), int(y), int(x1), int(y1)]


def _same_uniform(image):
    # Short-circuit on typical photos. The full comparison is necessary for a
    # constant field with one small, possibly off-center feature.
    sample = image[::max(1, image.shape[0] // 24), ::max(1, image.shape[1] // 24)]
    return np.all(sample == image[0, 0]) and np.all(image == image[0, 0])


def _powers(base, count):
    powers = np.full(count, base, dtype=np.uint64)
    if count:
        powers[0] = 1
    return np.multiply.accumulate(powers)


def _rolling_offsets(needle, image):
    """Screen repetitive anchors with a rolling rectangle hash before exact verification."""
    h, w = needle.shape[:2]
    ih, iw = image.shape[:2]
    modulus = 2**64
    bx, by = 1099511628211, 14029467366897019727
    inv_x, inv_y = pow(bx, -1, modulus), pow(by, -1, modulus)
    xp, yp = _powers(bx, iw), _powers(by, h)
    inverse_x = _powers(inv_x, iw - w + 1)

    def horizontal(line):
        words = np.asarray(line, dtype=np.float32).view(np.uint32)
        # Numeric equality treats both encodings of zero as equal.
        words = np.where(line == 0, 0, words).astype(np.uint64)
        words = (words[:, 0] * np.uint64(11400714819323198485) ^
                 words[:, 1] * np.uint64(3202034522624059733) ^ words[:, 2])
        prefix = np.empty(len(words) + 1, dtype=np.uint64)
        prefix[0] = 0
        np.cumsum(words * xp[:len(words)], out=prefix[1:])
        return (prefix[w:] - prefix[:-w]) * inverse_x[:len(prefix) - w]

    needle_rows = np.concatenate([horizontal(row) for row in needle])
    digest = np.sum(needle_rows * yp, dtype=np.uint64)
    width = iw - w + 1
    ring = np.empty((h, width), dtype=np.uint64)
    hashed = np.zeros(width, dtype=np.uint64)
    for i, row in enumerate(image[:h]):
        ring[i] = horizontal(row)
        hashed += ring[i] * yp[i]
    for y in range(ih - h + 1):
        for x in np.flatnonzero(hashed == digest):
            yield y, int(x)
        if y + h < ih:
            slot = y % h
            hashed -= ring[slot]
            hashed *= np.uint64(inv_y)
            ring[slot] = horizontal(image[y + h])
            hashed += ring[slot] * yp[-1]


def _exact_offsets(needle, image, trace):
    """Find exact native crops, using sampled pixels to screen every possible offset."""
    h, w = needle.shape[:2]
    dy, dx = image.shape[0] - h, image.shape[1] - w
    if min(dy, dx) < 0:
        trace["result"] = "does_not_fit"
        return []
    if dy == dx == 0:
        match = np.array_equal(needle, image)
        trace.update(result="found" if match else "different_samples", verified_offsets=1)
        return [(0, 0)] if match else []
    center = (dy // 2, dx // 2)
    if _same_uniform(needle):
        if _same_uniform(image):
            match = np.array_equal(needle[0, 0], image[0, 0])
            trace.update(result="uniform_match" if match else "different_samples", verified_offsets=1,
                         equivalent_offsets=(dy + 1) * (dx + 1) if match else 0)
            return [center] if match else []
        # A rectangle count tests constant crops even in mostly constant fields
        # with sparse outliers, without rechecking the same large patch repeatedly.
        match = np.all(image == needle[0, 0], axis=-1)
        integral = np.pad(match.cumsum(axis=0, dtype=np.int64).cumsum(axis=1), ((1, 0), (1, 0)))
        counts = integral[h:, w:] - integral[:-h, w:] - integral[h:, :-w] + integral[:-h, :-w]
        equal = counts == h * w
        trace.update(result="found" if equal.any() else "different_samples",
                     equivalent_offsets=int(equal.sum()), verified_offsets=(dy + 1) * (dx + 1))
        if equal[center]:
            return [center]
        if equal.any():
            return [tuple(int(v) for v in np.unravel_index(equal.argmax(), equal.shape))]
        return []
    ys, xs = np.linspace(0, h - 1, min(9, h), dtype=int), np.linspace(0, w - 1, min(9, w), dtype=int)
    points = list(product(ys, xs))
    samples = needle[np.ix_(ys, xs)].reshape(-1, 3).astype(np.float64)
    rarity = np.max(abs(samples - np.median(samples, axis=0)), axis=1)
    anchors = [points[i] for i in np.argsort(-rarity, kind="stable")[:16]]
    anchors.extend([(h // 2, w // 2), (0, 0), (h - 1, w - 1)])
    matches = []
    verified, examined = 0, 0
    verify_limit = max(16, 12_000_000 // (h * w))
    rows = max(1, 262144 // (dx + 1))
    for start in range(0, dy + 1, rows):
        end = min(dy + 1, start + rows)
        survivors = np.ones((end - start, dx + 1), dtype=bool)
        for y, x in anchors:
            for c in range(3):
                survivors &= image[start + y:end + y, x:x + dx + 1, c] == needle[y, x, c]
            if not survivors.any():
                break
        examined += survivors.size
        for sy, x in zip(*np.where(survivors), strict=True):
            y, x = int(start + sy), int(x)
            verified += 1
            if np.array_equal(needle, image[y:y + h, x:x + w]):
                matches.append((y, x))
                if len(matches) == _MAX_PLACEMENTS:
                    break
            if verified >= verify_limit:
                trace["fallback"] = "rolling_rgb_rectangle_hash"
                matches = []
                for y, x in _rolling_offsets(needle, image):
                    verified += 1
                    if np.array_equal(needle, image[y:y + h, x:x + w]):
                        matches.append((y, x))
                        if len(matches) == _MAX_PLACEMENTS:
                            break
                trace.update(result="found" if matches else "different_samples", anchors=len(anchors),
                             screened_offsets=examined, verified_offsets=verified, retained_limit=_MAX_PLACEMENTS)
                return matches
        if len(matches) == _MAX_PLACEMENTS:
            break
    trace.update(result="found" if matches else "different_samples", anchors=len(anchors),
                 screened_offsets=examined, verified_offsets=verified, retained_limit=_MAX_PLACEMENTS)
    return matches


def _proposed_content(image):
    """Detect exact flat-fill and repeated-edge runs, without classifying them yet."""
    bounds = list(_box(image.shape))
    strips = []
    names = ("top", "bottom", "left", "right")
    for index, (axis, reverse) in enumerate(product((0, 1), (False, True))):
        lines = np.swapaxes(image, 0, axis)
        if reverse:
            lines = lines[::-1]
        # Look past corners that may belong to a perpendicular strip.
        margin = lines.shape[1] // 6
        lines = lines[:, margin:lines.shape[1] - margin]
        length = 1
        for line in lines[1:len(lines) // 2]:
            if not np.array_equal(line, lines[0]):
                break
            length += 1
        fill = np.all(lines[0] == lines[0, 0])
        thickness = length if fill else length - 1
        if thickness < (2 if fill else 1) or length >= len(lines) // 2:
            continue
        bound = axis + 2 * reverse
        bounds[bound] += -thickness if reverse else thickness
        strips.append({"side": names[index], "kind": "flat_fill" if fill else "repeated_edge",
                       "pixels": thickness, "bound": bound})
    return tuple(bounds), strips


@dataclass(frozen=True)
class _Texture:
    values: np.ndarray
    step: np.ndarray

    def sample(self, points):
        coords = (points + 0.5) / self.step[:, None] - 0.5
        return np.stack([ndi.map_coordinates(plane, coords, order=1, mode="nearest", prefilter=False)
                         for plane in self.values], axis=0)


@dataclass(frozen=True)
class _NativeTexture:
    image: np.ndarray

    @property
    def step(self):
        return np.ones(2)

    def sample(self, points):
        offsets = np.array([[0, 0], [-3, 0], [3, 0], [0, -3], [0, 3]]).T
        neighbors = (points[:, None, :] + offsets[:, :, None]).reshape(2, -1)
        rgb = np.clip(_sample_rgb(self.image, neighbors), 0, 1).reshape(5, -1, 3)
        delta = rgb[0] - rgb[1:].mean(axis=0)
        light = delta @ _LUMA
        return np.stack((light, (delta[:, 0] - delta[:, 1]) * 0.55,
                         (delta[:, 2] - delta[:, 1]) * 0.55)).astype(np.float32)


def _resize_plane(image, size):
    clipped = np.clip(image, 0, 1)
    if clipped.shape == tuple(size):
        return clipped
    return np.asarray(Image.fromarray(clipped).resize(tuple(size[::-1]), Image.Resampling.BOX))


def _texture_pair(candidate, target):
    """Reuse each full-resolution clipped plane at every search scale."""
    shapes = [np.array(image.shape[:2]) for image in (candidate, target)]
    channels = [image[..., channel] for image in (candidate, target) for channel in range(3)]
    large = max(max(shape) for shape in shapes) >= 1024
    process = workers().map if large else map

    def prepare(channel):
        return Image.fromarray(np.clip(channel, 0, 1))

    planes = list(process(prepare, channels))

    def at_scale(side):
        sizes = [np.maximum(1, np.rint(shape / max(1, max(shape) / side))).astype(int) for shape in shapes]

        def resize(index):
            size = tuple(sizes[index // 3][::-1])
            plane = planes[index]
            return np.asarray(plane if plane.size == size else plane.resize(size, Image.Resampling.BOX))

        reduced = list(process(resize, range(6)))
        return [_texture(np.stack(reduced[3 * i:3 * i + 3]), shape, size)
                for i, (shape, size) in enumerate(zip(shapes, sizes, strict=True))]

    return at_scale


def _texture(rgb, shape, size):
    light = np.einsum("cij,c->ij", rgb, _LUMA)
    color = np.stack((light, (rgb[0] - rgb[1]) * 0.55, (rgb[2] - rgb[1]) * 0.55))
    # Locally centered luma and two color-opponent planes are used for
    # registration only. Color, exposure and pixel values remain in the grade.
    texture = ndi.gaussian_filter(color, (0, 0.65, 0.65), mode="reflect")
    texture -= ndi.gaussian_filter(color, (0, 4, 4), mode="reflect")
    return _Texture(texture, shape / size), float(np.std(color, axis=(1, 2)).max())


def _anchor_points(texture):
    shape = np.array(texture.values.shape[1:])
    choices = []
    for y, x in product(range(4), repeat=2):
        if (y + x) % 2:
            continue
        y0, x0 = np.floor(shape / 4 * [y, x]).astype(int)
        y1, x1 = np.ceil(shape / 4 * [y + 1, x + 1]).astype(int)
        patch = texture.values[:, y0:y1, x0:x1]
        center = shape * np.array([y + 0.5, x + 0.5]) / 4
        energy = np.std(patch, axis=(1, 2)).max()
        choices.append((float(energy), y, x, center))
    anchors = {}
    for _, y, x, center in sorted(choices, reverse=True):
        anchors.setdefault((y // 2, x // 2), center * texture.step - 0.5)
    return list(anchors.values())


class _Correlation:
    """Reuse a reference spectrum and summed-area energy across template sizes."""

    def __init__(self, ref):
        self.shape = ref.shape[1:]
        self.fft_shape = tuple(fft.next_fast_len(s + 42, real=True) for s in self.shape)
        self.spectrum = fft.rfftn(ref, self.fft_shape, axes=(1, 2))
        self.total = self._integral(ref)
        self.squares = self._integral(np.square(ref).sum(axis=0, keepdims=True))
        self.energy = {}

    @staticmethod
    def _integral(x):
        return np.pad(x.cumsum(axis=1, dtype=np.float64).cumsum(axis=2), ((0, 0), (1, 0), (1, 0)))

    @staticmethod
    def _sums(integral, h, w):
        return integral[:, h:, w:] - integral[:, :-h, w:] - integral[:, h:, :-w] + integral[:, :-h, :-w]

    def correlate(self, templates):
        # FFTs operate on each template and channel separately. Pack a bounded
        # batch with the same zero padding the individual FFT calls use.
        batch = np.zeros((len(templates), self.spectrum.shape[0], *self.fft_shape), dtype=np.float32)
        metadata = []
        for index, template in enumerate(templates):
            h, w = template.shape[1:]
            centered = template - template.mean(axis=(1, 2), keepdims=True)
            norm = np.square(centered).sum(dtype=np.float64)
            metadata.append((h, w, norm))
            if norm > h * w * 1e-13:
                batch[index, :, :h, :w] = centered
        kernel = fft.rfftn(batch, self.fft_shape, axes=(-2, -1), workers=4)
        numerator = fft.irfftn((self.spectrum * kernel.conj()).sum(axis=1), self.fft_shape,
                              axes=(-2, -1), workers=4)
        for values, (h, w, norm) in zip(numerator, metadata, strict=True):
            if norm <= h * w * 1e-13:
                yield None
                continue
            if (h, w) not in self.energy:
                totals = self._sums(self.total, h, w)
                energy = self._sums(self.squares, h, w)[0] - np.square(totals).sum(axis=0) / (h * w)
                self.energy[h, w] = np.maximum(energy, h * w * 1e-13)
            values = values[:self.shape[0] - h + 1, :self.shape[1] - w + 1]
            yield np.clip(values / np.sqrt(norm * self.energy[h, w]), -1, 1)


def _coarse_candidates(crop, reference, dimensions, scales, angles):
    ref_h, ref_w = reference.values.shape[1:]
    corr = _Correlation(reference.values)
    peaks, attempted = [], 0
    anchors = _anchor_points(crop)

    @lru_cache(maxsize=None)
    def template_offsets(scale, angle):
        pose = Pose(0, 0, angle, scale)
        # Keep the template inside one of the fit cells. Neither the
        # translation proposals nor the optimizer use the held-out cells.
        half = dimensions * scale * 0.115
        inverse_extent = np.abs(np.linalg.inv(pose.matrix)) @ half
        shrink = min(1, min(dimensions * 0.115 / inverse_extent))
        half = np.floor(half * shrink / reference.step).astype(int)
        hy, hx = np.minimum(np.minimum(half, (np.array([ref_h, ref_w]) - 1) // 2), 20)
        if min(hy, hx) < 2:
            return None
        yy, xx = np.mgrid[-hy:hy + 1, -hx:hx + 1]
        displacements = np.array([yy.ravel(), xx.ravel()]) * reference.step[:, None]
        return pose, hy, hx, np.linalg.solve(pose.matrix, displacements)

    def templates():
        for anchor, scale, angle in product(anchors, scales, angles):
            offsets = template_offsets(scale, angle)
            if offsets is not None:
                pose, hy, hx, coords = offsets
                template = crop.sample(coords + anchor[:, None]).reshape(3, 2 * hy + 1, 2 * hx + 1)
                yield anchor, pose, hy, hx, template

    for batch in batched(templates(), 12):
        attempted += len(batch)
        agreements = corr.correlate([template for *_, template in batch])
        for (anchor, pose, hy, hx, template), agreement in zip(batch, agreements, strict=True):
            if agreement is None:
                continue
            reliability = 0.30 * exp(-template[0].size / 100)
            for _ in range(2):
                y, x = np.unravel_index(agreement.argmax(), agreement.shape)
                value = float(agreement[y, x])
                if value < 0.18:
                    break
                center = (np.array([y + hy, x + hx]) + 0.5) * reference.step - 0.5
                peak = center - pose.matrix @ anchor
                # Tiny templates can produce high correlations by chance.
                peaks.append((value - reliability, Pose(*peak, pose.angle, pose.scale)))
                agreement[max(0, y - 3):y + 4, max(0, x - 3):x + 4] = -1
    return peaks, attempted, [point.tolist() for point in anchors]


def _pearson(values, samples):
    if values.shape[1] < 4:
        return 0.0
    a = values - values.mean(axis=1, keepdims=True)
    b = samples - samples.mean(axis=1, keepdims=True)
    norm = np.sqrt(np.square(a).sum(dtype=np.float64) * np.square(b).sum(dtype=np.float64))
    if norm <= values.shape[1] * 1e-13:
        return 0.0
    return float(np.clip((a * b).sum(dtype=np.float64) / norm, -1, 1))


@dataclass(frozen=True)
class _Evidence:
    agreement: float
    support: float
    regions: tuple

    @property
    def objective(self):
        # A minimum overlap and a continuous preference for comparing more of
        # the delivery discourage a fit on one coincidentally matching corner.
        if self.support < _MIN_SUPPORT:
            return -1.0 + self.support
        return self.agreement - 0.16 * (1 - self.support)

    def describe(self):
        return {"agreement": self.agreement, "support_fraction": self.support,
                "regions": [list(region) for region in self.regions]}


class _EvidenceSampler:
    def __init__(self, crop, reference, crop_shape, ref_shape, count, *, native=False):
        h, w = crop_shape
        self.reference = reference
        self.shape = np.asarray(ref_shape)
        margin_y, margin_x = np.minimum((np.array(crop_shape) - 1) / 5, crop.step * 3)
        yy, xx = np.meshgrid(np.linspace(margin_y, h - 1 - margin_y, min(h, count)),
                             np.linspace(margin_x, w - 1 - margin_x, min(w, count)), indexing="ij")
        if native:
            yy, xx = np.rint(yy), np.rint(xx)
        points = np.stack((yy.ravel(), xx.ravel()))
        values = crop.sample(points)
        ry = np.minimum(3, (yy.ravel() * 4 / h).astype(int))
        rx = np.minimum(3, (xx.ravel() * 4 / w).astype(int))
        regions = 4 * ry + rx
        fit = (ry + rx) % 2 == 0
        self.subsets = ((points[:, fit], values[:, fit], regions[fit]),
                        (points[:, ~fit], values[:, ~fit], regions[~fit]))

    def __call__(self, pose, subset="fit", *, regional=False):
        points, values, region_ids = self.subsets[subset != "fit"]
        points = pose.matrix @ points + pose.offset[:, None]
        valid = np.all((points >= 0) & (points <= self.shape[:, None] - 1), axis=0)
        support = float(valid.mean()) if valid.size else 0.0
        if valid.sum() < 4:
            return _Evidence(0.0, support, ())
        samples = self.reference.sample(points[:, valid])
        values = values[:, valid]
        agreement = _pearson(values, samples)
        regions = []
        if regional:
            cells = region_ids[valid]
            for index in np.unique(cells):
                cell = cells == index
                a, b = values[:, cell], samples[:, cell]
                energy = float(np.sqrt(np.std(a, axis=1).max() * np.std(b, axis=1).max()))
                if cell.sum() >= 4 and energy > 1e-6:
                    regions.append((int(index), int(cell.sum()), _pearson(a, b), energy))
        return _Evidence(agreement, support, tuple(regions))


def _displacement(a, b, shape):
    ends = np.array([[0, 0, shape[0] - 1, shape[0] - 1], [0, shape[1] - 1, 0, shape[1] - 1]])
    displacement = (a.matrix - b.matrix) @ ends + (a.offset - b.offset)[:, None]
    return float(np.max(np.abs(displacement)))


def _distinct(seeds, shape, distance, limit):
    kept = []
    for seed in seeds:
        if all(_displacement(seed, other, shape) > distance for other in kept):
            kept.append(seed)
            if len(kept) >= limit:
                break
    return kept


def _refine(seed, sampler, shape, step, scale_limits, *, wide=False):
    # Parameterize around the delivery center so slight scale and roll
    # changes do not also require a compensating translation.
    center = (np.array(shape) - 1) / 2
    mapped = seed.matrix @ center + seed.offset
    params = (0, 0, seed.angle, log(seed.scale))
    move = max(1.3, step * (4.8 if wide else 3.0))
    if wide:
        roll, size = 2.6, 0.13
    else:
        # Keep refinement local on larger images, where a slight roll or
        # scale change sweeps many pixels across the outer field.
        relative = max(0.001, move / (max(shape) * seed.scale))
        roll, size = min(0.9, relative * 110), min(0.035, relative * 2)
    bounds = [(-move, move), (-move, move),
              (max(-MAX_ROTATION_DEGREES, seed.angle - roll), min(MAX_ROTATION_DEGREES, seed.angle + roll)),
              (max(log(scale_limits[0]), params[3] - size), min(log(scale_limits[1]), params[3] + size))]

    def pose(p):
        prototype = Pose(0, 0, float(p[2]), exp(p[3]))
        offset = mapped + p[:2] - prototype.matrix @ center
        return Pose(*offset, prototype.angle, prototype.scale)

    def loss(p):
        return -sampler(pose(p)).objective

    result = optimize.minimize(loss, params, bounds=bounds, method="Powell",
                               options={"maxfev": 155, "xtol": 0.006, "ftol": 0.00015})
    continuous = pose(result.x)
    center_offset = mapped - center
    native = Pose(float(round(continuous.y)), float(round(continuous.x)))
    centered = Pose(*np.rint(center_offset))
    return max((seed, continuous, native, centered), key=lambda p: sampler(p).objective)


def _regional_strength(evidence):
    # A broad fit should have support in more than one textured region. Use a
    # soft term so real local damage doesn't veto a good global placement.
    if not evidence.regions:
        return max(0.0, evidence.agreement)
    top_energy = max(region[3] for region in evidence.regions)
    local = np.median([region[2] for region in evidence.regions if region[3] >= 0.09 * top_energy])
    return float(0.76 * evidence.agreement + 0.24 * local)


def _strength(fit, holdout):
    return min(_regional_strength(fit), _regional_strength(holdout))


def _compare(pose, sampler, broad_sampler, shape):
    fit = sampler(pose, regional=True)
    holdout = sampler(pose, "holdout", regional=True)
    broad_fit = broad_sampler(pose, regional=True)
    broad_holdout = broad_sampler(pose, "holdout", regional=True)
    fine = _strength(fit, holdout)
    broad = _strength(broad_fit, broad_holdout)
    strength = max(fine, broad)
    support = min(fit.support, holdout.support, broad_fit.support, broad_holdout.support)
    native = Pose(float(round(pose.y)), float(round(pose.x)))
    complexity = min(1, _displacement(pose, native, shape) / 1.8)
    # Broad correspondence still identifies the scene when a real defect
    # destroys fine detail. The fine band supplies the more precise pose.
    joint = 0.60 * strength + 0.40 * min(fine, broad)
    rank = joint - 0.11 * (1 - support) - 0.035 * complexity if support >= _MIN_SUPPORT else -1.0
    return rank, strength, fit, holdout, broad_fit, broad_holdout


def _confidence(strength):
    point = float(np.clip((strength - 0.23) / (0.71 - 0.23), 0, 1))
    return point * point * (3 - 2 * point)


def _scales(crop_shape, ref_shape):
    ratio = np.asarray(ref_shape) / crop_shape
    lower, upper = min(0.35, max(0.2, min(ratio) * 0.75)), min(8, max(2.5, min(ratio) * 1.08))
    hypotheses = list(np.geomspace(lower, upper, 1 + ceil(log(upper / lower) / 0.095)))
    hypotheses.extend([1.0, *np.clip(ratio, lower, upper)])
    return sorted(set(map(float, hypotheses))), (float(lower), float(upper))


def _approximate(crop, target, trace, native, origin):
    shape, ref_shape = np.asarray(crop.shape[:2]), np.asarray(target.shape[:2])
    centered = Pose(*np.floor((ref_shape - shape) / 2))
    pair = _texture_pair(crop, target)
    (coarse_crop, crop_contrast), (coarse_ref, ref_contrast) = pair(_COARSE_SIDE)
    trace["coarse_rgb_contrast"] = {"delivery": crop_contrast, "reference": ref_contrast}
    if (max(crop_contrast, ref_contrast) < 0.006 or min(*shape, *ref_shape) < 3
            or (ref_contrast < 1e-6 and _same_uniform(target))):
        trace["stages"].append({"method": "rgb_similarity_search", "result": "insufficient_spatial_structure"})
        return [(native, 1.0, {"source": "low_information_native_fallback"})], 1.0, "low_information_native_fallback"

    scales, bounds = _scales(shape, ref_shape)
    angles = np.linspace(-MAX_ROTATION_DEGREES, MAX_ROTATION_DEGREES, 5)
    peaks, templates, anchors = _coarse_candidates(coarse_crop, coarse_ref, shape, scales, angles)
    coarse_sampler = _EvidenceSampler(coarse_crop, coarse_ref, shape, ref_shape, 28)
    # Dimension-derived starts complement the global search. They compete on
    # the same image evidence as the translations proposed by the templates.
    starts = [(1.0, native), (1.0, centered), (1.0, Pose(0, 0))]
    for scale in (min(np.asarray(ref_shape) / shape), max(np.asarray(ref_shape) / shape)):
        if bounds[0] <= scale <= bounds[1]:
            starts.append((1.0, Pose(*((ref_shape - 1 - scale * (shape - 1)) / 2), scale=scale)))
    starts.extend(sorted(peaks, key=lambda p: -p[0])[:96])
    step = float(max(coarse_ref.step))
    distinct = _distinct([p for _, p in starts], shape, max(2.0, step * 1.2), 76)
    ranked = sorted(distinct, key=lambda p: -coarse_sampler(p).objective)
    seeds = _distinct(ranked, shape, max(3.0, 2.5 * step), _MAX_SEEDS)
    if all(_displacement(centered, p, shape) > step for p in seeds):
        seeds = [*seeds[:-1], centered]
    trace["stages"].append({"method": "rgb_patch_correlation", "reference_size_wh": list(coarse_ref.values.shape[:0:-1]),
                            "scale_hypotheses": scales, "angles_degrees": angles.tolist(),
                            "anchor_yx": [(np.asarray(p) + origin).tolist() for p in anchors],
                            "templates": templates, "peaks": len(peaks),
                            "ranked_starts": len(distinct), "refined_starts": len(seeds),
                            "seed_transforms": [p.move_origin(-origin).describe() for p in seeds]})

    coarse_fits = [_refine(p, coarse_sampler, shape, step, bounds, wide=True) for p in seeds]
    ranked = sorted(coarse_fits, key=lambda p: -coarse_sampler(p).objective)
    refined = _distinct(ranked, shape, max(1.8, step), 5)
    (fine_crop, _), (fine_ref, _) = pair(_FINE_SIDE)
    fine_sampler = _EvidenceSampler(fine_crop, fine_ref, shape, ref_shape, 44)
    (broad_crop, _), (broad_ref, _) = pair(96)
    del pair
    broad_sampler = _EvidenceSampler(broad_crop, broad_ref, shape, ref_shape, 28)
    step = float(max(fine_ref.step))
    fine = [_refine(p, fine_sampler, shape, max(step, max(coarse_ref.step) * 0.55), bounds) for p in refined]
    trace["stages"].append({"method": "coarse_to_fine_fit", "fine_reference_size_wh": list(fine_ref.values.shape[:0:-1]),
                            "broad_reference_size_wh": list(broad_ref.values.shape[:0:-1]), "fine_starts": len(refined), "max_objective_evaluations_per_start": 155,
                            "fit_regions": "even checkerboard cells of a 4 by 4 delivery grid",
                            "held_out_regions": "odd checkerboard cells of the same grid",
                            "regional_columns": ["cell_index", "supporting_samples", "agreement", "shared_texture_energy"],
                            "feature_planes": ["local_luma", "local_red_minus_green", "local_blue_minus_green"]})

    tried = [native, centered]
    for pose in fine:
        tried.extend([Pose(float(round(pose.y)), float(round(pose.x))), pose])
    if max(fine_ref.step.max(), fine_crop.step.max()) > 1.05:
        native_sampler = _EvidenceSampler(_NativeTexture(crop), _NativeTexture(target), shape, ref_shape,
                                          42, native=True)
        strongest = sorted(fine, key=lambda p: -fine_sampler(p).objective)[:3]
        for seed in strongest:
            # The fine pyramid can place a sharp peak about one decimation
            # step away from its native position. Try nearby integer samples
            # before the last fractional fit.
            reach = min(5, max(2, ceil(step / 2)))
            y, x = round(seed.y), round(seed.x)
            integers = (Pose(y + dy, x + dx) for dy, dx in product(range(-reach, reach + 1), repeat=2))
            best_integer = max(integers, key=lambda p: native_sampler(p).objective)
            tried.extend([best_integer, _refine(seed, native_sampler, shape, step / 2, bounds)])
        fine_sampler = native_sampler
        trace["stages"].append({"method": "native_sample_refinement", "refined_starts": len(strongest),
                                "integer_search_radius": reach, "samples_per_axis_limit": 42,
                                "max_objective_evaluations_per_start": 155})
    comparisons = [(p, *_compare(p, fine_sampler, broad_sampler, shape)) for p in dict.fromkeys(tried)]
    comparisons.sort(key=lambda row: -row[1])
    best_rank = comparisons[0][1]
    confidence = _confidence(comparisons[0][2]) if best_rank > -1 else 0.0
    hypotheses = []
    # Small parameter variations around the same field are one placement.
    # Preserve truly separate locations in repeating or ambiguous scenes.
    cluster_distance = max(2.25, max(ref_shape) * 0.004)
    for pose, rank, strength, fit, holdout, broad_fit, broad_holdout in comparisons:
        decision = "selected"
        if not confidence:
            decision = "insufficient_distributed_agreement"
        elif best_rank - rank >= _AMBIGUITY_MARGIN:
            decision = "weaker_alignment"
        elif any(_displacement(previous, pose, shape) <= cluster_distance
                 for previous, _, _ in hypotheses):
            decision = "same_placement"
        elif len(hypotheses) == _MAX_PLACEMENTS:
            decision = "placement_limit"
        row = {"transform": pose.move_origin(-origin).describe(), "fit": fit.describe(), "held_out": holdout.describe(),
               "broad_fit": broad_fit.describe(), "broad_held_out": broad_holdout.describe(),
               "strength": strength, "rank": rank, "decision": decision}
        trace["candidates"].append(row)
        if decision == "selected":
            weight = (1 - (best_rank - rank) / _AMBIGUITY_MARGIN) ** 2
            hypotheses.append((pose, weight, {"source": "rgb_similarity_search",
                                              "candidate_index": len(trace["candidates"]) - 1}))
    if hypotheses:
        return hypotheses, confidence, "distributed_rgb_agreement"

    # Low-energy detail can be lost to blur or export noise. Infer a native
    # placement only when the available large-scale appearance is similar.
    near = np.all(abs(ref_shape - shape) <= np.maximum(3, ref_shape * 0.05))
    if near and max(crop_contrast, ref_contrast) < 0.045:
        level = min(12, min(*shape, *ref_shape))
        means = [np.stack([_resize_plane(image[..., i], (level, level)) for i in range(3)])
                 for image in (crop, target)]
        difference = float(np.mean(abs(means[0] - means[1])))
        fallback = float(np.clip(1 - difference / 0.05, 0, 1))
        trace["low_information_appearance_delta"] = difference
        if fallback > 0:
            return [(centered, 1.0, {"source": "near_native_low_information_fallback"})], fallback, "near_native_low_information_fallback"
    return [], 0.0, "weak_image_agreement"


def _sample_rgb(image, points):
    return np.stack([ndi.map_coordinates(image[..., i], points, order=1, mode="nearest", prefilter=False)
                     for i in range(3)], axis=1).astype(np.float64)


def _verify_strips(image, target, box, strips, pose, exact):
    if not strips:
        return _box(image.shape), []
    bounds = list(_box(image.shape))
    report = []
    h, w = target.shape[:2]
    for strip in strips:
        bound = strip["bound"]
        axis, end = bound % 2, bound >= 2
        orth = 1 - axis
        inner = box[bound] - int(end)
        outer = (image.shape[axis] - 1) if end else 0
        across = np.linspace(box[orth], box[orth + 2] - 1, 28)
        width = strip["pixels"]
        first_outer = inner + (1 if end else -1)
        normal = np.linspace(first_outer, outer, min(width, 10))
        yy, xx = np.meshgrid(normal, across, indexing="ij")
        points = np.empty((2, yy.size))
        points[axis], points[orth] = yy.ravel(), xx.ravel()
        ref_points = pose.matrix @ points + pose.offset[:, None]
        inside = np.all((ref_points >= -0.5) & (ref_points <= np.array([[h], [w]]) - 0.5), axis=0)
        outside = float(1 - inside.mean())
        edge = points.copy()
        edge[axis] = inner
        edge_ref = pose.matrix @ edge + pose.offset[:, None]
        present = np.all((edge_ref >= 0) & (edge_ref <= np.array([[h], [w]]) - 1), axis=0)
        seam_error = 1.0
        if present.any():
            a = _sample_rgb(image, edge[:, present])
            b = _sample_rgb(target, edge_ref[:, present])
            seam_error = float(np.mean(abs(np.clip(a, 0, 1) - np.clip(b, 0, 1))))
        strip_error = 0.0
        if inside.any():
            a = _sample_rgb(image, points[:, inside])
            b = _sample_rgb(target, ref_points[:, inside])
            strip_error = float(np.mean(abs(np.clip(a, 0, 1) - np.clip(b, 0, 1))))
        # A repeated edge with matching interior evidence and genuine canvas
        # extension can be removed through the seam. Borders on the original
        # field are ambiguous with occlusion and must still be graded.
        verified = (exact or seam_error < 0.045) and outside >= 0.60 and (
            outside >= 0.98 or strip_error > max(0.02, seam_error * 1.8))
        if verified:
            bounds[bound] = box[bound]
        report.append({key: value for key, value in strip.items() if key != "bound"} |
                      {"outside_reference_fraction": outside, "seam_rgb_delta": seam_error,
                       "strip_rgb_delta": strip_error, "decision": "excluded" if verified else "retained"})
    return tuple(bounds), report


@dataclass(frozen=True)
class Placement:
    pose: Pose
    box: tuple
    weight: float
    evidence: dict
    padding: tuple


def align(candidate, target):
    full = _box(candidate.shape)
    trace = {"schema_version": ALIGNMENT_VERSION,
             "reference_size_wh": list(target.shape[1::-1]), "candidate_size_wh": list(candidate.shape[1::-1]),
             "stages": [], "candidates": []}

    attempt = {"method": "exact_native_delivery_in_reference"}
    trace["stages"].append(attempt)
    offsets = _exact_offsets(candidate, target, attempt)
    if offsets:
        placements = [Placement(Pose(y, x), full, 1 / len(offsets), {"source": attempt["method"]}, ())
                      for y, x in offsets]
        return placements, {**trace, "status": "matched", "reason": attempt["result"], "confidence": 1.0}

    if np.all(np.asarray(candidate.shape[:2]) >= target.shape[:2]) and candidate.shape != target.shape:
        attempt = {"method": "exact_reference_in_native_delivery"}
        trace["stages"].append(attempt)
        offsets = [(-y, -x) for y, x in _exact_offsets(target, candidate, attempt)]
        if offsets:
            content, strips = _proposed_content(candidate)
            placements = []
            for y, x in offsets:
                pose = Pose(y, x)
                box, padding = _verify_strips(candidate, target, content, strips, pose, True)
                placements.append(Placement(pose, box, 1 / len(offsets), {"source": attempt["method"]}, tuple(padding)))
            return placements, {**trace, "status": "matched", "reason": attempt["result"], "confidence": 1.0}

    content, strips = _proposed_content(candidate)
    trace["stages"].append({"method": "exact_border_runs", "proposed_content_box_xyxy": _xyxy(content),
                            "runs": [{k: v for k, v in strip.items() if k != "bound"} for strip in strips]})
    crop = _cut(candidate, content)
    hypotheses = []
    if strips:
        attempt = {"method": "exact_interior_in_reference"}
        trace["stages"].append(attempt)
        hypotheses = [(Pose(y, x), 1.0, {"source": attempt["method"]})
                      for y, x in _exact_offsets(crop, target, attempt)]
    if hypotheses:
        confidence, reason = 1.0, "exact_interior"
    else:
        near_center = np.floor((np.array(target.shape[:2]) - candidate.shape[:2]) / 2)
        origin = np.asarray(content[:2])
        hypotheses, confidence, reason = _approximate(crop, target, trace, Pose(*(near_center + origin)), origin)

    placements = []
    total = sum(weight for _, weight, _ in hypotheses)
    for pose, weight, evidence in hypotheses:
        pose = pose.move_origin(-np.array(content[:2]))
        bounds, padding = _verify_strips(candidate, target, content, strips, pose, reason == "exact_interior")
        placements.append(Placement(pose, bounds, confidence * weight / total, evidence, tuple(padding)))
    status = "unmatched" if not placements else ("uncertain" if "fallback" in reason else "matched")
    return placements, {**trace, "status": status, "reason": reason, "confidence": confidence}


def _clip_polygon(vertices, low, high):
    for axis in (0, 1):
        for bound, sign in ((low[axis], 1), (high[axis], -1)):
            clipped = []
            if not vertices:
                return []
            previous = vertices[-1]
            for vertex in vertices:
                p, v = sign * (previous[axis] - bound), sign * (vertex[axis] - bound)
                if (p < 0) != (v < 0):
                    clipped.append(previous + (vertex - previous) * (p / (p - v)))
                if v >= 0:
                    clipped.append(vertex)
                previous = vertex
            vertices = clipped
    return vertices


def overlap(placement, shape):
    y, x, y1, x1 = placement.box
    corners = np.array([[y, x], [y, x1], [y1, x1], [y1, x]], dtype=float) - 0.5
    pose = placement.pose
    vertices = list(corners @ pose.matrix.T + pose.offset)
    clipped = _clip_polygon(vertices, (-0.5, -0.5), np.asarray(shape) - 0.5)
    area = 0.0
    if len(clipped) >= 3:
        polygon = np.asarray(clipped) - clipped[0]
        area = abs(float(np.dot(polygon[:, 0], np.roll(polygon[:, 1], 1))
                         - np.dot(polygon[:, 1], np.roll(polygon[:, 0], 1)))) / 2
    geometry = min(1.0, area / (shape[0] * shape[1]))
    resolution = min(1.0, geometry / pose.scale ** 2)
    return geometry, resolution, [[float(x), float(y)] for y, x in clipped]


def native_overlap(placement, shape):
    """Reference bounds and delivery offsets for an integer-grid placement."""
    y, x, y1, x1 = placement.box
    dy, dx = int(placement.pose.y), int(placement.pose.x)
    return (max(0, y + dy), max(0, x + dx), min(shape[0], y1 + dy), min(shape[1], x1 + dx))


def project(candidate, shape, placement):
    """Project one delivery on the native grid, with a lossless integer path."""
    y, x, y1, x1 = placement.box
    pose = placement.pose.move_origin(np.array([y, x]))
    content = candidate[y:y1, x:x1]
    if pose.scale == 1 and pose.angle == 0 and all(float(v).is_integer() for v in (pose.y, pose.x)):
        dy, dx = int(pose.y), int(pose.x)
        top, left = max(0, dy), max(0, dx)
        bottom, right = min(shape[0], dy + content.shape[0]), min(shape[1], dx + content.shape[1])
        pixels = content[top - dy:bottom - dy, left - dx:right - dx]
        if (top, left, bottom, right) == _box(shape):
            return pixels, None
        support = np.zeros(shape[:2], dtype=bool)
        support[top:bottom, left:right] = True
        aligned = np.zeros(shape, dtype=np.float32)
        aligned[top:bottom, left:right] = pixels
        return aligned, support

    inverse = np.linalg.inv(pose.matrix)
    offset = -inverse @ pose.offset
    support = ndi.affine_transform(np.ones(content.shape[:2], dtype=bool), inverse, offset,
                                   output_shape=shape[:2], order=0, mode="grid-constant",
                                   cval=False, prefilter=False)
    aligned = np.empty(shape, dtype=np.float32)
    for channel in range(3):
        ndi.affine_transform(content[..., channel], inverse, offset, output=aligned[..., channel],
                             order=1, mode="nearest", prefilter=False)
    return aligned, None if support.all() else support
